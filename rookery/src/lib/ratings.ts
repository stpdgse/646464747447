import type { Variant } from "@/lib/chess/variants";
import { categoryOf } from "@/lib/live/timecontrols";

export type Perf = "bullet" | "blitz" | "rapid" | "chess960" | "kingOfTheHill" | "threeCheck";

export const PERFS: { id: Perf; label: string }[] = [
  { id: "bullet", label: "Bullet" },
  { id: "blitz", label: "Blitz" },
  { id: "rapid", label: "Rapid" },
  { id: "chess960", label: "Chess960" },
  { id: "kingOfTheHill", label: "King of the Hill" },
  { id: "threeCheck", label: "Three-check" },
];

export const perfLabel = (p: string) => PERFS.find((x) => x.id === p)?.label ?? p;

/** Which rating a game counts towards. Mirrors private.perf_of in the database. */
export function perfOf(variant: Variant, timeMs: number, incMs: number): Perf {
  if (variant !== "standard") return variant;
  return categoryOf(timeMs, incMs).toLowerCase() as Perf;
}

/** A rating is "provisional" until its uncertainty (rating deviation) has shrunk below 110. */
export const isProvisional = (rd: number) => rd > 110;

export type RatingRow = { user_id: string; perf: Perf; rating: number; rd: number; games: number };
export type HistoryPoint = { game_id: string; perf: string; rating_before: number; rating_after: number; created_at: string };

export const formatRating = (rating: number, rd: number) => `${Math.round(rating)}${isProvisional(rd) ? "?" : ""}`;

export const signed = (n: number) => (n > 0 ? `+${n}` : n < 0 ? `−${Math.abs(n)}` : "±0");

/** Wins, draws and losses for `userId` over finished games. */
export function tally(games: { white_id: string; black_id: string; result: string | null }[], userId: string) {
  let wins = 0, draws = 0, losses = 0;
  for (const g of games) {
    if (!g.result) continue;
    if (g.result === "1/2-1/2") draws++;
    else if ((g.result === "1-0") === (g.white_id === userId)) wins++;
    else losses++;
  }
  return { wins, draws, losses, total: wins + draws + losses };
}

/** "1 win", "2 wins". */
export const plural = (n: number, one: string, many: string = `${one}s`) => `${n} ${n === 1 ? one : many}`;
