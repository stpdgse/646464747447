import type { GameRecord, Result } from "@/lib/chess/game";
import type { Variant } from "@/lib/chess/variants";

export type GameStatus = "started" | "finished" | "aborted";

/** A row of public.games as the API returns it. */
export type GameRow = {
  id: string;
  variant: Variant;
  initial_fen: string;
  moves: string[];
  white_id: string;
  black_id: string;
  status: GameStatus;
  result: Result | null;
  reason: string | null;
  time_initial_ms: number;
  time_inc_ms: number;
  white_clock_ms: number;
  black_clock_ms: number;
  clock_updated_at: string;
  draw_offer_by: "white" | "black" | null;
  rated: boolean;
  created_at: string;
  ended_at: string | null;
};

export type SeekRow = {
  id: string;
  user_id: string;
  target_id: string | null;
  variant: Variant;
  time_initial_ms: number;
  time_inc_ms: number;
  color: "white" | "black" | "random";
  created_at: string;
  expires_at: string;
};

export type Names = { white: string; black: string };

export const recordOf = (row: Pick<GameRow, "variant" | "initial_fen" | "moves">): GameRecord => ({
  variant: row.variant,
  initialFen: row.initial_fen,
  moves: row.moves,
});

/** Rows only ever move forward: more moves, or the same moves with a status/offer/clock change. */
export const isNewer = (prev: GameRow, next: GameRow): boolean => {
  if (next.moves.length !== prev.moves.length) return next.moves.length > prev.moves.length;
  if (prev.status !== "started" && next.status === "started") return false;
  return true;
};

export const FIRST_MOVE_DEADLINE_MS = 60_000;

export type Side = "white" | "black";

/**
 * Time left on a side's clock, in ms. `serverNow` is the database's current time as estimated by the client.
 * The side to move is charged for the time since the last update, once both players have moved.
 */
export function clockLeft(row: GameRow, side: Side, serverNow: number): number {
  const base = side === "white" ? row.white_clock_ms : row.black_clock_ms;
  if (row.status !== "started") return base;
  const ply = row.moves.length;
  const turn: Side = ply % 2 === 0 ? "white" : "black";
  if (turn !== side || ply < 2) return base;
  return Math.max(0, base - Math.max(0, serverNow - Date.parse(row.clock_updated_at)));
}

/** Has the game stalled? (flag fell, or nobody made the first move in time) */
export function needsTimeoutCheck(row: GameRow, serverNow: number): boolean {
  if (row.status !== "started") return false;
  const ply = row.moves.length;
  if (ply < 2) return serverNow - Date.parse(row.clock_updated_at) > FIRST_MOVE_DEADLINE_MS;
  const turn: Side = ply % 2 === 0 ? "white" : "black";
  return clockLeft(row, turn, serverNow) <= 0;
}

export const sideOf = (row: GameRow, userId: string | null | undefined): Side | null =>
  !userId ? null : userId === row.white_id ? "white" : userId === row.black_id ? "black" : null;
