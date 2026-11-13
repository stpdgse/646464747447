import { viewAt, type GameRecord } from "@/lib/chess/game";
import { getBot, type Bot } from "@/lib/engine/bots";
import { remaining, TIME_CONTROLS, type Clocks, type Side } from "./clock";
import type { End, Game } from "./ComputerGame";

/**
 * A game against the computer lives in the browser, so it is saved to localStorage after every change.
 * Close the tab, lose the connection or quit the app and the game is waiting when you come back.
 * Clocks are paused while you are away (it is not a rated game), so nobody loses on time for closing the app.
 */
export const SAVE_KEY = "rookery.botgame.v1";
const MAX_AGE_MS = 7 * 24 * 3600_000;

export type SavedGame = {
  v: 1;
  id: string;
  watchable: boolean;
  botId: string;
  /** The whole bot, so a game against an admin-made bot (or one that was changed since) restores exactly as it was, even offline. */
  bot?: Bot;
  player: Side;
  variant: "standard" | "chess960";
  initialFen: string;
  moves: string[];
  tcId: string;
  takebacks: boolean;
  clocks: Clocks;
  end: End | null;
  botEval: number | null;
  startDate: string;
  savedAt: number;
};

/** The game as it stands at `now`, with the running clock folded into the saved clocks. */
export function snapshot(g: Game, now: number): SavedGame {
  let clocks = g.clocks;
  if (g.tc.initialMs > 0 && !g.end) {
    try {
      const turn = viewAt(g.record).turn;
      const n = g.record.moves.length;
      clocks = {
        white: remaining(g.clocks, turn, "white", g.startedAt, now, n),
        black: remaining(g.clocks, turn, "black", g.startedAt, now, n),
      };
    } catch {
      /* keep the stored clocks */
    }
  }
  return {
    v: 1, id: g.id, watchable: g.watchable, botId: g.bot.id, bot: g.bot, player: g.player, variant: g.record.variant === "chess960" ? "chess960" : "standard",
    initialFen: g.record.initialFen, moves: g.record.moves, tcId: g.tc.id, takebacks: g.takebacks, clocks, end: g.end, botEval: g.botEval,
    startDate: g.startDate, savedAt: now,
  };
}

/** Rebuild a playable game from a save, or null if it is not valid any more. The clock restarts now. */
export function restore(s: SavedGame, now: number): Game | null {
  const bot = s.bot ?? getBot(s.botId);
  const tc = TIME_CONTROLS.find((t) => t.id === s.tcId);
  if (!bot || !tc) return null;
  const record: GameRecord = { variant: s.variant, initialFen: s.initialFen, moves: s.moves };
  try {
    viewAt(record);
  } catch {
    return null;
  }
  return {
    id: s.id, watchable: s.watchable, bot, player: s.player, record, tc, takebacks: s.takebacks, clocks: s.clocks, startedAt: now,
    end: s.end, botEval: s.botEval, startDate: s.startDate,
  };
}

const isSaved = (x: unknown): x is SavedGame => {
  const s = x as Partial<SavedGame> | null;
  return !!s && s.v === 1 && typeof s.id === "string" && typeof s.botId === "string" && Array.isArray(s.moves) && typeof s.initialFen === "string" && typeof s.savedAt === "number";
};

export function loadSaved(now: number): SavedGame | null {
  try {
    const raw = window.localStorage.getItem(SAVE_KEY);
    if (!raw) return null;
    const s = JSON.parse(raw);
    if (!isSaved(s) || now - s.savedAt > MAX_AGE_MS) return null;
    return s;
  } catch {
    return null;
  }
}

export function writeSaved(s: SavedGame) {
  try {
    window.localStorage.setItem(SAVE_KEY, JSON.stringify(s));
  } catch {
    /* storage full or blocked: the game still works, it just cannot be restored */
  }
}

export function clearSaved() {
  try {
    window.localStorage.removeItem(SAVE_KEY);
  } catch {
    /* ignore */
  }
}
