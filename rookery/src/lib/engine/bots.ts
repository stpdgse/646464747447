import type { Line } from "./uci";

/**
 * How a bot leans when several moves are about equally good:
 * balanced = no preference, aggressive = captures/checks/promotions, solid = quiet moves,
 * greedy = grabs material, checker = loves giving check, chaotic = picks with random flair.
 */
export type Style = "balanced" | "aggressive" | "solid" | "greedy" | "checker" | "chaotic";

export type Bot = {
  id: string;
  name: string;
  avatar: string;
  /** Optional pixel-art portrait (a path under /public) shown instead of the emoji. */
  avatarImage?: string;
  /** Rough playing strength. Bots are tuned by feel, not calibrated against a rating pool. */
  rating: number;
  blurb: string;
  greeting: string;
  /** What the bot says when the game ends, from its own point of view. */
  quips: { win: string; lose: string; draw: string };
  /** Highlighted as a special character in the picker. */
  special?: boolean;
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
    id: "dermann", name: "Der Mann", avatar: "🧔", avatarImage: "/bots/dermann.png", rating: 51, special: true,
    blurb: "Just Der Mann. Knows the pieces move. Not where, not why. The weakest bot there is.",
    greeting: "Ich bin Der Mann. Which one is the horse?",
    quips: { win: "Der Mann hat gewonnen?! Das war ein Versehen.", lose: "Ja. Das habe ich erwartet.", draw: "Unentschieden. Der Mann weiß nicht, wie." },
    engine: { depth: 1, multipv: 10 },
    human: { temperature: 5000, blunderRate: 0.9, style: "chaotic", styleBonus: 400, mateSight: 0 },
  },
  {
    id: "pawnie", name: "Pawnie", avatar: "🐣", rating: 400,
    blurb: "Just learned how the pieces move. Loves hanging them.",
    greeting: "Hi! Is it my turn yet?",
    quips: { win: "I won?! Wait, really?", lose: "Oh no, my pieces!", draw: "A draw! That's like winning, right?" },
    engine: { depth: 1, multipv: 10 },
    human: { temperature: 260, blunderRate: 0.25, style: "balanced", styleBonus: 0, mateSight: 0.3 },
  },
  {
    id: "wanda", name: "Wobbly Wanda", avatar: "🦆", rating: 450,
    blurb: "Moves with total confidence and no plan whatsoever.",
    greeting: "Quack! I mean, e4. Probably.",
    quips: { win: "Quack-mate!", lose: "I blame the pond.", draw: "Nobody won. Everybody quacks." },
    engine: { depth: 1, multipv: 10 },
    human: { temperature: 300, blunderRate: 0.25, style: "chaotic", styleBonus: 120, mateSight: 0.3 },
  },
  {
    id: "sam", name: "Sleepy Sam", avatar: "😴", rating: 500,
    blurb: "Falls asleep mid-game. Wakes up to hang a piece.",
    greeting: "*yawn* Oh, is it time?",
    quips: { win: "Huh? I won? Cool. Nap time.", lose: "Zzz... what happened?", draw: "Draw. Fine by me, I'm tired." },
    engine: { depth: 1, multipv: 8 },
    human: { temperature: 240, blunderRate: 0.22, style: "solid", styleBonus: 30, mateSight: 0.35 },
  },
  {
    id: "gus", name: "Greedy Gus", avatar: "🍩", rating: 550,
    blurb: "Takes every piece he can. Never asks if it's a trap.",
    greeting: "Free stuff? Yes please.",
    quips: { win: "Om nom nom, all your pieces are mine!", lose: "I only wanted a snack...", draw: "We split the pieces. Fair enough." },
    engine: { depth: 1, multipv: 8 },
    human: { temperature: 210, blunderRate: 0.2, style: "greedy", styleBonus: 160, mateSight: 0.35 },
  },
  {
    id: "chip", name: "Cheeky Chip", avatar: "🐿️", rating: 600,
    blurb: "Gives check at every opportunity, wise or not.",
    greeting: "Check! Oh, not yet? Soon!",
    quips: { win: "Check, check, checkmate!", lose: "Hey, all those checks should count for something!", draw: "A draw, with a ton of checks." },
    engine: { depth: 2, multipv: 8 },
    human: { temperature: 190, blunderRate: 0.18, style: "checker", styleBonus: 110, mateSight: 0.4 },
  },
  {
    id: "alf", name: "Pong Alf Femboy", avatar: "🏓", rating: 650, special: true,
    blurb: "Table-tennis champion, chess hobbyist. Cheerful, chaotic and loves a trade.",
    greeting: "Pong! Ready for a rally? Ping first, chess second.",
    quips: { win: "Smashed it! Ping, pong, mate!", lose: "Good rally! I'll get you in the rematch.", draw: "A deuce! Let's play another point." },
    engine: { depth: 2, multipv: 8 },
    human: { temperature: 170, blunderRate: 0.17, style: "chaotic", styleBonus: 85, mateSight: 0.5 },
  },
  {
    id: "rae", name: "Rookie Rae", avatar: "🎒", rating: 700,
    blurb: "Knows a few tricks but misses most of yours.",
    greeting: "Good luck! I've been practising.",
    quips: { win: "I did it! All that practice paid off!", lose: "Great game! I learned a lot.", draw: "A draw! Not bad for me." },
    engine: { depth: 2, multipv: 8 },
    human: { temperature: 150, blunderRate: 0.15, style: "balanced", styleBonus: 0, mateSight: 0.55 },
  },
  {
    id: "pip", name: "Panicky Pip", avatar: "🐭", rating: 750,
    blurb: "Scared of everything. Retreats at the first sign of trouble.",
    greeting: "Please be gentle...",
    quips: { win: "Oh! I won? I was so scared!", lose: "I knew it. I knew it!", draw: "Phew, a draw. I'll take it." },
    engine: { depth: 2, multipv: 7 },
    human: { temperature: 140, blunderRate: 0.14, style: "solid", styleBonus: 40, mateSight: 0.55 },
  },
  {
    id: "gabe", name: "Gambit Gabe", avatar: "🎩", rating: 800,
    blurb: "Throws pawns at you for a wild attack. Mostly it doesn't work.",
    greeting: "A gambit a day keeps the boredom away!",
    quips: { win: "The gambit worked! It never works!", lose: "Worth a try. It's always worth a try.", draw: "Drawn, but what a mess!" },
    engine: { depth: 3, multipv: 6 },
    human: { temperature: 130, blunderRate: 0.14, style: "aggressive", styleBonus: 70, mateSight: 0.6 },
  },
  {
    id: "charlie", name: "Chatty Charlie", avatar: "🦜", rating: 850,
    blurb: "Talks nonstop. Plays decent chess in between sentences.",
    greeting: "So, did you see the game last night? Anyway, e4!",
    quips: { win: "And THAT, my friend, is checkmate.", lose: "Hmm. I wasn't paying attention.", draw: "A draw. Rematch? I have stories." },
    engine: { depth: 3, multipv: 6 },
    human: { temperature: 115, blunderRate: 0.12, style: "balanced", styleBonus: 0, mateSight: 0.65 },
  },
  {
    id: "suri", name: "Sneaky Suri", avatar: "🦊", rating: 900,
    blurb: "Loves a surprise check or a quiet trap.",
    greeting: "Nothing to see here...",
    quips: { win: "Gotcha. Didn't see that coming, did you?", lose: "Huh, the trap fell apart.", draw: "Neither of us fell for it." },
    engine: { depth: 3, multipv: 6 },
    human: { temperature: 105, blunderRate: 0.1, style: "checker", styleBonus: 75, mateSight: 0.7 },
  },
  {
    id: "nora", name: "Knight Nora", avatar: "🐴", rating: 950,
    blurb: "Hops about the board looking for forks.",
    greeting: "Neigh! Let's gallop.",
    quips: { win: "Forked and finished!", lose: "You got me. Well played!", draw: "A draw, with plenty of galloping." },
    engine: { depth: 3, multipv: 5 },
    human: { temperature: 95, blunderRate: 0.09, style: "aggressive", styleBonus: 55, mateSight: 0.75 },
  },
  {
    id: "cam", name: "Casual Cam", avatar: "🍕", rating: 1000,
    blurb: "Plays between snacks. Solid-ish, easily distracted.",
    greeting: "Let's have a chill game.",
    quips: { win: "GG! Pizza's on you.", lose: "Nice one. I got distracted by lunch.", draw: "A draw. Chill." },
    engine: { depth: 3, multipv: 6 },
    human: { temperature: 90, blunderRate: 0.08, style: "balanced", styleBonus: 0, mateSight: 0.8 },
  },
  {
    id: "stella", name: "Steady Stella", avatar: "🐢", rating: 1100,
    blurb: "Slow and careful. Rarely loses a piece for free.",
    greeting: "Slow and steady wins the race.",
    quips: { win: "Slow and steady. Checkmate.", lose: "Well played. I'll study that.", draw: "A careful draw." },
    engine: { depth: 4, multipv: 5 },
    human: { temperature: 60, blunderRate: 0.06, style: "solid", styleBonus: 35, mateSight: 0.85 },
  },
  {
    id: "rafa", name: "Rapid Rafa", avatar: "⚡", rating: 1200,
    blurb: "Plays fast and sharp. Sometimes too fast.",
    greeting: "Fast hands! Ready?",
    quips: { win: "Lightning!", lose: "Too fast for my own good.", draw: "Draw. Run it back!" },
    engine: { depth: 5, multipv: 5 },
    human: { temperature: 45, blunderRate: 0.07, style: "aggressive", styleBonus: 50, mateSight: 0.9 },
  },
  {
    id: "bert", name: "Blunder Bert", avatar: "🎲", rating: 1300,
    blurb: "Plays sharp chess, then randomly throws a piece away.",
    greeting: "Don't worry, I'll make a mistake eventually.",
    quips: { win: "Wow, I didn't blunder. Mark the date.", lose: "There it is. The blunder.", draw: "A draw, with only a couple of blunders." },
    engine: { depth: 8, multipv: 4 },
    human: { temperature: 15, blunderRate: 0.12, style: "balanced", styleBonus: 0, mateSight: 0.95 },
  },
  {
    id: "pim", name: "Professor Pim", avatar: "📚", rating: 1400,
    blurb: "Plays by the book. Quotes the book, too.",
    greeting: "Let us begin with a classical approach.",
    quips: { win: "As the textbooks predict.", lose: "Fascinating. A novelty.", draw: "A theoretical draw." },
    engine: { elo: 1400, movetime: 300, multipv: 3 },
    human: { temperature: 22, blunderRate: 0.02, style: "balanced", styleBonus: 0, mateSight: 0.95 },
  },
  {
    id: "tess", name: "Tactician Tess", avatar: "⚔️", rating: 1500,
    blurb: "Always attacking. Captures and checks are her favourites.",
    greeting: "Prepare to be attacked.",
    quips: { win: "Tactics wins again.", lose: "A fine counterattack!", draw: "A draw. I'll find the tactic next time." },
    engine: { depth: 6, multipv: 5 },
    human: { temperature: 25, blunderRate: 0.02, style: "aggressive", styleBonus: 45, mateSight: 1 },
  },
  {
    id: "kai", name: "Counter Kai", avatar: "🥋", rating: 1600,
    blurb: "Waits for you to overreach, then strikes back.",
    greeting: "Show me your best attack.",
    quips: { win: "Your attack ran out of steam.", lose: "You outplayed my counter.", draw: "Solid defence. Draw." },
    engine: { elo: 1550, movetime: 350, multipv: 3 },
    human: { temperature: 12, blunderRate: 0, style: "solid", styleBonus: 30, mateSight: 1 },
  },
  {
    id: "sol", name: "Sacrifice Sol", avatar: "🔥", rating: 1700,
    blurb: "Loves giving up material for a ferocious attack.",
    greeting: "Material is temporary. Attack is forever.",
    quips: { win: "The sacrifice was correct!", lose: "The sacrifice was... incorrect.", draw: "A draw, but a beautiful one." },
    engine: { elo: 1650, movetime: 400, multipv: 4 },
    human: { temperature: 20, blunderRate: 0, style: "aggressive", styleBonus: 55, mateSight: 1 },
  },
  {
    id: "finn", name: "Fortress Finn", avatar: "🛡️", rating: 1800,
    blurb: "Patient and solid. Hard to crack, happy to wait.",
    greeting: "No rush. I can wait all day.",
    quips: { win: "The fortress held.", lose: "You broke through. Impressive.", draw: "A fortress. Draw." },
    engine: { elo: 1750, movetime: 500, multipv: 3 },
    human: { temperature: 8, blunderRate: 0, style: "solid", styleBonus: 25, mateSight: 1 },
  },
  {
    id: "edda", name: "Endgame Edda", avatar: "🏔️", rating: 1900,
    blurb: "Trades down to an endgame, then grinds you out.",
    greeting: "Let's simplify things.",
    quips: { win: "Endgames are my home.", lose: "You beat me at my own game.", draw: "A drawn endgame, naturally." },
    engine: { elo: 1850, movetime: 550, multipv: 2 },
    human: { temperature: 6, blunderRate: 0, style: "solid", styleBonus: 25, mateSight: 1 },
  },
  {
    id: "quinn", name: "Queen Quinn", avatar: "👑", rating: 2000,
    blurb: "Brings the queen out early and makes it count.",
    greeting: "The queen goes where she pleases.",
    quips: { win: "Long live the queen.", lose: "Dethroned. Well played.", draw: "A draw. The queen is content." },
    engine: { elo: 1950, movetime: 600, multipv: 2 },
    human: { temperature: 6, blunderRate: 0, style: "aggressive", styleBonus: 35, mateSight: 1 },
  },
  {
    id: "sven", name: "Silent Sven", avatar: "🧊", rating: 2100,
    blurb: "Says nothing. Plays accurately. Wins quietly.",
    greeting: "...",
    quips: { win: "...", lose: "...", draw: "..." },
    engine: { elo: 2100, movetime: 700, multipv: 1 },
    human: { temperature: 0, blunderRate: 0, style: "balanced", styleBonus: 0, mateSight: 1 },
  },
  {
    id: "ines", name: "Iron Ines", avatar: "🦾", rating: 2200,
    blurb: "Positional pressure that never lets up.",
    greeting: "Prepare to be squeezed.",
    quips: { win: "Slowly, then all at once.", lose: "I didn't see that coming.", draw: "A draw. I'll squeeze harder next time." },
    engine: { elo: 2200, movetime: 750, multipv: 1 },
    human: { temperature: 0, blunderRate: 0, style: "solid", styleBonus: 15, mateSight: 1 },
  },
  {
    id: "max", name: "Master Max", avatar: "🎓", rating: 2300,
    blurb: "A seasoned tournament player. Few mistakes.",
    greeting: "Let's see what you've got.",
    quips: { win: "A good game. Study the endgame.", lose: "Well played. Genuinely.", draw: "A fair result." },
    engine: { elo: 2300, movetime: 800, multipv: 1 },
    human: { temperature: 0, blunderRate: 0, style: "balanced", styleBonus: 0, mateSight: 1 },
  },
  {
    id: "greta", name: "Grandmaster Greta", avatar: "🏆", rating: 2450,
    blurb: "Titled player. Converts small advantages without fuss.",
    greeting: "Good luck. You'll need it.",
    quips: { win: "Thank you for the game.", lose: "That's rare. Well done.", draw: "A draw against the grandmaster. Not bad." },
    engine: { elo: 2450, movetime: 900, multipv: 1 },
    human: { temperature: 0, blunderRate: 0, style: "balanced", styleBonus: 0, mateSight: 1 },
  },
  {
    id: "ravi", name: "Super GM Ravi", avatar: "🌟", rating: 2650,
    blurb: "Elite level. Deep preparation, almost no errors.",
    greeting: "Let's play some chess.",
    quips: { win: "Accurate play wins.", lose: "Remarkable. Truly.", draw: "A draw. Impressive resistance." },
    engine: { elo: 2650, movetime: 1000, multipv: 1 },
    human: { temperature: 0, blunderRate: 0, style: "balanced", styleBonus: 0, mateSight: 1 },
  },
  {
    id: "tori", name: "Titan Tori", avatar: "🗿", rating: 2800,
    blurb: "Barely human. Barely loses.",
    greeting: "...",
    quips: { win: "Expected.", lose: "Impossible. Screenshot this.", draw: "A draw?! That's a story for school." },
    engine: { elo: 2850, movetime: 1100, multipv: 1 },
    human: { temperature: 0, blunderRate: 0, style: "balanced", styleBonus: 0, mateSight: 1 },
  },
  {
    id: "stockfish", name: "Stockfish", avatar: "🤖", rating: 3000,
    blurb: "The full engine, no mercy. Don't expect to win.",
    greeting: "…",
    quips: { win: "…", lose: "…", draw: "…" },
    engine: { movetime: 1200, multipv: 1 },
    human: { temperature: 0, blunderRate: 0, style: "balanced", styleBonus: 0, mateSight: 1 },
  },
  {
    id: "ruedenrache", name: "Der Mann (Rüdenrache)", avatar: "🐺", avatarImage: "/bots/ruedenrache.png", rating: 4000, special: true,
    blurb: "Der Mann came back for revenge. Full engine power and no mercy at all.",
    greeting: "Der Mann ist zurück. Diesmal ist es persönlich.",
    quips: { win: "Rüdenrache. Süß.", lose: "Unmöglich. Mach einen Screenshot (Taste S).", draw: "Remis. Der Mann kommt wieder." },
    engine: { movetime: 2500, multipv: 1 },
    human: { temperature: 0, blunderRate: 0, style: "balanced", styleBonus: 0, mateSight: 1 },
  },
];

/** Groups for the opponent picker (min inclusive, max exclusive). */
export const BOT_TIERS = [
  { label: "Beginners", min: 0, max: 1000 },
  { label: "Casual", min: 1000, max: 1500 },
  { label: "Club players", min: 1500, max: 2000 },
  { label: "Experts", min: 2000, max: 2500 },
  { label: "Masters and engines", min: 2500, max: 10000 },
] as const;

export const BOT_IDS = BOTS.map((b) => b.id);
/**
 * The bots currently on offer: the built-in roster plus whatever admins added or changed. Set by `setBotRegistry` once the
 * custom bots have loaded; until then (and in tests) it is just the built-in roster.
 */
let registry: Bot[] | null = null;
export const setBotRegistry = (list: Bot[] | null) => {
  registry = list;
};
export const getBot = (id: string): Bot | undefined => (registry ?? BOTS).find((b) => b.id === id) ?? BOTS.find((b) => b.id === id);

// ---------------------------------------------------------------------------------------------- smartness 0-24

export const SMARTNESS_MAX = 24;
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
const clamp01 = (x: number) => Math.max(0, Math.min(1, x));

/**
 * One slider for playing strength. 0 plays almost random moves and never sees a mate; 24 is the full engine.
 * 1-5 beginner, 6-10 casual, 11-13 club, 14-24 real engine at an Elo from about 1350 up to 3050 (UCI_Elo).
 */
export function smartnessToPlay(level: number): Pick<Bot, "engine" | "human"> {
  const s = Math.max(0, Math.min(SMARTNESS_MAX, Math.round(level)));
  const base = { style: "balanced" as Style, styleBonus: 0 };
  if (s === 0) return { engine: { depth: 1, multipv: 10 }, human: { ...base, temperature: 5000, blunderRate: 0.9, mateSight: 0 } };
  if (s <= 5) {
    const t = (s - 1) / 4;
    return { engine: { depth: 1, multipv: Math.round(lerp(10, 8, t)) }, human: { ...base, temperature: Math.round(lerp(300, 150, t)), blunderRate: +lerp(0.25, 0.15, t).toFixed(2), mateSight: +lerp(0.3, 0.6, t).toFixed(2) } };
  }
  if (s <= 10) {
    const t = (s - 6) / 4;
    return { engine: { depth: Math.round(lerp(2, 4, t)), multipv: 6 }, human: { ...base, temperature: Math.round(lerp(150, 60, t)), blunderRate: +lerp(0.15, 0.06, t).toFixed(2), mateSight: +lerp(0.6, 0.9, t).toFixed(2) } };
  }
  if (s <= 13) {
    const t = (s - 11) / 2;
    return { engine: { depth: Math.round(lerp(5, 8, t)), multipv: 4 }, human: { ...base, temperature: Math.round(lerp(40, 12, t)), blunderRate: +lerp(0.06, 0.02, t).toFixed(2), mateSight: +lerp(0.9, 1, t).toFixed(2) } };
  }
  const t = clamp01((s - 14) / 10);
  return {
    engine: { elo: Math.round(lerp(1350, 3050, t)), movetime: Math.round(lerp(300, 1300, t)), multipv: s < 17 ? 3 : s < 20 ? 2 : 1 },
    human: { ...base, temperature: Math.round(lerp(10, 0, t)), blunderRate: 0, mateSight: 1 },
  };
}

/** A sensible "rating" label for a smartness level (the label is only shown, strength comes from smartness). */
export const suggestRating = (level: number): number => {
  const pts: [number, number][] = [[0, 100], [5, 600], [10, 1000], [14, 1400], [19, 2200], [24, 3000]];
  const s = Math.max(0, Math.min(SMARTNESS_MAX, level));
  for (let i = 1; i < pts.length; i++) if (s <= pts[i][0]) return Math.round(lerp(pts[i - 1][1], pts[i][1], (s - pts[i - 1][0]) / (pts[i][0] - pts[i - 1][0])) / 10) * 10;
  return 3000;
};

// ---------------------------------------------------------------------------------------------- admin-made bots

/** A row of public.custom_bots. */
export type CustomBotRow = {
  id: string; name: string; avatar: string; avatar_image: string | null; rating: number; blurb: string; greeting: string;
  quip_win: string; quip_lose: string; quip_draw: string; smartness: number | null; style: Style | null; style_bonus: number | null;
  blunder_pct: number | null; mate_sight_pct: number | null; thinking_ms: number | null; special: boolean; hidden: boolean;
};

/** Build a bot from an admin's row, on top of the built-in bot with the same id (if any). */
export function botFromRow(row: CustomBotRow, base?: Bot): Bot | null {
  let play: Pick<Bot, "engine" | "human"> | null = null;
  if (row.smartness !== null) play = smartnessToPlay(row.smartness);
  else if (base) play = { engine: { ...base.engine }, human: { ...base.human } };
  if (!play) return null; // a brand-new bot needs a smartness level
  const human = { ...play.human };
  if (row.style) human.style = row.style;
  if (row.style_bonus !== null) human.styleBonus = row.style_bonus;
  if (row.blunder_pct !== null) human.blunderRate = row.blunder_pct / 100;
  if (row.mate_sight_pct !== null) human.mateSight = row.mate_sight_pct / 100;
  const engine = { ...play.engine };
  if (row.thinking_ms !== null) engine.movetime = row.thinking_ms; // a depth-limited bot also respects the time cap
  return {
    id: row.id, name: row.name, avatar: row.avatar, avatarImage: row.avatar_image ?? base?.avatarImage, rating: row.rating,
    blurb: row.blurb, greeting: row.greeting, quips: { win: row.quip_win, lose: row.quip_lose, draw: row.quip_draw },
    special: row.special || undefined, engine, human,
  };
}

/** Built-in roster with admin overrides and additions applied, hidden bots removed, strongest last. */
export function mergeBots(builtin: Bot[], rows: CustomBotRow[]): Bot[] {
  const byId = new Map(rows.map((r) => [r.id, r]));
  const out: Bot[] = [];
  for (const b of builtin) {
    const row = byId.get(b.id);
    if (!row) out.push(b);
    else if (!row.hidden) out.push(botFromRow(row, b) ?? b);
  }
  const known = new Set(builtin.map((b) => b.id));
  for (const r of rows) {
    if (known.has(r.id) || r.hidden) continue;
    const bot = botFromRow(r);
    if (bot) out.push(bot);
  }
  return out.sort((a, b) => a.rating - b.rating);
}

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
      if (style === "greedy" && f.capture) bonus = styleBonus;
      if (style === "checker" && (f.check || f.promotion)) bonus = styleBonus;
    }
    if (style === "chaotic") bonus = (rng() * 2 - 1) * styleBonus;
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
