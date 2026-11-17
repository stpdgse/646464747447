"use server";

import { z } from "zod";
import { requireProfile } from "@/lib/auth";
import { newRecord, tryUci, viewAt } from "@/lib/chess/game";
import { LEVELS, MIN_WIN_PLIES, PLY_LIMIT } from "@/lib/rude";
import { createClient } from "@/lib/supabase/server";

export type RudeReport =
  | { ok: true; level: number; best: number; status: "playing" | "finished"; cleared: boolean; outcome: "win" | "loss" | "draw" }
  | { ok: false; error: string };

const MESSAGES: Record<string, string> = {
  not_found: "That attempt was not found.",
  finished: "That attempt is already over.",
  wrong_level: "That wasn't the Rüde you were facing.",
  banned: "Your account is suspended.",
  forbidden: "The challenge isn't switched on yet (server secret missing).",
};

const schema = z.object({
  attemptId: z.string().uuid(),
  level: z.number().int().refine((n) => LEVELS.includes(n)),
  color: z.enum(["white", "black"]),
  moves: z.array(z.string().regex(/^[a-h][1-8][a-h][1-8][qrbn]?$/)).max(1000),
  how: z.enum(["finished", "resign", "limit"]),
});

/**
 * Report the end of a game against a Rüde. The server replays the whole game: it must be legal, a claimed win must be a real
 * checkmate (or other win) for the player's colour that lasted long enough, and only then does the database move the player up
 * the ladder. Anything else (a loss, a draw, a resignation) ends the attempt.
 */
export async function reportRudeGame(input: z.input<typeof schema>): Promise<RudeReport> {
  await requireProfile();
  const secret = process.env.GAME_SERVER_SECRET;
  if (!secret) return { ok: false, error: MESSAGES.forbidden };
  const parsed = schema.safeParse(input);
  if (!parsed.success) return { ok: false, error: "That game could not be read." };
  const { attemptId, level, color, moves, how } = parsed.data;

  let outcome: "win" | "loss" | "draw" = "loss";
  if (how === "limit") {
    // a game that went on and on without a result counts as a draw
    if (moves.length < PLY_LIMIT) return { ok: false, error: "That game isn't over yet." };
    outcome = "draw";
  }
  if (how === "finished") {
    let record = newRecord("standard");
    for (const m of moves) {
      const r = tryUci(record, m);
      if (!r.ok) return { ok: false, error: "That game contains an illegal move." };
      record = r.record;
    }
    const end = viewAt(record).outcome;
    if (!end) return { ok: false, error: "That game isn't finished." };
    const winner = end.result === "1-0" ? "white" : end.result === "0-1" ? "black" : null;
    if (winner === color) {
      if (moves.length < MIN_WIN_PLIES) return { ok: false, error: "That game was too short to count." };
      outcome = "win";
    } else outcome = winner ? "loss" : "draw";
  }

  const supabase = await createClient();
  const { data, error } = await supabase.rpc("server_rude_report", { p_secret: secret, p_attempt: attemptId, p_level: level, p_outcome: outcome });
  if (error) {
    const key = Object.keys(MESSAGES).find((k) => error.message.includes(k));
    return { ok: false, error: key ? MESSAGES[key] : "Something went wrong. Please try again." };
  }
  const a = data as { level: number; best_level: number; status: "playing" | "finished"; cleared: boolean };
  return { ok: true, level: a.level, best: a.best_level, status: a.status, cleared: a.cleared, outcome };
}
