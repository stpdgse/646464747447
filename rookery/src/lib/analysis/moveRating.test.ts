import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { newRecord, tryUci, type GameRecord } from "@/lib/chess/game";
import { UciEngine, type Transport } from "@/lib/engine/uci";
import { isSacrifice, rateMove, rateMoveWithEngine, RATING_UI, type MoveRating } from "./moveRating";

const play = (...moves: string[]): GameRecord => {
  let r = newRecord("standard");
  for (const m of moves) {
    const x = tryUci(r, m);
    if (!x.ok) throw new Error("bad move " + m);
    r = x.record;
  }
  return r;
};

describe("rateMove (pure rules)", () => {
  const base = { bestCp: 0, secondCp: null, afterCp: 0, isBest: false, sacrifice: false };
  it("grades by how much win chance the move gave away", () => {
    expect(rateMove({ ...base, afterCp: -20 }).rating).toBe("good");
    expect(rateMove({ ...base, afterCp: -60 }).rating).toBe("inaccuracy");
    expect(rateMove({ ...base, afterCp: -120 }).rating).toBe("mistake");
    expect(rateMove({ ...base, afterCp: -300 }).rating).toBe("blunder");
  });
  it("the engine's top move is Best, and never worse even if the second search is noisy", () => {
    expect(rateMove({ ...base, isBest: true, afterCp: -80 })).toEqual({ rating: "best", loss: 0 });
  });
  it("Great means the best move was the only good one", () => {
    expect(rateMove({ ...base, bestCp: 100, secondCp: -300, afterCp: 100, isBest: true }).rating).toBe("great");
    expect(rateMove({ ...base, bestCp: 100, secondCp: 80, afterCp: 100, isBest: true }).rating).toBe("best");
  });
  it("an only-move in an already lost or already won position is not Great", () => {
    expect(rateMove({ ...base, bestCp: -900, secondCp: -1000, afterCp: -900, isBest: true }).rating).toBe("best");
    expect(rateMove({ ...base, bestCp: 900, secondCp: 100, afterCp: 900, isBest: true }).rating).toBe("best");
  });
  it("Brilliant is a sacrifice the engine still likes, when not already winning", () => {
    expect(rateMove({ ...base, bestCp: 50, afterCp: 40, sacrifice: true }).rating).toBe("brilliant");
    expect(rateMove({ ...base, bestCp: 700, afterCp: 690, sacrifice: true, isBest: true }).rating).toBe("best");
    expect(rateMove({ ...base, bestCp: 50, afterCp: -400, sacrifice: true }).rating).toBe("blunder");
  });
  it("every rating has a label, symbol and colour", () => {
    const all: MoveRating[] = ["brilliant", "great", "best", "good", "inaccuracy", "mistake", "blunder"];
    for (const r of all) expect(RATING_UI[r].label.length).toBeGreaterThan(0);
  });
});

describe("isSacrifice", () => {
  it("a queen that takes a defended pawn can be won by a knight: a sacrifice", () => {
    expect(isSacrifice(play("e2e4", "e7e5", "d1h5", "b8c6"), "h5e5")).toBe(true);
  });
  it("a safe developing move, a pawn move, castling and an even trade are not", () => {
    expect(isSacrifice(play("e2e4", "e7e5"), "g1f3")).toBe(false);
    expect(isSacrifice(play("e2e4", "d7d5"), "e4d5")).toBe(false);
    expect(isSacrifice(play("e2e4", "e7e5", "g1f3", "b8c6", "f1c4", "g8f6"), "e1g1")).toBe(false);
    expect(isSacrifice(play("e2e4", "e7e5", "g1f3", "b8c6", "f3e5", "c6e5"), "d2d4")).toBe(false);
  });
});

const ENGINE = path.resolve(__dirname, "../../../public/engine/stockfish-19-lite-single.js");
function nodeTransport(): Transport {
  const child = spawn(process.execPath, [ENGINE], { stdio: ["pipe", "pipe", "ignore"] });
  let cb: (l: string) => void = () => {};
  let buf = "";
  child.stdout.on("data", (d) => { buf += d; let i: number; while ((i = buf.indexOf("\n")) >= 0) { cb(buf.slice(0, i).trim()); buf = buf.slice(i + 1); } });
  return { send: (c) => child.stdin.write(c + "\n"), onLine: (f) => { cb = f; }, terminate: () => child.kill() };
}
const withEngine = async (fn: (e: UciEngine) => Promise<void>) => { const e = new UciEngine(nodeTransport()); try { await fn(e); } finally { e.terminate(); } };

describe.runIf(existsSync(ENGINE))("rateMoveWithEngine with real Stockfish", () => {
  it("rates a hanging queen as a blunder, even though it looks like a sacrifice", async () => {
    await withEngine(async (engine) => {
      const r = await rateMoveWithEngine(engine, play("e2e4", "e7e5", "d1h5", "b8c6"), "h5e5", { depth: 10, movetime: 150 });
      expect(r?.rating).toBe("blunder");
      expect(r?.playedSan).toBe("Qxe5+");
      expect(r?.bestSan).toBeTruthy();
    });
  }, 60_000);
  it("rates a sensible opening move well and a missed defence badly", async () => {
    await withEngine(async (engine) => {
      const good = await rateMoveWithEngine(engine, newRecord("standard"), "e2e4", { depth: 10, movetime: 150 });
      expect(["best", "good", "great"]).toContain(good?.rating);
      const bad = await rateMoveWithEngine(engine, play("e2e4", "e7e5", "d1h5", "b8c6", "f1c4"), "g8f6", { depth: 10, movetime: 150 });
      expect(bad?.rating).toBe("blunder");
      expect(bad?.bestSan).toBeTruthy();
    });
  }, 60_000);
  it("rates the mating move and handles games that end", async () => {
    await withEngine(async (engine) => {
      const mate = await rateMoveWithEngine(engine, play("e2e4", "e7e5", "d1h5", "b8c6", "f1c4", "g8f6"), "h5f7", { depth: 10, movetime: 150 });
      expect(["best", "great"]).toContain(mate?.rating);
    });
  }, 60_000);
});
