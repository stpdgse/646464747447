"use server";

import { z } from "zod";
import { getCurrentProfile, requireProfile } from "@/lib/auth";
import { dailyPuzzleFor, lookup, pickPuzzle, rushBatch } from "@/lib/puzzles/data";
import { verifySolution, type ClientPuzzle } from "@/lib/puzzles/solve";
import { createClient } from "@/lib/supabase/server";

type Failure = { ok: false; error: string };

const secret = () => process.env.GAME_SERVER_SECRET ?? null;
const NOT_CONFIGURED: Failure = { ok: false, error: "Puzzles are not switched on yet (server secret missing)." };

/** The viewer's puzzle rating (1500 for newcomers) and every puzzle they have already attempted. */
async function history(userId: string) {
  const supabase = await createClient();
  const [{ data: rating }, { data: tried }] = await Promise.all([
    supabase.from("ratings").select("rating").eq("user_id", userId).eq("perf", "puzzle").maybeSingle(),
    supabase.from("puzzle_attempts").select("puzzle_id").eq("user_id", userId).limit(20000),
  ]);
  return { rating: Math.round((rating?.rating as number | undefined) ?? 1500), tried: new Set((tried ?? []).map((t) => t.puzzle_id as string)) };
}

export type PuzzleResponse = { ok: true; puzzle: ClientPuzzle; rated: boolean; rating: number | null } | Failure;

/** The next puzzle: matched to your rating and never one you've done, or any mid-level puzzle when signed out. */
export async function nextPuzzle(theme?: string): Promise<PuzzleResponse> {
  const me = await getCurrentProfile();
  const h = me ? await history(me.id) : null;
  const puzzle = pickPuzzle({ target: h?.rating ?? 1300, exclude: h?.tried, theme: theme || undefined });
  if (!puzzle) return { ok: false, error: "You've done every puzzle for that theme. Try another one." };
  return { ok: true, puzzle, rated: !!me, rating: h?.rating ?? null };
}

/** Today's puzzle, the same for everyone. */
export type DailyResponse = { ok: true; puzzle: ClientPuzzle; rated: boolean; rating: number | null; date: string; done: boolean } | Failure;

export async function dailyPuzzle(): Promise<DailyResponse> {
  const date = new Date().toISOString().slice(0, 10);
  const puzzle = dailyPuzzleFor(date);
  const me = await getCurrentProfile();
  let done = false;
  let rating: number | null = null;
  if (me) {
    const h = await history(me.id);
    done = h.tried.has(puzzle.id);
    rating = h.rating;
  }
  return { ok: true, puzzle, rated: !!me && !done, rating, date, done };
}

export type SubmitResult = { ok: true; repeat: boolean; before?: number; after?: number; rating?: number } | Failure;

const submitSchema = z.object({ puzzleId: z.string().min(1).max(16), played: z.array(z.string().regex(/^[a-h][1-8][a-h][1-8][qrbn]?$/)).max(12), failed: z.boolean() });

/** Record a finished puzzle. The server checks any claim of success against the real solution. */
export async function submitPuzzle(input: z.input<typeof submitSchema>): Promise<SubmitResult> {
  await requireProfile();
  const s = secret();
  if (!s) return NOT_CONFIGURED;
  const parsed = submitSchema.safeParse(input);
  if (!parsed.success) return { ok: false, error: "Bad puzzle result." };
  const puzzle = lookup(parsed.data.puzzleId);
  if (!puzzle) return { ok: false, error: "Unknown puzzle." };
  const solved = !parsed.data.failed && verifySolution(puzzle, parsed.data.played);

  const supabase = await createClient();
  const { data, error } = await supabase.rpc("server_record_puzzle", {
    p_secret: s, p_puzzle_id: puzzle.id, p_puzzle_rating: puzzle.rating, p_puzzle_rd: puzzle.rd, p_solved: solved,
  });
  if (error) return { ok: false, error: "Could not save the result." };
  const out = data as { repeat: boolean; before?: number; after?: number; rating?: number };
  return { ok: true, ...out };
}

// ------------------------------------------------------------------------------------------------ Puzzle Rush

export async function startRush(): Promise<{ ok: true; sessionId: string; puzzles: ClientPuzzle[] } | Failure> {
  await requireProfile();
  const s = secret();
  if (!s) return NOT_CONFIGURED;
  const supabase = await createClient();
  const { data, error } = await supabase.rpc("server_rush_start", { p_secret: s });
  if (error) return { ok: false, error: "Could not start a run." };
  return { ok: true, sessionId: data as string, puzzles: rushBatch(500, 30, new Set()) };
}

/** More puzzles for a running Puzzle Rush, continuing the difficulty ramp. */
export async function moreRushPuzzles(startRating: number, seen: string[]): Promise<{ ok: true; puzzles: ClientPuzzle[] } | Failure> {
  const parsed = z.object({ r: z.number().min(400).max(2800), seen: z.array(z.string().max(16)).max(400) }).safeParse({ r: startRating, seen });
  if (!parsed.success) return { ok: false, error: "Bad request." };
  return { ok: true, puzzles: rushBatch(parsed.data.r, 20, new Set(parsed.data.seen)) };
}

export async function finishRush(sessionId: string, score: number): Promise<{ ok: true; score: number; best: number } | { ok: true; rejected: string } | Failure> {
  await requireProfile();
  const s = secret();
  if (!s) return NOT_CONFIGURED;
  if (!z.string().uuid().safeParse(sessionId).success || !Number.isInteger(score)) return { ok: false, error: "Bad run." };
  const supabase = await createClient();
  const { data, error } = await supabase.rpc("server_rush_finish", { p_secret: s, p_session: sessionId, p_score: score });
  if (error) return { ok: false, error: "Could not save your score." };
  const out = data as { score?: number; best?: number; rejected?: string };
  return out.rejected ? { ok: true, rejected: out.rejected } : { ok: true, score: out.score!, best: out.best! };
}
