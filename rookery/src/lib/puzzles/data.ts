import "server-only";
import raw from "@/data/puzzles.json";
import type { ClientPuzzle } from "./solve";

type Tuple = [id: string, fen: string, moves: string, rating: number, rd: number, themes: string];

const ALL: ClientPuzzle[] = (raw as Tuple[]).map(([id, fen, moves, rating, rd, themes]) => ({
  id, fen, moves: moves.split(" "), rating, rd, themes: themes.split(" "),
}));
// Already sorted by rating then id.

export const puzzleCount = ALL.length;
export const getPuzzleById = (id: string): ClientPuzzle | undefined => ALL.find((p) => p.id === id);

const BY_ID = new Map(ALL.map((p) => [p.id, p]));
export const lookup = (id: string) => BY_ID.get(id);

function lowerBound(rating: number): number {
  let lo = 0, hi = ALL.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (ALL[mid].rating < rating) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}

export type PickOptions = { target: number; exclude?: ReadonlySet<string>; theme?: string; rng?: () => number };

/** A random puzzle near `target`, skipping `exclude`, widening the search until something fits. */
export function pickPuzzle({ target, exclude, theme, rng = Math.random }: PickOptions): ClientPuzzle | null {
  for (let width = 60; width <= 20000; width *= 2) {
    const lo = lowerBound(target - width), hi = lowerBound(target + width);
    const pool: ClientPuzzle[] = [];
    for (let i = lo; i < hi; i++) {
      const p = ALL[i];
      if (exclude?.has(p.id)) continue;
      if (theme && !p.themes.includes(theme)) continue;
      pool.push(p);
    }
    if (pool.length) return pool[Math.floor(rng() * pool.length)];
    if (lo === 0 && hi === ALL.length) break;
  }
  return null;
}

/** One puzzle for every UTC date, the same for everyone: well-liked puzzles in the middle of the rating range. */
export function dailyPuzzleFor(date: string): ClientPuzzle {
  const pool = ALL.filter((p) => p.rating >= 1100 && p.rating <= 2100 && p.themes.some((t) => ["mateIn2", "mateIn3", "fork", "pin", "skewer", "discoveredAttack", "sacrifice", "deflection", "attraction", "quietMove", "hangingPiece", "backRankMate", "trappedPiece"].includes(t)));
  let h = 2166136261;
  for (const ch of date) h = Math.imul(h ^ ch.charCodeAt(0), 16777619) >>> 0;
  return pool[h % pool.length];
}

/** A ramp of puzzles for Puzzle Rush: gently rising difficulty from `startRating`. */
export function rushBatch(startRating: number, count: number, exclude: ReadonlySet<string>, rng: () => number = Math.random): ClientPuzzle[] {
  const out: ClientPuzzle[] = [];
  const used = new Set(exclude);
  for (let i = 0; i < count; i++) {
    const p = pickPuzzle({ target: Math.min(2800, startRating + i * 45), exclude: used, rng });
    if (!p) break;
    used.add(p.id);
    out.push(p);
  }
  return out;
}

export const THEMES = [
  { id: "mateIn1", label: "Mate in 1" }, { id: "mateIn2", label: "Mate in 2" }, { id: "mateIn3", label: "Mate in 3" },
  { id: "fork", label: "Fork" }, { id: "pin", label: "Pin" }, { id: "skewer", label: "Skewer" },
  { id: "discoveredAttack", label: "Discovered attack" }, { id: "hangingPiece", label: "Hanging piece" },
  { id: "backRankMate", label: "Back-rank mate" }, { id: "sacrifice", label: "Sacrifice" },
  { id: "deflection", label: "Deflection" }, { id: "quietMove", label: "Quiet move" },
  { id: "endgame", label: "Endgame" }, { id: "opening", label: "Opening" }, { id: "middlegame", label: "Middlegame" },
  { id: "promotion", label: "Promotion" }, { id: "trappedPiece", label: "Trapped piece" },
] as const;
