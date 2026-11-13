import { describe, expect, it } from "vitest";
import { BOTS, BOT_TIERS, botFromRow, mergeBots, setBotRegistry, smartnessToPlay, suggestRating, type CustomBotRow, candidatesFor, chooseMove, getBot, type Bot, type MoveFeatures } from "./bots";
import { parseInfo, type Line } from "./uci";

const line = (uci: string, cp: number, multipv: number): Line => ({ uci, cp, mate: null, depth: 10, multipv, pv: [uci] });

/** Deterministic pseudo-random generator so statistical tests never flake. */
function lcg(seed = 1) {
  let s = seed >>> 0;
  return () => {
    s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
    return s / 2 ** 32;
  };
}

const bot = (human: Partial<Bot["human"]>): Bot => ({ ...getBot("max")!, human: { ...getBot("max")!.human, ...human } });
const lines = [line("e2e4", 50, 1), line("d2d4", 45, 2), line("g1f3", 30, 3), line("a2a3", -150, 4)];
const none = new Map<string, MoveFeatures>();

describe("parseInfo", () => {
  it("reads centipawn lines", () => {
    const l = parseInfo("info depth 8 seldepth 14 multipv 2 score cp 19 nodes 17320 nps 481111 hashfull 5 time 36 pv b2b4 b7b6 c2c4");
    expect(l).toMatchObject({ uci: "b2b4", cp: 19, mate: null, depth: 8, multipv: 2 });
    expect(l?.pv).toEqual(["b2b4", "b7b6", "c2c4"]);
  });
  it("turns mate scores into huge centipawn values with the right sign", () => {
    expect(parseInfo("info depth 5 multipv 1 score mate 2 nodes 1 pv a1a8")?.cp).toBeGreaterThan(99_000);
    expect(parseInfo("info depth 5 multipv 1 score mate -3 nodes 1 pv a1a8")?.cp).toBeLessThan(-99_000);
    expect(parseInfo("info depth 5 multipv 1 score mate 1 nodes 1 pv a1a8")?.mate).toBe(1);
  });
  it("accepts bound markers and ignores lines without a score", () => {
    expect(parseInfo("info depth 3 multipv 1 score cp 12 lowerbound nodes 9 pv e2e4")?.cp).toBe(12);
    expect(parseInfo("info string NNUE evaluation using nn.bin")).toBeNull();
    expect(parseInfo("info depth 4 currmove e2e4 currmovenumber 1")).toBeNull();
  });
});

describe("chooseMove", () => {
  it("plays the engine's best move at temperature 0", () => {
    expect(chooseMove(lines, bot({ temperature: 0 }), none, lcg(), { ply: 40 })).toBe("e2e4");
  });
  it("returns null with no candidates and the only move when there is one", () => {
    expect(chooseMove([], bot({}), none)).toBeNull();
    expect(chooseMove([line("a1a2", 0, 1)], bot({}), none)).toBe("a1a2");
  });
  it("a weak bot varies its play but a bad move stays rarer than good ones", () => {
    const weak = bot({ temperature: 90, blunderRate: 0 });
    const rng = lcg(7);
    const counts: Record<string, number> = {};
    for (let i = 0; i < 4000; i++) {
      const m = chooseMove(lines, weak, none, rng, { ply: 40 })!;
      counts[m] = (counts[m] ?? 0) + 1;
    }
    expect(Object.keys(counts).length).toBeGreaterThan(2);
    expect(counts["e2e4"]).toBeGreaterThan(counts["g1f3"]);
    expect(counts["g1f3"]).toBeGreaterThan(counts["a2a3"] ?? 0);
  });
  it("blunder rate picks outside the top two at about the configured frequency", () => {
    const b = bot({ temperature: 0, blunderRate: 0.3 });
    const rng = lcg(3);
    let off = 0;
    const n = 5000;
    for (let i = 0; i < n; i++) if (!["e2e4", "d2d4"].includes(chooseMove(lines, b, none, rng, { ply: 40 })!)) off++;
    expect(off / n).toBeGreaterThan(0.25);
    expect(off / n).toBeLessThan(0.35);
  });
  it("never blunders with only two candidates to choose from", () => {
    const b = bot({ temperature: 0, blunderRate: 1 });
    expect(chooseMove(lines.slice(0, 2), b, none, lcg(), { ply: 40 })).toBe("e2e4");
  });
  it("aggressive bots prefer a near-equal capture; solid bots prefer a near-equal quiet move", () => {
    const feats = new Map<string, MoveFeatures>([
      ["e2e4", { capture: false, check: false, promotion: false }],
      ["d2d4", { capture: true, check: false, promotion: false }],
    ]);
    const two = lines.slice(0, 2);
    expect(chooseMove(two, bot({ temperature: 0, style: "aggressive", styleBonus: 45 }), feats, lcg(), { ply: 40 })).toBe("d2d4");
    expect(chooseMove(two, bot({ temperature: 0, style: "solid", styleBonus: 45 }), feats, lcg(), { ply: 40 })).toBe("e2e4");
  });
  it("style bonuses do not override a clearly better move", () => {
    const feats = new Map<string, MoveFeatures>([["d2d4", { capture: true, check: false, promotion: false }]]);
    const l = [line("e2e4", 200, 1), line("d2d4", 20, 2)];
    expect(chooseMove(l, bot({ temperature: 0, style: "aggressive", styleBonus: 45 }), feats, lcg(), { ply: 40 })).toBe("e2e4");
  });
  it("adds opening variety even to deterministic bots", () => {
    const strong = bot({ temperature: 0 });
    const rng = lcg(11);
    const seen = new Set<string>();
    for (let i = 0; i < 300; i++) seen.add(chooseMove(lines, strong, none, rng, { ply: 0 })!);
    expect(seen.size).toBeGreaterThan(1);
    const late = new Set<string>();
    for (let i = 0; i < 300; i++) late.add(chooseMove(lines, strong, none, rng, { ply: 30 })!);
    expect(late.size).toBe(1);
  });
  it("beginners often miss a mate in one; strong bots almost never do", () => {
    const l = [{ ...line("a1a8", 99_999, 1), mate: 1 }, line("h2h3", 0, 2)];
    const rate = (mateSight: number, seed: number) => {
      const b = bot({ temperature: 0, blunderRate: 0, mateSight });
      const rng = lcg(seed);
      let found = 0;
      for (let i = 0; i < 3000; i++) if (chooseMove(l, b, none, rng, { ply: 40 }) === "a1a8") found++;
      return found / 3000;
    };
    expect(rate(1, 5)).toBe(1);
    expect(rate(0.35, 6)).toBeGreaterThan(0.55);
    expect(rate(0.35, 6)).toBeLessThan(0.8);
    expect(rate(0, 7)).toBeGreaterThan(0.4);
    expect(rate(0, 7)).toBeLessThan(0.6);
  });
});

describe("bot roster", () => {
  it("has unique ids, rising ratings and sane engine settings", () => {
    expect(new Set(BOTS.map((b) => b.id)).size).toBe(BOTS.length);
    for (let i = 1; i < BOTS.length; i++) expect(BOTS[i].rating).toBeGreaterThan(BOTS[i - 1].rating);
    for (let i = 1; i < BOTS.length; i++) expect(BOTS[i].human.mateSight).toBeGreaterThanOrEqual(BOTS[i - 1].human.mateSight);
    for (const b of BOTS) {
      expect(b.engine.multipv).toBeGreaterThanOrEqual(1);
      if (b.engine.elo !== undefined) expect(b.engine.elo).toBeGreaterThanOrEqual(1320);
      expect(b.engine.depth !== undefined || b.engine.movetime !== undefined).toBe(true);
    }
  });
  it("has many bots, each with a personality, and the special Pong Alf Femboy at 650", () => {
    expect(BOTS.length).toBeGreaterThanOrEqual(25);
    const alf = getBot("alf")!;
    expect(alf.name).toBe("Pong Alf Femboy");
    expect(alf.rating).toBe(650);
    expect(alf.special).toBe(true);
    for (const b of BOTS) {
      expect(b.quips.win.length).toBeGreaterThan(0);
      expect(b.quips.lose.length).toBeGreaterThan(0);
      expect(b.quips.draw.length).toBeGreaterThan(0);
    }
    for (const b of BOTS) expect(BOT_TIERS.filter((t) => b.rating >= t.min && b.rating < t.max)).toHaveLength(1);
  });
  it("has Der Mann at 51 (the weakest) and Der Mann (Rüdenrache) at 4000 (the strongest)", () => {
    expect(BOTS[0].id).toBe("dermann");
    expect(BOTS[0].rating).toBe(51);
    expect(BOTS[0].avatarImage).toBe("/bots/dermann.png");
    expect(BOTS[BOTS.length - 1].id).toBe("ruedenrache");
    expect(BOTS[BOTS.length - 1].rating).toBe(4000);
    expect(BOTS[BOTS.length - 1].name).toBe("Der Mann (Rüdenrache)");
  });
  it("new styles change the choice: greedy takes captures, checker gives checks", () => {
    const two = [line("e2e4", 0, 1), line("d2d4", 0, 2)];
    const feats = new Map<string, MoveFeatures>([["e2e4", { capture: false, check: false, promotion: false }], ["d2d4", { capture: true, check: false, promotion: false }]]);
    expect(chooseMove(two, bot({ temperature: 0, style: "greedy", styleBonus: 45 }), feats, lcg(), { ply: 40 })).toBe("d2d4");
    const feats2 = new Map<string, MoveFeatures>([["e2e4", { capture: false, check: true, promotion: false }], ["d2d4", { capture: true, check: false, promotion: false }]]);
    expect(chooseMove(two, bot({ temperature: 0, style: "checker", styleBonus: 45 }), feats2, lcg(), { ply: 40 })).toBe("e2e4");
  });
  it("asks for more candidate lines early for single-line bots", () => {
    expect(candidatesFor(getBot("max")!, 0)).toBe(3);
    expect(candidatesFor(getBot("max")!, 20)).toBe(1);
    expect(candidatesFor(getBot("pawnie")!, 0)).toBe(10);
  });
});

const row = (over: Partial<CustomBotRow>): CustomBotRow => ({
  id: "newbie", name: "Newbie", avatar: "🤖", avatar_image: null, rating: 1000, blurb: "b", greeting: "g", quip_win: "w", quip_lose: "l", quip_draw: "d",
  smartness: 10, style: null, style_bonus: null, blunder_pct: null, mate_sight_pct: null, thinking_ms: null, special: false, hidden: false, ...over,
});

describe("smartness 0-24", () => {
  it("gets stronger with every step: lower noise, deeper search, higher engine Elo", () => {
    let prev = smartnessToPlay(0);
    for (let s = 1; s <= 24; s++) {
      const cur = smartnessToPlay(s);
      expect(cur.human.temperature).toBeLessThanOrEqual(prev.human.temperature);
      expect(cur.human.blunderRate).toBeLessThanOrEqual(prev.human.blunderRate);
      expect(cur.human.mateSight).toBeGreaterThanOrEqual(prev.human.mateSight);
      if (cur.engine.elo !== undefined) expect(cur.engine.elo).toBeGreaterThanOrEqual(prev.engine.elo ?? 0);
      prev = cur;
    }
  });
  it("0 is nearly random and never sees mate; 24 is the full engine", () => {
    const dumb = smartnessToPlay(0);
    expect(dumb.human.blunderRate).toBeGreaterThanOrEqual(0.9);
    expect(dumb.human.mateSight).toBe(0);
    const top = smartnessToPlay(24);
    expect(top.engine.elo).toBeGreaterThanOrEqual(3000);
    expect(top.human.blunderRate).toBe(0);
  });
  it("engine Elo stays within what Stockfish accepts and always has a limit to search by", () => {
    for (let s = 0; s <= 24; s++) {
      const p = smartnessToPlay(s);
      if (p.engine.elo !== undefined) expect(p.engine.elo).toBeGreaterThanOrEqual(1320);
      expect(p.engine.elo !== undefined ? p.engine.movetime !== undefined : p.engine.depth !== undefined).toBe(true);
    }
  });
  it("clamps out-of-range levels and suggests a rating that rises with smartness", () => {
    expect(smartnessToPlay(-5)).toEqual(smartnessToPlay(0));
    expect(smartnessToPlay(99)).toEqual(smartnessToPlay(24));
    for (let s = 1; s <= 24; s++) expect(suggestRating(s)).toBeGreaterThanOrEqual(suggestRating(s - 1));
  });
});

describe("admin-made bots", () => {
  it("builds a new bot from a row, with per-habit overrides on top of smartness", () => {
    const bot = botFromRow(row({ smartness: 12, style: "greedy", style_bonus: 99, blunder_pct: 20, mate_sight_pct: 40, thinking_ms: 700 }))!;
    expect(bot.human.style).toBe("greedy");
    expect(bot.human.styleBonus).toBe(99);
    expect(bot.human.blunderRate).toBeCloseTo(0.2);
    expect(bot.human.mateSight).toBeCloseTo(0.4);
    expect(bot.engine.movetime).toBe(700);
    expect(bot.quips).toEqual({ win: "w", lose: "l", draw: "d" });
  });
  it("a brand-new bot without smartness is rejected; an override without it keeps the original strength", () => {
    expect(botFromRow(row({ smartness: null }))).toBeNull();
    const cam = getBot("cam")!;
    const edited = botFromRow(row({ id: "cam", name: "Cam 2", smartness: null, rating: 1111 }), cam)!;
    expect(edited.name).toBe("Cam 2");
    expect(edited.rating).toBe(1111);
    expect(edited.engine).toEqual(cam.engine);
    expect(edited.human).toEqual(cam.human);
  });
  it("merges: overrides replace, additions appear, hidden bots vanish, sorted by rating", () => {
    const merged = mergeBots(BOTS, [row({ id: "cam", name: "Changed Cam", smartness: null, rating: 5 }), row({ id: "pawnie", hidden: true, smartness: null }), row({ id: "zed", name: "Zed", rating: 2750 }), row({ id: "ghost", smartness: 3, hidden: true })]);
    expect(merged.find((b) => b.id === "cam")!.name).toBe("Changed Cam");
    expect(merged.some((b) => b.id === "pawnie")).toBe(false);
    expect(merged.some((b) => b.id === "ghost")).toBe(false);
    expect(merged.some((b) => b.id === "zed")).toBe(true);
    expect(merged.length).toBe(BOTS.length);
    for (let i = 1; i < merged.length; i++) expect(merged[i].rating).toBeGreaterThanOrEqual(merged[i - 1].rating);
  });
  it("getBot finds admin-made bots once the registry is set, and falls back to built-ins", () => {
    setBotRegistry(mergeBots(BOTS, [row({ id: "zed", name: "Zed", rating: 2750 })]));
    expect(getBot("zed")!.name).toBe("Zed");
    expect(getBot("max")!.name).toBe("Master Max");
    setBotRegistry(null);
    expect(getBot("zed")).toBeUndefined();
  });
});
