import { describe, expect, it } from "vitest";
import { LEVELS, TOP_LEVEL, colorForLevel, hueFor, leaderboard, rudeBot } from "./rude";

describe("the ladder", () => {
  it("has 20 Rüden rated 100 to 2000 in steps of 100", () => {
    expect(LEVELS).toHaveLength(20);
    expect(LEVELS[0]).toBe(100);
    expect(LEVELS.at(-1)).toBe(TOP_LEVEL);
    for (let i = 1; i < LEVELS.length; i++) expect(LEVELS[i] - LEVELS[i - 1]).toBe(100);
  });
  it("every Rüde is a valid special bot with a unique id and its own rating label", () => {
    const bots = LEVELS.map(rudeBot);
    expect(new Set(bots.map((b) => b.id)).size).toBe(20);
    for (const b of bots) {
      expect(b.id).toBe(`rude-${b.rating}`);
      expect(b.name).toBe(`Rüde ${b.rating}`);
      expect(b.special).toBe(true);
      expect(b.engine.multipv).toBeGreaterThanOrEqual(1);
      if (b.engine.elo !== undefined) expect(b.engine.elo).toBeGreaterThanOrEqual(1320);
      expect(b.engine.depth !== undefined || b.engine.movetime !== undefined).toBe(true);
      expect(b.human.mateSight).toBeGreaterThanOrEqual(0);
      expect(b.human.mateSight).toBeLessThanOrEqual(1);
    }
    expect(() => rudeBot(150)).toThrow();
  });
  it("gets steadily harder: no Rüde plays like the one before it", () => {
    const bots = LEVELS.map(rudeBot);
    const key = (b: ReturnType<typeof rudeBot>) => JSON.stringify([b.engine, b.human.temperature, b.human.blunderRate, b.human.mateSight]);
    for (let i = 1; i < bots.length; i++) {
      expect(key(bots[i]), `Rüde ${bots[i].rating}`).not.toBe(key(bots[i - 1]));
      const a = bots[i - 1], b = bots[i];
      expect(b.human.temperature).toBeLessThanOrEqual(a.human.temperature);
      expect(b.human.blunderRate).toBeLessThanOrEqual(a.human.blunderRate);
      expect(b.human.mateSight).toBeGreaterThanOrEqual(a.human.mateSight);
      if (a.engine.elo !== undefined && b.engine.elo !== undefined) expect(b.engine.elo).toBeGreaterThan(a.engine.elo);
    }
    expect(bots[0].human.blunderRate).toBeGreaterThan(0.5); // Rüde 100 is almost random
    expect(bots.at(-1)!.engine.elo).toBeGreaterThanOrEqual(2000);
  });
  it("alternates colours: White against the odd Rüden, Black against the even ones", () => {
    expect(LEVELS.map(colorForLevel).slice(0, 4)).toEqual(["white", "black", "white", "black"]);
  });
  it("gives each Rüde a different colour", () => {
    expect(new Set(LEVELS.map(hueFor)).size).toBe(20);
  });
});

describe("the leaderboard", () => {
  const row = (user_id: string, username: string, best_level: number, updated_at: string, cleared = false) => ({ user_id, username, best_level, cleared, updated_at });
  it("shows each player once with their best attempt and the number of attempts used", () => {
    const board = leaderboard([row("a", "Ann", 800, "2026-12-01T10:00"), row("a", "Ann", 300, "2026-12-02T10:00"), row("b", "Ben", 1500, "2026-12-01T09:00"), row("c", "Cy", 0, "2026-12-01T08:00")]);
    expect(board.map((e) => [e.username, e.best, e.attempts])).toEqual([["Ben", 1500, 1], ["Ann", 800, 2], ["Cy", 0, 1]]);
  });
  it("puts anyone who cleared the whole ladder first, then ties go to fewer attempts, then who got there first", () => {
    const board = leaderboard([
      row("a", "Ann", 2000, "2026-12-03T10:00", true), row("b", "Ben", 1900, "2026-12-01T10:00"), row("b", "Ben", 1900, "2026-12-02T10:00"),
      row("c", "Cy", 1900, "2026-12-01T08:00"), row("d", "Di", 1900, "2026-12-04T08:00"),
    ]);
    expect(board.map((e) => e.username)).toEqual(["Ann", "Cy", "Di", "Ben"]);
    expect(board[0].cleared).toBe(true);
  });
});
