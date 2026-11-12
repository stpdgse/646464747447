import { describe, expect, it } from "vitest";
import { BOTS, candidatesFor, chooseMove, getBot, type Bot, type MoveFeatures } from "./bots";
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
  it("asks for more candidate lines early for single-line bots", () => {
    expect(candidatesFor(getBot("max")!, 0)).toBe(3);
    expect(candidatesFor(getBot("max")!, 20)).toBe(1);
    expect(candidatesFor(getBot("pawnie")!, 0)).toBe(10);
  });
});
