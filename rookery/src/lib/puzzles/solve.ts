import { tryUci, viewAt, type GameRecord } from "@/lib/chess/game";

/** What the browser needs to run a puzzle. `moves` is Lichess's list: the opponent's first move, then alternating solver/opponent moves. */
export type ClientPuzzle = { id: string; fen: string; moves: string[]; rating: number; rd: number; themes: string[] };

export const puzzleStart = (p: ClientPuzzle): GameRecord => ({ variant: "standard", initialFen: p.fen, moves: [] });

/** The position the solver sees: the puzzle FEN after the opponent's first move. */
export function afterOpponentOpening(p: ClientPuzzle): GameRecord {
  const res = tryUci(puzzleStart(p), p.moves[0]);
  if (!res.ok) throw new Error(`Puzzle ${p.id} has an illegal first move`);
  return res.record;
}

/** How many moves the solver has to find. */
export const solverMoveCount = (p: ClientPuzzle) => Math.ceil((p.moves.length - 1) / 2);

/** Moves are compared in our canonical notation, so castling written either way matches. */
export function canonical(record: GameRecord, uci: string): string | null {
  const res = tryUci(record, uci);
  return res.ok ? res.record.moves[res.record.moves.length - 1] : null;
}

export type StepResult =
  | { kind: "wrong" }
  | { kind: "correct"; record: GameRecord; done: boolean; reply?: string }
  | { kind: "mate"; record: GameRecord };

/**
 * Judge the solver's `k`-th move (0-based) from `record`. Like Lichess, a different move that checkmates also solves the puzzle.
 * For a correct move that isn't the last, `reply` is the opponent's answer to play next.
 */
export function judgeMove(p: ClientPuzzle, k: number, record: GameRecord, uci: string): StepResult {
  const expected = p.moves[1 + 2 * k];
  if (expected === undefined) return { kind: "wrong" };
  const played = tryUci(record, uci);
  if (!played.ok) return { kind: "wrong" };
  const playedCanon = played.record.moves[played.record.moves.length - 1];
  if (playedCanon === canonical(record, expected)) {
    const last = 1 + 2 * k >= p.moves.length - 1;
    return { kind: "correct", record: played.record, done: last, reply: last ? undefined : p.moves[2 + 2 * k] };
  }
  if (viewAt(played.record).outcome?.reason === "checkmate") return { kind: "mate", record: played.record };
  return { kind: "wrong" };
}

/**
 * Server-side check of a claimed solution: replay the solver's moves against the real puzzle.
 * `played` holds only the solver's moves, in order.
 */
export function verifySolution(p: ClientPuzzle, played: string[]): boolean {
  let record = afterOpponentOpening(p);
  for (let k = 0; k < solverMoveCount(p); k++) {
    const uci = played[k];
    if (!uci) return false;
    const step = judgeMove(p, k, record, uci);
    if (step.kind === "wrong") return false;
    if (step.kind === "mate") return true;
    record = step.record;
    if (step.done) return true;
    const reply = tryUci(record, step.reply!);
    if (!reply.ok) return false;
    record = reply.record;
  }
  return true;
}

/** The full solution as solver moves, for tests and "show solution". */
export const solutionMoves = (p: ClientPuzzle) => p.moves.filter((_, i) => i > 0 && i % 2 === 1);
