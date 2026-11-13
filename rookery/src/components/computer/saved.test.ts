import { describe, expect, it } from "vitest";
import { getBot } from "@/lib/engine/bots";
import { newRecord, tryUci } from "@/lib/chess/game";
import { TIME_CONTROLS } from "./clock";
import type { Game } from "./ComputerGame";
import { restore, snapshot } from "./saved";

const game = (moves: string[], over: Partial<Game> = {}): Game => {
  let record = newRecord("standard");
  for (const m of moves) {
    const r = tryUci(record, m);
    if (!r.ok) throw new Error("bad test move " + m);
    record = r.record;
  }
  return {
    id: "g1", watchable: true, bot: getBot("alf")!, player: "white", record, tc: TIME_CONTROLS.find((t) => t.id === "5+0")!, takebacks: true,
    clocks: { white: 300_000, black: 300_000 }, startedAt: 1_000_000, end: null, botEval: null, startDate: "2026.11.13", ...over,
  };
};

describe("saving a bot game", () => {
  it("round-trips moves, bot, side and settings", () => {
    const g = game(["e2e4", "e7e5", "g1f3"]);
    const back = restore(snapshot(g, 1_000_500), 9_000_000)!;
    expect(back.record.moves).toEqual(["e2e4", "e7e5", "g1f3"]);
    expect(back.bot.id).toBe("alf");
    expect(back.player).toBe("white");
    expect(back.tc.id).toBe("5+0");
    expect(back.id).toBe("g1");
  });
  it("folds the running clock into the save, then pauses it while you are away", () => {
    // 4 plies: both have moved, so the side to move (white) is on the clock since startedAt.
    const g = game(["e2e4", "e7e5", "g1f3", "b8c6"], { clocks: { white: 250_000, black: 240_000 }, startedAt: 1_000_000 });
    const snap = snapshot(g, 1_030_000); // 30 s of thinking
    expect(snap.clocks.white).toBe(220_000);
    expect(snap.clocks.black).toBe(240_000);
    const back = restore(snap, 50_000_000)!; // days later: nothing is lost
    expect(back.clocks.white).toBe(220_000);
    expect(back.startedAt).toBe(50_000_000);
  });
  it("does not run clocks before both sides have moved", () => {
    const g = game(["e2e4"], { startedAt: 1_000_000 });
    expect(snapshot(g, 1_090_000).clocks).toEqual({ white: 300_000, black: 300_000 });
  });
  it("refuses a corrupt save", () => {
    const snap = snapshot(game(["e2e4"]), 1);
    expect(restore({ ...snap, moves: ["e2e5"] }, 2)).toBeNull();
    expect(restore({ ...snap, botId: "nobody" }, 2)).toBeNull();
    expect(restore({ ...snap, tcId: "99+99" }, 2)).toBeNull();
  });
});
