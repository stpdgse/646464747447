import { smartnessToPlay, suggestRating, type Bot } from "@/lib/engine/bots";

/** The Rüdenrache holiday challenge: 20 Rüde bots, rated 100, 200 ... 2000. Win to move up; a draw or a loss ends the attempt. */
export const EVENT_KEY = "rude-holiday-2026";
export const MAX_ATTEMPTS = 8;
export const LEVELS = Array.from({ length: 20 }, (_, i) => (i + 1) * 100);
export const TOP_LEVEL = 2000;
/** A win needs at least this many half-moves, so a "win" cannot be a two-move trick against a bot that never played. */
export const MIN_WIN_PLIES = 10;
export const PLY_LIMIT = 400;

/** You play White against the 1st, 3rd, 5th... Rüde and Black against the 2nd, 4th, 6th... */
export const colorForLevel = (level: number): "white" | "black" => ((level / 100) % 2 === 1 ? "white" : "black");

/** The smallest smartness whose label rating reaches `rating`, for the lower half of the ladder. */
const smartnessFor = (rating: number): number => {
  for (let s = 0; s <= 24; s++) if (suggestRating(s) >= rating) return s;
  return 24;
};

const LINES: Record<number, string> = {
  100: "Grrr... I mostly know where the pieces are.",
  500: "I'm just getting started. Ow, my paw.",
  1000: "You made it to the middle of the pack. Don't get comfortable.",
  1500: "The big wolves are watching.",
  2000: "I am the last Rüde. Nobody has passed me.",
};

/** The Rüde of this level as an ordinary bot (never shown in the normal bot lists). */
export function rudeBot(level: number): Bot {
  const i = LEVELS.indexOf(level);
  if (i < 0) throw new Error(`There is no Rüde of level ${level}`);
  let engine: Bot["engine"], human: Bot["human"];
  if (level <= 1300) {
    const p = smartnessToPlay(smartnessFor(level));
    engine = p.engine;
    human = { ...p.human, style: "aggressive", styleBonus: 25 };
  } else {
    // 1400 ... 2000: a real engine at a rising Elo, thinking a little longer each time
    engine = { elo: Math.round(1320 + (level - 1400) * 1.35), movetime: Math.round(250 + (level - 1400) * 0.9), multipv: level < 1700 ? 2 : 1 };
    human = { temperature: Math.max(0, Math.round(10 - (level - 1400) / 60)), blunderRate: level < 1700 ? 0.02 : 0, style: "aggressive", styleBonus: 15, mateSight: 1 };
  }
  return {
    id: `rude-${level}`, name: `Rüde ${level}`, avatar: "🐺", avatarImage: "/bots/ruedenrache.png", rating: level, special: true,
    blurb: `Rüde number ${i + 1} of 20. Rated about ${level}.`,
    greeting: LINES[level] ?? `Rüde ${level} bares its teeth.`,
    quips: { win: "Rüdenrache! Try again.", lose: "Grrr... on to the next one.", draw: "A draw does not beat a wolf." },
    engine, human,
  };
}

/** A hue turn so every Rüde looks a little different (the same pixel wolf, a new colour per level). */
export const hueFor = (level: number): number => ((level / 100 - 1) * 47) % 360;

export type Entry = { user_id: string; username: string; best: number; attempts: number; cleared: boolean; last: string };

/** One row per player: their best level over all attempts, how many attempts they used, and whether they cleared the ladder. Best first. */
export function leaderboard(rows: { user_id: string; username: string; best_level: number; cleared: boolean; updated_at: string }[]): Entry[] {
  const by = new Map<string, Entry>();
  for (const r of rows) {
    const e = by.get(r.user_id) ?? { user_id: r.user_id, username: r.username, best: 0, attempts: 0, cleared: false, last: r.updated_at };
    e.attempts++;
    e.cleared ||= r.cleared;
    if (r.best_level > e.best || (r.best_level === e.best && r.updated_at < e.last)) { e.best = r.best_level; e.last = r.updated_at; }
    by.set(r.user_id, e);
  }
  return [...by.values()].sort((a, b) => Number(b.cleared) - Number(a.cleared) || b.best - a.best || a.attempts - b.attempts || a.last.localeCompare(b.last) || a.username.localeCompare(b.username));
}
