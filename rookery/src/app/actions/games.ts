"use server";

import { z } from "zod";
import { requireProfile } from "@/lib/auth";
import { cannotMate, tryMove, viewAt, type PromotionRole } from "@/lib/chess/game";
import { VARIANT_IDS, startFen, type Variant } from "@/lib/chess/variants";
import { getLiveTimeControl } from "@/lib/live/timecontrols";
import { recordOf, type GameRow, type SeekRow } from "@/lib/live/types";
import { createClient } from "@/lib/supabase/server";
import { usernameSchema } from "@/lib/validation";

export type Failure = { ok: false; error: string };
export type GameResult = { ok: true; game: GameRow } | Failure;

const MESSAGES: Record<string, string> = {
  busy: "You (or your opponent) already have a game in progress.",
  gone: "That game offer is no longer available.",
  own_seek: "You can't accept your own offer.",
  forbidden: "That isn't yours to do.",
  stale: "The position changed. Try again.",
  not_your_turn: "It isn't your turn.",
  not_a_player: "You aren't a player in this game.",
  finished: "That game is already over.",
  too_late_to_abort: "Both players have moved, so you can't abort. Resign instead.",
  too_early: "Wait until each side has moved before offering a draw.",
  no_offer: "There is no draw offer to answer.",
  banned: "Your account is suspended.",
  not_signed_in: "Please log in first.",
  not_found: "Game not found.",
  bad_move: "That move isn't valid.",
};

function explain(message: string | undefined): string {
  const m = message ?? "";
  for (const key of Object.keys(MESSAGES)) if (m.includes(key)) return MESSAGES[key];
  if (m.includes("duplicate key")) return "You already have an open offer.";
  return "Something went wrong. Please try again.";
}

function secret(): string | null {
  return process.env.GAME_SERVER_SECRET ?? null;
}
const NOT_CONFIGURED: Failure = { ok: false, error: "Live games are not switched on yet (server secret missing)." };

const variantSchema = z.enum(VARIANT_IDS as [Variant, ...Variant[]]);
const idSchema = z.string().uuid();

async function loadGame(gameId: string): Promise<GameRow | null> {
  const supabase = await createClient();
  const { data } = await supabase.from("games").select("*").eq("id", gameId).maybeSingle();
  return (data as GameRow | null) ?? null;
}

// --------------------------------------------------------------------------------------------- matchmaking

export type PairResult = { ok: true; gameId: string } | { ok: true; waiting: true; seekId: string } | Failure;

export async function quickPair(variantIn: string, timeId: string): Promise<PairResult> {
  await requireProfile();
  const s = secret();
  if (!s) return NOT_CONFIGURED;
  const variant = variantSchema.safeParse(variantIn);
  const tc = getLiveTimeControl(timeId);
  if (!variant.success || !tc) return { ok: false, error: "Pick a game type and a time control." };

  const supabase = await createClient();
  const { data, error } = await supabase.rpc("server_quick_pair", {
    p_secret: s, p_variant: variant.data, p_time_ms: tc.ms, p_inc_ms: tc.inc, p_initial_fen: startFen(variant.data),
  });
  if (error) return { ok: false, error: explain(error.message) };
  const out = data as { game?: GameRow; seek?: SeekRow };
  if (out.game) return { ok: true, gameId: out.game.id };
  if (out.seek) return { ok: true, waiting: true, seekId: out.seek.id };
  return { ok: false, error: "Could not find or create a game." };
}

export async function acceptSeek(seekId: string): Promise<{ ok: true; gameId: string } | Failure> {
  await requireProfile();
  const s = secret();
  if (!s) return NOT_CONFIGURED;
  if (!idSchema.safeParse(seekId).success) return { ok: false, error: MESSAGES.gone };
  const supabase = await createClient();
  const { data: seek } = await supabase.from("seeks").select("variant").eq("id", seekId).maybeSingle();
  if (!seek) return { ok: false, error: MESSAGES.gone };
  const { data, error } = await supabase.rpc("server_accept_seek", {
    p_secret: s, p_seek_id: seekId, p_initial_fen: startFen((seek as { variant: Variant }).variant),
  });
  if (error) return { ok: false, error: explain(error.message) };
  return { ok: true, gameId: (data as GameRow).id };
}

export async function cancelSeek(seekId: string): Promise<{ ok: true } | Failure> {
  await requireProfile();
  if (!idSchema.safeParse(seekId).success) return { ok: false, error: MESSAGES.gone };
  const supabase = await createClient();
  const { error } = await supabase.from("seeks").delete().eq("id", seekId);
  return error ? { ok: false, error: explain(error.message) } : { ok: true };
}

const challengeSchema = z.object({
  username: usernameSchema,
  variant: variantSchema,
  timeId: z.string(),
  color: z.enum(["white", "black", "random"]),
});

export async function createChallenge(input: z.input<typeof challengeSchema>): Promise<{ ok: true } | Failure> {
  const me = await requireProfile();
  const parsed = challengeSchema.safeParse(input);
  const tc = parsed.success ? getLiveTimeControl(parsed.data.timeId) : undefined;
  if (!parsed.success || !tc) return { ok: false, error: "Check the username and options." };
  const supabase = await createClient();
  const { data: target } = await supabase.from("profiles").select("id, username").eq("username", parsed.data.username).maybeSingle();
  if (!target) return { ok: false, error: "No player has that username." };
  if (target.id === me.id) return { ok: false, error: "You can't challenge yourself." };
  const { error } = await supabase.from("seeks").insert({
    user_id: me.id, target_id: target.id, variant: parsed.data.variant, time_initial_ms: tc.ms, time_inc_ms: tc.inc, color: parsed.data.color,
  });
  return error ? { ok: false, error: explain(error.message) } : { ok: true };
}

// --------------------------------------------------------------------------------------------- in a game

export async function makeMove(gameId: string, from: string, to: string, promotion?: PromotionRole): Promise<GameResult> {
  await requireProfile();
  const s = secret();
  if (!s) return NOT_CONFIGURED;
  if (!idSchema.safeParse(gameId).success || !/^[a-h][1-8]$/.test(from) || !/^[a-h][1-8]$/.test(to)) return { ok: false, error: MESSAGES.bad_move };

  const row = await loadGame(gameId);
  if (!row) return { ok: false, error: MESSAGES.not_found };
  const record = recordOf(row);
  const attempt = tryMove(record, from as never, to as never, promotion);
  if (!attempt.ok) return { ok: false, error: attempt.reason === "promotion" ? "Choose a promotion piece." : MESSAGES.bad_move };

  const after = viewAt(attempt.record);
  const supabase = await createClient();
  const { data, error } = await supabase.rpc("server_move", {
    p_secret: s,
    p_game_id: gameId,
    p_uci: attempt.record.moves[attempt.record.moves.length - 1],
    p_expected_ply: record.moves.length,
    p_result: after.outcome?.result ?? null,
    p_reason: after.outcome?.reason ?? null,
  });
  if (error) return { ok: false, error: explain(error.message) };
  return { ok: true, game: data as GameRow };
}

async function simple(fn: "server_resign" | "server_abort", gameId: string): Promise<GameResult> {
  await requireProfile();
  const s = secret();
  if (!s) return NOT_CONFIGURED;
  if (!idSchema.safeParse(gameId).success) return { ok: false, error: MESSAGES.not_found };
  const supabase = await createClient();
  const { data, error } = await supabase.rpc(fn, { p_secret: s, p_game_id: gameId });
  return error ? { ok: false, error: explain(error.message) } : { ok: true, game: data as GameRow };
}
export async function resign(gameId: string): Promise<GameResult> {
  return simple("server_resign", gameId);
}
export async function abortGame(gameId: string): Promise<GameResult> {
  return simple("server_abort", gameId);
}

export async function drawAction(gameId: string, action: "offer" | "accept" | "decline" | "cancel"): Promise<GameResult> {
  await requireProfile();
  const s = secret();
  if (!s) return NOT_CONFIGURED;
  if (!idSchema.safeParse(gameId).success || !["offer", "accept", "decline", "cancel"].includes(action)) return { ok: false, error: MESSAGES.not_found };
  const supabase = await createClient();
  const { data, error } = await supabase.rpc("server_draw", { p_secret: s, p_game_id: gameId, p_action: action });
  return error ? { ok: false, error: explain(error.message) } : { ok: true, game: data as GameRow };
}

/** Ask the server to check the clock. Safe for anyone signed in to call; it only acts if time really ran out. */
export async function claimTimeout(gameId: string): Promise<GameResult> {
  await requireProfile();
  const s = secret();
  if (!s) return NOT_CONFIGURED;
  if (!idSchema.safeParse(gameId).success) return { ok: false, error: MESSAGES.not_found };
  const row = await loadGame(gameId);
  if (!row) return { ok: false, error: MESSAGES.not_found };
  const onMove = row.moves.length % 2 === 0 ? "white" : "black";
  const other = onMove === "white" ? "black" : "white";
  const draw = row.status === "started" && cannotMate(recordOf(row), other);
  const supabase = await createClient();
  const { data, error } = await supabase.rpc("server_claim_timeout", { p_secret: s, p_game_id: gameId, p_other_cannot_mate: draw });
  return error ? { ok: false, error: explain(error.message) } : { ok: true, game: data as GameRow };
}

/** Offer a rematch: a direct challenge to the opponent with colours swapped. */
export async function rematch(gameId: string): Promise<{ ok: true } | Failure> {
  const me = await requireProfile();
  if (!idSchema.safeParse(gameId).success) return { ok: false, error: MESSAGES.not_found };
  const row = await loadGame(gameId);
  if (!row || row.status === "started") return { ok: false, error: "The game isn't over yet." };
  const mine = row.white_id === me.id ? "white" : row.black_id === me.id ? "black" : null;
  if (!mine) return { ok: false, error: MESSAGES.not_a_player };
  const supabase = await createClient();
  const { error } = await supabase.from("seeks").insert({
    user_id: me.id,
    target_id: mine === "white" ? row.black_id : row.white_id,
    variant: row.variant,
    time_initial_ms: row.time_initial_ms,
    time_inc_ms: row.time_inc_ms,
    color: mine === "white" ? "black" : "white",
  });
  return error ? { ok: false, error: error.message.includes("duplicate") ? "Rematch already offered." : explain(error.message) } : { ok: true };
}
