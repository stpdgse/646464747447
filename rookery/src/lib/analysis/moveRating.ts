import { tryUci, viewAt, type GameRecord } from "@/lib/chess/game";
import { VARIANTS } from "@/lib/chess/variants";
import type { UciEngine } from "@/lib/engine/uci";
import { parseFen } from "chessops/fen";
import type { Role } from "chessops/types";
import { parseUci } from "chessops/util";
import { setupPosition } from "chessops/variant";
import { classify, winPercent } from "./analysis";

/** How good a move was, from the point of view of the player who made it. */
export type MoveRating = "brilliant" | "great" | "best" | "good" | "inaccuracy" | "mistake" | "blunder";

export const RATING_UI: Record<MoveRating, { label: string; symbol: string; bg: string; blurb: string }> = {
  brilliant: { label: "Brilliant", symbol: "!!", bg: "#3fd0c5", blurb: "A sacrifice that works. Superb." },
  great: { label: "Great", symbol: "!", bg: "#7fb7e6", blurb: "The only move that kept your position strong." },
  best: { label: "Best", symbol: "★", bg: "#95bb4a", blurb: "The engine's top choice." },
  good: { label: "Good", symbol: "✓", bg: "#b9dc83", blurb: "A solid move." },
  inaccuracy: { label: "Inaccuracy", symbol: "?!", bg: "#f2d15b", blurb: "There was a slightly better move." },
  mistake: { label: "Mistake", symbol: "?", bg: "#f0a04b", blurb: "This cost you a real advantage." },
  blunder: { label: "Blunder", symbol: "??", bg: "#ef6b63", blurb: "This move threw the position away." },
};

const VALUE: Record<Role, number> = { pawn: 1, knight: 3, bishop: 3, rook: 5, queen: 9, king: 100 };

function positionOf(record: GameRecord, ply = record.moves.length) {
  const fen = viewAt(record, ply).fen;
  return setupPosition(VARIANTS[record.variant].rules, parseFen(fen).unwrap()).unwrap();
}

/**
 * Did this move give up material for nothing immediate? True when a knight, bishop, rook or queen lands where it can be
 * captured by a cheaper piece (or for free), and the move wins back at least two points less than the piece is worth.
 * Used with the engine's verdict: a "sacrifice" that the engine still likes is brilliant.
 */
export function isSacrifice(before: GameRecord, uci: string): boolean {
  const move = parseUci(uci);
  if (!move || !("from" in move)) return false;
  const res = tryUci(before, uci);
  if (!res.ok) return false;
  try {
    const pre = positionOf(before);
    const mover = pre.board.get(move.from);
    if (!mover || mover.role === "king" || mover.role === "pawn") return false; // castling and pawn pushes are never "sacrifices" here
    const taken = pre.board.get(move.to);
    const capturedValue = taken ? VALUE[taken.role] : 0;
    const post = positionOf(res.record);
    const landed = post.board.get(move.to);
    if (!landed) return false;
    const movedValue = VALUE[landed.role];
    if (movedValue < 3 || movedValue - capturedValue < 2) return false;

    let minAttacker = Infinity;
    let attackerFrom: number | null = null;
    for (const [from, dests] of post.allDests()) {
      if (!dests.has(move.to)) continue;
      const role = post.board.getRole(from);
      if (!role) continue;
      if (VALUE[role] < minAttacker) {
        minAttacker = VALUE[role];
        attackerFrom = from;
      }
    }
    if (attackerFrom === null) return false; // nothing can take it
    const after = post.clone();
    after.play({ from: attackerFrom, to: move.to });
    let defended = false;
    for (const [, dests] of after.allDests()) if (dests.has(move.to)) defended = true;
    return minAttacker < movedValue || !defended;
  } catch {
    return false;
  }
}

export type RateInput = {
  /** Evaluations in centipawns from the mover's point of view. */
  bestCp: number;
  /** The engine's second-best line, if there was one. */
  secondCp: number | null;
  /** The position after the move that was played. */
  afterCp: number;
  /** The move played is the engine's top choice. */
  isBest: boolean;
  sacrifice: boolean;
};

/** Pure rating rules, so they can be tested without an engine. Win chances are in percent. */
export function rateMove(i: RateInput): { rating: MoveRating; loss: number } {
  const before = winPercent(i.bestCp);
  const after = winPercent(i.afterCp);
  const loss = i.isBest ? 0 : Math.max(0, before - after);

  // Brilliant: gives up material, still (nearly) the best move, and you are not already winning or losing.
  if (i.sacrifice && loss <= 1.5 && before <= 85 && after >= 45) return { rating: "brilliant", loss };

  const base = classify(loss, i.isBest);
  if (base === "best" || base === "book") {
    // Great: the best move, and the next best was clearly worse (an "only move"), in a position still worth playing for.
    if (i.secondCp !== null && before - winPercent(i.secondCp) >= 12 && before >= 15 && before <= 96) return { rating: "great", loss };
    return { rating: "best", loss };
  }
  return { rating: base, loss };
}

export type RatedMove = { rating: MoveRating; loss: number; playedSan: string; bestSan: string | null; bestUci: string | null };

function sameMove(before: GameRecord, a: string, b: string): boolean {
  if (a === b) return true;
  const ra = tryUci(before, a), rb = tryUci(before, b);
  return ra.ok && rb.ok && ra.record.moves.at(-1) === rb.record.moves.at(-1);
}

/**
 * Rate `playedUci`, which was just played from the position after `before.moves`.
 * Two short searches: the position before (two lines, to spot only-moves) and the position after.
 */
export async function rateMoveWithEngine(
  engine: UciEngine, before: GameRecord, playedUci: string, opts: { depth?: number; movetime?: number; signal?: AbortSignal } = {},
): Promise<RatedMove | null> {
  const { depth = 12, movetime = 350, signal } = opts;
  const played = tryUci(before, playedUci);
  if (!played.ok) return null;
  const mover = viewAt(before).turn;
  const playedView = viewAt(played.record);
  const playedSan = playedView.sans.at(-1) ?? playedUci;

  const pre = await engine.search({ fen: before.initialFen, moves: before.moves, depth, movetime, multipv: 2 }, signal);
  if (signal?.aborted) return null;
  const lines = [...pre.lines].sort((a, b) => a.multipv - b.multipv);
  const best = lines[0];
  if (!best) return null;
  const second = lines[1] ?? null;

  let afterCp: number;
  if (playedView.outcome) {
    afterCp = playedView.outcome.result === "1/2-1/2" ? 0 : (playedView.outcome.result === "1-0") === (mover === "white") ? 100_000 : -100_000;
  } else {
    const post = await engine.search({ fen: before.initialFen, moves: played.record.moves, depth, movetime, multipv: 1 }, signal);
    if (signal?.aborted) return null;
    afterCp = -(post.lines[0]?.cp ?? 0);
  }

  const bestUci = pre.best ?? best.uci;
  const isBest = sameMove(before, playedUci, bestUci);
  const { rating, loss } = rateMove({ bestCp: best.cp, secondCp: second ? second.cp : null, afterCp, isBest, sacrifice: isSacrifice(before, playedUci) });
  let bestSan: string | null = null;
  if (!isBest) {
    const b = tryUci(before, bestUci);
    if (b.ok) bestSan = viewAt(b.record).sans.at(-1) ?? null;
  }
  return { rating, loss, playedSan, bestSan, bestUci: isBest ? null : bestUci };
}
