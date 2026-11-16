import { describeMoves, type GameRecord } from "@/lib/chess/game";
import { candidatesFor, chooseMove, type Bot } from "./bots";
import type { UciEngine } from "./uci";

/** How fast a bot battle runs: the most a bot may think per move, and the shortest pause between moves. */
export const SPEEDS = {
  fast: { label: "Fast", capMs: 150, pauseMs: 150 },
  normal: { label: "Normal", capMs: 600, pauseMs: 700 },
  slow: { label: "Slow (easy to follow)", capMs: 1500, pauseMs: 1600 },
} as const;
export type Speed = keyof typeof SPEEDS;

/**
 * The move `bot` plays in this position: its engine searches, then the bot's personality (noise, blunders, style) picks from
 * the candidates. `movetimeCap` keeps strong bots from thinking too long. Returns null if the search was cancelled.
 */
export async function pickBotMove(
  engine: UciEngine, bot: Bot, record: GameRecord, opts: { movetimeCap?: number; signal?: AbortSignal; rng?: () => number } = {},
): Promise<string | null> {
  const ply = record.moves.length;
  const movetime = bot.engine.movetime !== undefined ? Math.min(bot.engine.movetime, opts.movetimeCap ?? Infinity) : opts.movetimeCap;
  let result;
  try {
    result = await engine.search(
      { fen: record.initialFen, moves: record.moves, depth: bot.engine.depth, movetime, multipv: candidatesFor(bot, ply), elo: bot.engine.elo, skill: bot.engine.skill },
      opts.signal,
    );
  } catch {
    return null;
  }
  if (opts.signal?.aborted) return null;
  const feats = describeMoves(record, result.lines.map((l) => l.uci));
  return chooseMove(result.lines, bot, feats, opts.rng ?? Math.random, { ply }) ?? result.best;
}
