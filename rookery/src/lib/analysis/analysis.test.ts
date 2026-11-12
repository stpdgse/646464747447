import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { newRecord, tryUci, type GameRecord } from "@/lib/chess/game";
import { UciEngine, type Transport } from "@/lib/engine/uci";
import { analyseGame, buildReview, classify, clampCp, formatEval, gameAccuracy, moveAccuracy, winPercent, type EvalPoint } from "./analysis";

const play = (...u: string[]): GameRecord => u.reduce((r, m) => { const x = tryUci(r, m); if (!x.ok) throw new Error(`illegal ${m}`); return x.record; }, newRecord("standard"));

describe("win chance and accuracy maths (Lichess formulas)", () => {
  it("anchors", () => {
    expect(winPercent(0)).toBeCloseTo(50, 6);
    expect(winPercent(100)).toBeCloseTo(59.1, 1);
    expect(winPercent(-100)).toBeCloseTo(40.9, 1);
    expect(winPercent(1000)).toBeGreaterThan(97.5);
    expect(winPercent(100000)).toBe(winPercent(1000)); // mates are clamped
    expect(clampCp(5000)).toBe(1000);
    expect(winPercent(300) + winPercent(-300)).toBeCloseTo(100, 6);
  });
  it("move accuracy: no loss is ~100, bigger losses fall off smoothly", () => {
    expect(moveAccuracy(50, 50)).toBeCloseTo(100, 2);
    expect(moveAccuracy(50, 60)).toBe(100); // improving the position never exceeds 100
    expect(moveAccuracy(60, 50)).toBeCloseTo(63.6, 1);
    expect(moveAccuracy(90, 10)).toBeLessThan(5);
    let prev = 101;
    for (let loss = 0; loss <= 60; loss += 5) { const a = moveAccuracy(70, 70 - loss); expect(a).toBeLessThanOrEqual(prev); prev = a; }
  });
  it("classifies by the Lichess cut-offs", () => {
    expect(classify(0, true)).toBe("best");
    expect(classify(0.3, false)).toBe("best");
    expect(classify(2, false)).toBe("good");
    expect(classify(5, false)).toBe("inaccuracy");
    expect(classify(9.9, false)).toBe("inaccuracy");
    expect(classify(10, false)).toBe("mistake");
    expect(classify(15, false)).toBe("blunder");
    expect(classify(80, true)).toBe("blunder");
  });
  it("game accuracy: perfect play is ~100, one blunder drags it down, order of magnitude is sane", () => {
    const wins = Array.from({ length: 41 }, (_, i) => 50 + (i % 5));
    const perfect = Array.from({ length: 20 }, (_, i) => ({ accuracy: 100, index: i * 2 }));
    expect(gameAccuracy(perfect, wins)).toBeCloseTo(100, 4);
    const oneBlunder = [...perfect.slice(0, 19), { accuracy: 5, index: 38 }];
    const a = gameAccuracy(oneBlunder, wins)!;
    expect(a).toBeLessThan(97);
    expect(a).toBeGreaterThan(60);
    expect(gameAccuracy([], wins)).toBeNull();
    const meh = gameAccuracy(Array.from({ length: 20 }, (_, i) => ({ accuracy: 70, index: i * 2 })), wins)!;
    expect(meh).toBeCloseTo(70, 3);
  });
  it("formats evaluations", () => {
    expect(formatEval({ cp: 140, mate: null })).toBe("+1.4");
    expect(formatEval({ cp: -35, mate: null })).toBe("-0.3");
    expect(formatEval({ cp: 100000, mate: 3 })).toBe("M3");
    expect(formatEval({ cp: -100000, mate: -2 })).toBe("-M2");
    expect(formatEval({ cp: 100000, mate: 0 })).toBe("#");
  });
});

describe("buildReview (no engine)", () => {
  const rec = play("e2e4", "e7e5", "g1f3");
  const ev = (cp: number, best: string | null): EvalPoint => ({ cp, mate: null, best, searched: true });
  it("judges each move against the previous evaluation and credits matching the engine's move", () => {
    const evals = [ev(20, "e2e4"), ev(30, "e7e5"), ev(25, "g1f3"), ev(30, null)];
    const r = buildReview(rec, evals);
    expect(r.moves.map((m) => m.san)).toEqual(["e4", "e5", "Nf3"]);
    expect(r.moves.every((m) => m.judgement === "best")).toBe(true);
    expect(r.counts.white.best).toBe(2);
    expect(r.counts.black.best).toBe(1);
  });
  it("flags a swing as a blunder for the right side and names the better move", () => {
    const evals = [ev(20, "e2e4"), ev(30, "e7e5"), ev(25, "d2d4"), ev(-600, null)]; // 3.Nf3 collapses White's position
    const r = buildReview(rec, evals);
    expect(r.moves[2].judgement).toBe("blunder");
    expect(r.moves[2].mover).toBe("white");
    expect(r.moves[2].bestSan).toBe("d4");
    expect(r.counts.white.blunder).toBe(1);
    expect(r.accuracy.white!).toBeLessThan(r.accuracy.black!);
  });
  it("evaluates a black blunder from Black's side", () => {
    const evals = [ev(0, "e2e4"), ev(30, "e7e5"), ev(30, "g1f3"), ev(30, null)];
    const swing = [ev(0, "e2e4"), ev(30, "e7e5"), ev(-500, null), ev(-500, null)];
    expect(buildReview(rec, evals).moves[1].judgement).toBe("best");
    const r2 = buildReview(play("e2e4", "e7e5"), [ev(0, "e2e4"), ev(30, "e7e5"), ev(900, null)]);
    expect(r2.moves[1].judgement).toBe("blunder");
    expect(r2.moves[1].mover).toBe("black");
    void swing;
  });
  it("treats opening-book moves kindly", () => {
    const r = buildReview(rec, [ev(20, "d2d4"), ev(30, "d7d5"), ev(25, "e2e4"), ev(30, null)], 3);
    expect(r.moves.every((m) => m.judgement === "book")).toBe(true);
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

describe.runIf(existsSync(ENGINE))("analyseGame with real Stockfish", () => {
  it("spots the blunder in Scholar's mate and the mating move", async () => {
    await withEngine(async (engine) => {
      const rec = play("e2e4", "e7e5", "d1h5", "b8c6", "f1c4", "g8f6", "h5f7"); // 3...Nf6?? allows 4.Qxf7#
      const seen: number[] = [];
      const review = await analyseGame(engine, rec, { depth: 10, movetime: 150, onProgress: (d) => seen.push(d) });
      expect(seen).toEqual([1, 2, 3, 4, 5, 6, 7, 8]);
      expect(review.evals).toHaveLength(8);
      expect(review.evals[7]).toMatchObject({ cp: 100000, mate: 0, searched: false });
      const nf6 = review.moves[5];
      expect(nf6.san).toBe("Nf6");
      expect(nf6.judgement).toBe("blunder");
      expect(nf6.bestSan).toBeTruthy();
      expect(review.moves[6].san).toBe("Qxf7#");
      expect(["best", "good"]).toContain(review.moves[6].judgement);
      expect(review.counts.black.blunder).toBeGreaterThanOrEqual(1);
      expect(review.accuracy.black!).toBeLessThan(review.accuracy.white!);
    });
  }, 90_000);
  it("a sensible opening has no errors and high accuracy", async () => {
    await withEngine(async (engine) => {
      const rec = play("e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "a7a6");
      const review = await analyseGame(engine, rec, { depth: 10, movetime: 150 });
      for (const m of review.moves) expect(["best", "good", "inaccuracy"], `${m.san} ${m.loss}`).toContain(m.judgement);
      expect(review.accuracy.white!).toBeGreaterThan(80);
      expect(review.accuracy.black!).toBeGreaterThan(70);
    });
  }, 90_000);
  it("can be cancelled", async () => {
    await withEngine(async (engine) => {
      const ac = new AbortController();
      const rec = play("e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "a7a6", "b5a4", "g8f6");
      const p = analyseGame(engine, rec, { depth: 12, movetime: 300, signal: ac.signal, onProgress: (d) => d === 2 && ac.abort() });
      await expect(p).rejects.toMatchObject({ name: "AbortError" });
    });
  }, 90_000);
});
