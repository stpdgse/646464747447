import { tryUci, viewAt, type GameRecord } from "@/lib/chess/game";
import type { UciEngine } from "@/lib/engine/uci";

/** Centipawns are clamped like Lichess does before turning them into win chances. */
export const clampCp = (cp: number) => Math.max(-1000, Math.min(1000, cp));

/** White's chance of winning, 0-100, from an evaluation in centipawns (white's point of view). Lichess's logistic. */
export const winPercent = (cpWhite: number) => 50 + 50 * (2 / (1 + Math.exp(-0.00368208 * clampCp(cpWhite))) - 1);

/** Lichess's per-move accuracy from the mover's win chance before and after the move. */
export const moveAccuracy = (winBefore: number, winAfter: number) =>
  Math.max(0, Math.min(100, 103.1668 * Math.exp(-0.04354 * (winBefore - winAfter)) - 3.1669));

export type Judgement = "best" | "good" | "inaccuracy" | "mistake" | "blunder" | "book";

/** Thresholds are drops in win chance (percentage points): 5, 10, 15, the same cut-offs as Lichess's 0.1/0.2/0.3 in chances. */
export function classify(lossPoints: number, isBest: boolean): Judgement {
  if (lossPoints >= 15) return "blunder";
  if (lossPoints >= 10) return "mistake";
  if (lossPoints >= 5) return "inaccuracy";
  return isBest || lossPoints <= 0.5 ? "best" : "good";
}

export const SYMBOL: Record<Judgement, string> = { best: "", good: "", book: "", inaccuracy: "?!", mistake: "?", blunder: "??" };

export type EvalPoint = {
  /** Centipawns from White's point of view. Mates are +/-100000. */
  cp: number;
  /** Moves until mate (White's point of view: positive = White mates), 0 if the game is already over by mate. */
  mate: number | null;
  /** The engine's preferred move here, if searched. */
  best: string | null;
  /** Searched to a result by the engine (false for finished positions). */
  searched: boolean;
};

export type MoveReview = {
  ply: number;
  uci: string;
  san: string;
  mover: "white" | "black";
  judgement: Judgement;
  /** How much win chance the move gave away, in percentage points (never negative). */
  loss: number;
  accuracy: number;
  bestUci: string | null;
  bestSan: string | null;
};

export type Review = {
  evals: EvalPoint[]; // one per position, 0..n
  moves: MoveReview[];
  accuracy: { white: number | null; black: number | null };
  counts: Record<"white" | "black", Record<Judgement, number>>;
};

const emptyCounts = (): Record<Judgement, number> => ({ best: 0, good: 0, book: 0, inaccuracy: 0, mistake: 0, blunder: 0 });

const stdev = (xs: number[]) => {
  const mean = xs.reduce((a, b) => a + b, 0) / xs.length;
  return Math.sqrt(xs.reduce((a, b) => a + (b - mean) ** 2, 0) / xs.length);
};

/**
 * Accuracy for one player's moves, like Lichess: the average of a volatility-weighted mean (quiet positions count less
 * than wild ones) and a harmonic mean (which punishes the odd terrible move). `winSeries` is White's win chance per position.
 */
export function gameAccuracy(moveAccuracies: { accuracy: number; index: number }[], winSeries: number[]): number | null {
  if (moveAccuracies.length === 0) return null;
  const size = Math.max(2, Math.min(8, Math.floor(winSeries.length / 10)));
  const weight = (i: number) => {
    const from = Math.max(0, i - Math.floor(size / 2));
    const slice = winSeries.slice(from, Math.min(winSeries.length, from + size + 1));
    return Math.max(0.5, Math.min(12, slice.length > 1 ? stdev(slice) : 0.5));
  };
  let num = 0, den = 0, inv = 0;
  for (const m of moveAccuracies) {
    const w = weight(m.index);
    num += m.accuracy * w;
    den += w;
    inv += 1 / Math.max(m.accuracy, 0.001);
  }
  const weighted = num / den;
  const harmonic = moveAccuracies.length / inv;
  return (weighted + harmonic) / 2;
}

/** Turn a list of evaluations into the full review. Pure: no engine needed. */
export function buildReview(record: GameRecord, evals: EvalPoint[], bookPly = 0): Review {
  const winSeries = evals.map((e) => winPercent(e.cp));
  const moves: MoveReview[] = [];
  const counts = { white: emptyCounts(), black: emptyCounts() };
  const acc: Record<"white" | "black", { accuracy: number; index: number }[]> = { white: [], black: [] };

  for (let i = 0; i < record.moves.length; i++) {
    const mover = i % 2 === 0 ? "white" : "black"; // standard starts: White moves first
    const flip = (w: number) => (mover === "white" ? w : 100 - w);
    const before = flip(winSeries[i]);
    const after = flip(winSeries[i + 1]);
    const loss = Math.max(0, before - after);
    const accuracy = moveAccuracy(before, after);
    const played = record.moves[i];
    const bestUci = evals[i].best;
    const isBest = bestUci !== null && canonicalEq(record, i, played, bestUci);
    let judgement = classify(loss, isBest);
    if (i < bookPly && loss < 5) judgement = "book";
    const view = viewAt(record, i + 1);
    const bestSan = bestUci && !isBest ? sanFor(record, i, bestUci) : null;
    moves.push({ ply: i + 1, uci: played, san: view.sans[i], mover, judgement, loss, accuracy, bestUci, bestSan });
    counts[mover][judgement]++;
    acc[mover].push({ accuracy, index: i });
  }
  return {
    evals,
    moves,
    accuracy: { white: gameAccuracy(acc.white, winSeries), black: gameAccuracy(acc.black, winSeries) },
    counts,
  };
}

function prefix(record: GameRecord, i: number): GameRecord {
  return { ...record, moves: record.moves.slice(0, i) };
}
function canonicalEq(record: GameRecord, i: number, a: string, b: string): boolean {
  if (a === b) return true;
  const p = prefix(record, i);
  const ra = tryUci(p, a), rb = tryUci(p, b);
  return ra.ok && rb.ok && ra.record.moves.at(-1) === rb.record.moves.at(-1);
}
function sanFor(record: GameRecord, i: number, uci: string): string | null {
  const res = tryUci(prefix(record, i), uci);
  return res.ok ? viewAt(res.record).sans.at(-1) ?? null : null;
}

export type AnalyseOptions = { depth?: number; movetime?: number; bookPly?: number; onProgress?: (done: number, total: number) => void; signal?: AbortSignal };

/** Evaluate every position of the game with the engine, then build the review. */
export async function analyseGame(engine: UciEngine, record: GameRecord, opts: AnalyseOptions = {}): Promise<Review> {
  const { depth = 12, movetime = 220, onProgress, signal } = opts;
  const total = record.moves.length + 1;
  const evals: EvalPoint[] = [];
  for (let i = 0; i < total; i++) {
    if (signal?.aborted) throw new DOMException("Analysis cancelled", "AbortError");
    const view = viewAt(record, i);
    if (view.outcome) {
      const cp = view.outcome.result === "1-0" ? 100000 : view.outcome.result === "0-1" ? -100000 : 0;
      evals.push({ cp, mate: view.outcome.reason === "checkmate" ? 0 : null, best: null, searched: false });
    } else {
      const res = await engine.search({ fen: record.initialFen, moves: record.moves.slice(0, i), depth, movetime, multipv: 1 }, signal);
      if (signal?.aborted) throw new DOMException("Analysis cancelled", "AbortError");
      const line = res.lines[0];
      const sign = view.turn === "white" ? 1 : -1;
      evals.push({ cp: line ? sign * line.cp : 0, mate: line?.mate != null ? sign * line.mate : null, best: res.best, searched: true });
    }
    onProgress?.(i + 1, total);
  }
  return buildReview(record, evals, opts.bookPly ?? 0);
}

/** Format an evaluation for display: "+1.4", "-0.3", "M3", "#". */
export function formatEval(e: Pick<EvalPoint, "cp" | "mate">): string {
  if (e.mate !== null) return e.mate === 0 ? "#" : `${e.mate > 0 ? "" : "-"}M${Math.abs(e.mate)}`;
  const v = e.cp / 100;
  return `${v > 0 ? "+" : ""}${v.toFixed(1)}`;
}
