import type { Line } from "./uci";

export type Style = "balanced" | "aggressive" | "solid";

export type Bot = {
  id: string;
  name: string;
  avatar: string;
  /** Rough playing strength. Bots are tuned by feel, not calibrated against a rating pool. */
  rating: number;
  blurb: string;
  greeting: string;
  engine: { elo?: number; skill?: number; depth?: number; movetime?: number; multipv: number };
  human: {
    /** Softmax temperature in centipawns: 0 = always the engine's pick, large = near random among candidates. */
    temperature: number;
    /** Chance per move of deliberately picking something other than the top two moves. */
    blunderRate: number;
    style: Style;
    /** Centipawn bonus applied to the style's preferred kind of move. */
    styleBonus: number;
    /** 0-1: how often the bot notices a forced mate in its favour. Beginners miss them, as beginners do. */
    mateSight: number;
  };
};

export const BOTS: Bot[] = [
  {
    id: "pawnie", name: "Pawnie", avatar: "🐣", rating: 400,
    blurb: "Just learned how the pieces move. Loves hanging them.",
    greeting: "Hi! Is it my turn yet?",
    engine: { depth: 1, multipv: 10 },
    human: { temperature: 260, blunderRate: 0.25, style: "balanced", styleBonus: 0, mateSight: 0.35 },
  },
  {
    id: "rae", name: "Rookie Rae", avatar: "🎒", rating: 700,
    blurb: "Knows a few tricks but misses most of yours.",
    greeting: "Good luck! I've been practising.",
    engine: { depth: 2, multipv: 8 },
    human: { temperature: 150, blunderRate: 0.15, style: "balanced", styleBonus: 0, mateSight: 0.55 },
  },
  {
    id: "cam", name: "Casual Cam", avatar: "🍕", rating: 1000,
    blurb: "Plays between snacks. Solid-ish, easily distracted.",
    greeting: "Let's have a chill game.",
    engine: { depth: 3, multipv: 6 },
    human: { temperature: 90, blunderRate: 0.08, style: "balanced", styleBonus: 0, mateSight: 0.8 },
  },
  {
    id: "bert", name: "Blunder Bert", avatar: "🎲", rating: 1300,
    blurb: "Plays sharp chess, then randomly throws a piece away.",
    greeting: "Don't worry, I'll make a mistake eventually.",
    engine: { depth: 8, multipv: 4 },
    human: { temperature: 15, blunderRate: 0.12, style: "balanced", styleBonus: 0, mateSight: 0.95 },
  },
  {
    id: "tess", name: "Tactician Tess", avatar: "⚔️", rating: 1500,
    blurb: "Always attacking. Captures and checks are her favourites.",
    greeting: "Prepare to be attacked.",
    engine: { depth: 6, multipv: 5 },
    human: { temperature: 25, blunderRate: 0.02, style: "aggressive", styleBonus: 45, mateSight: 1 },
  },
  {
    id: "finn", name: "Fortress Finn", avatar: "🛡️", rating: 1800,
    blurb: "Patient and solid. Hard to crack, happy to wait.",
    greeting: "No rush. I can wait all day.",
    engine: { elo: 1750, movetime: 500, multipv: 3 },
    human: { temperature: 8, blunderRate: 0, style: "solid", styleBonus: 25, mateSight: 1 },
  },
  {
    id: "max", name: "Master Max", avatar: "🎓", rating: 2300,
    blurb: "A seasoned tournament player. Few mistakes.",
    greeting: "Let's see what you've got.",
    engine: { elo: 2300, movetime: 800, multipv: 1 },
    human: { temperature: 0, blunderRate: 0, style: "balanced", styleBonus: 0, mateSight: 1 },
  },
  {
    id: "stockfish", name: "Stockfish", avatar: "🤖", rating: 3000,
    blurb: "The full engine, no mercy. Don't expect to win.",
    greeting: "…",
    engine: { movetime: 1200, multipv: 1 },
    human: { temperature: 0, blunderRate: 0, style: "balanced", styleBonus: 0, mateSight: 1 },
  },
];

export const BOT_IDS = BOTS.map((b) => b.id);
export const getBot = (id: string): Bot | undefined => BOTS.find((b) => b.id === id);

export type MoveFeatures = { capture: boolean; check: boolean; promotion: boolean };

const CAP = 1500;
const clampScore = (cp: number) => Math.max(-CAP, Math.min(CAP, cp));

/**
 * Turn the engine's ranked candidate lines into the move this bot actually plays.
 * Pure and deterministic given `rng`, so it is unit-tested without an engine.
 */
export function chooseMove(
  lines: Line[],
  bot: Bot,
  features: Map<string, MoveFeatures>,
  rng: () => number = Math.random,
  opts: { ply?: number } = {},
): string | null {
  if (lines.length === 0) return null;
  if (lines.length === 1) return lines[0].uci;
  const { blunderRate, style, styleBonus } = bot.human;
  // Early moves get a little extra variety so strong bots do not play the same opening every game.
  const early = (opts.ply ?? 99) < 8;
  const temperature = bot.human.temperature === 0 && early ? 30 : bot.human.temperature;

  // Deliberate mistake: anything outside the top two.
  if (lines.length > 2 && blunderRate > 0 && rng() < blunderRate) {
    const rest = lines.slice(2);
    return rest[Math.floor(rng() * rest.length)].uci;
  }

  // A bot that does not see its mate treats the mating move like any other good move.
  const blind = bot.human.mateSight < 1 && rng() >= bot.human.mateSight;
  const calm = Math.max(...lines.filter((l) => !(l.mate !== null && l.mate > 0)).map((l) => clampScore(l.cp)), -CAP);

  const scored = lines.map((l) => {
    const f = features.get(l.uci);
    let bonus = 0;
    if (f) {
      const tactical = f.capture || f.check || f.promotion;
      if (style === "aggressive" && tactical) bonus = styleBonus;
      if (style === "solid" && !tactical) bonus = styleBonus;
    }
    const mating = l.mate !== null && l.mate > 0;
    return { uci: l.uci, score: (blind && mating ? calm : clampScore(l.cp)) + bonus };
  });
  const top = Math.max(...scored.map((s) => s.score));

  if (temperature <= 0) {
    const tied = scored.filter((s) => s.score === top);
    return tied[Math.floor(rng() * tied.length)].uci;
  }

  const weights = scored.map((s) => Math.exp((s.score - top) / temperature));
  const total = weights.reduce((a, b) => a + b, 0);
  let r = rng() * total;
  for (let i = 0; i < scored.length; i++) {
    r -= weights[i];
    if (r <= 0) return scored[i].uci;
  }
  return scored[0].uci;
}

/** How many candidate lines to request for this move (more at the start for variety). */
export function candidatesFor(bot: Bot, ply: number): number {
  return ply < 8 && bot.engine.multipv < 3 ? 3 : bot.engine.multipv;
}
