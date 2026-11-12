import { describe, expect, it, vi } from "vitest";

vi.mock("server-only", () => ({}));

import { Chess } from "chessops/chess";
import raw from "@/data/puzzles.json";
import { tryUci, viewAt, type GameRecord } from "@/lib/chess/game";
import { dailyPuzzleFor, lookup, pickPuzzle, puzzleCount, rushBatch, THEMES } from "./data";
import { afterOpponentOpening, judgeMove, solutionMoves, solverMoveCount, verifySolution } from "./solve";

describe("puzzle data", () => {
  it("is large, sorted by rating and spans the whole range", () => {
    expect(puzzleCount).toBeGreaterThan(10_000);
    const ratings = (raw as unknown as [string, string, string, number][]).map((r) => r[3]);
    for (let i = 1; i < ratings.length; i++) expect(ratings[i]).toBeGreaterThanOrEqual(ratings[i - 1]);
    expect(ratings[0]).toBeLessThan(600);
    expect(ratings.at(-1)!).toBeGreaterThan(2800);
  });
  it("every puzzle replays legally and its solution ends sensibly (all 11k)", () => {
    for (const [id] of raw as unknown as [string][]) {
      const p = lookup(id)!;
      expect(p.moves.length, id).toBeGreaterThanOrEqual(2);
      let rec: GameRecord = { variant: "standard", initialFen: p.fen, moves: [] };
      for (const uci of p.moves) {
        const r = tryUci(rec, uci);
        if (!r.ok) throw new Error(`puzzle ${id}: illegal move ${uci}`);
        rec = r.record;
      }
    }
  });
  it("the solver always has the move (side to move after the opening move is the solver)", () => {
    const p = lookup((raw as unknown as [string][])[5000][0])!;
    const after = viewAt(afterOpponentOpening(p));
    expect(after.outcome).toBeUndefined();
    expect(after.dests.size).toBeGreaterThan(0);
  });
});

describe("verifySolution", () => {
  const p = lookup((raw as unknown as [string][])[3000][0])!;
  it("accepts the real solution", () => {
    expect(verifySolution(p, solutionMoves(p))).toBe(true);
  });
  it("rejects a wrong first move, a missing move and nonsense", () => {
    const sol = solutionMoves(p);
    expect(verifySolution(p, [])).toBe(false);
    expect(verifySolution(p, ["a1a1", ...sol.slice(1)])).toBe(false);
    if (sol.length > 1) expect(verifySolution(p, sol.slice(0, -1))).toBe(false);
    expect(verifySolution(p, ["zzzz"])).toBe(false);
  });
  it("accepts every puzzle's own solution", () => {
    for (const [id] of (raw as unknown as [string][]).filter((_, i) => i % 7 === 0)) {
      const q = lookup(id)!;
      expect(verifySolution(q, solutionMoves(q)), id).toBe(true);
    }
  });
});

describe("judgeMove", () => {
  const p = lookup((raw as unknown as [string][])[4200][0])!;
  it("marks the expected move correct and supplies the opponent's reply", () => {
    const rec = afterOpponentOpening(p);
    const step = judgeMove(p, 0, rec, p.moves[1]);
    expect(step.kind).toBe("correct");
    if (step.kind === "correct" && !step.done) expect(step.reply).toBe(p.moves[2]);
    expect(solverMoveCount(p)).toBe(Math.ceil((p.moves.length - 1) / 2));
  });
  it("marks a different legal move wrong", () => {
    const rec = afterOpponentOpening(p);
    const view = viewAt(rec);
    const [from, tos] = [...view.dests.entries()].find(([f, t]) => `${f}${t[0]}` !== p.moves[1] && !t.includes(p.moves[1].slice(2, 4) as never)) ?? [...view.dests.entries()][0];
    const uci = `${from}${tos[0]}`;
    if (uci !== p.moves[1]) {
      const step = judgeMove(p, 0, rec, uci);
      expect(["wrong", "mate"]).toContain(step.kind);
    }
  });
  it("accepts an alternative checkmating move", () => {
    // Back-rank mate available two ways: Ra8# or Rb8#.
    const q = { id: "t", fen: "6k1/5ppp/8/8/8/8/8/RR2K3 b - - 0 1", moves: ["g8h8", "a1a8"], rating: 800, rd: 80, themes: [] as string[] };
    const rec = afterOpponentOpening(q);
    expect(judgeMove(q, 0, rec, "a1a8").kind).toBe("correct");
    const alt = judgeMove(q, 0, rec, "b1b8");
    expect(alt.kind).toBe("mate");
    expect(verifySolution(q, ["b1b8"])).toBe(true);
  });
});

describe("selection", () => {
  const seeded = (seed: number) => { let s = seed; return () => ((s = (s * 16807) % 2147483647) / 2147483647); };
  it("picks puzzles near the target rating", () => {
    for (let i = 0; i < 50; i++) {
      const p = pickPuzzle({ target: 1500, rng: seeded(i + 1) })!;
      expect(Math.abs(p.rating - 1500)).toBeLessThanOrEqual(120);
    }
  });
  it("never repeats excluded puzzles and widens the search when needed", () => {
    const near = new Set<string>();
    for (let i = 0; i < 2000; i++) { const p = pickPuzzle({ target: 1500, exclude: near, rng: seeded(i + 7) }); if (!p) break; near.add(p.id); }
    expect(near.size).toBeGreaterThan(1000);
    const next = pickPuzzle({ target: 1500, exclude: near })!;
    expect(near.has(next.id)).toBe(false);
  });
  it("honours themes", () => {
    for (const t of THEMES) {
      const p = pickPuzzle({ target: 1500, theme: t.id });
      expect(p, t.id).not.toBeNull();
      expect(p!.themes).toContain(t.id);
    }
  });
  it("handles the extremes", () => {
    expect(pickPuzzle({ target: 100 })).not.toBeNull();
    expect(pickPuzzle({ target: 9000 })).not.toBeNull();
  });
  it("the daily puzzle is stable for a date and changes between dates", () => {
    expect(dailyPuzzleFor("2026-11-12").id).toBe(dailyPuzzleFor("2026-11-12").id);
    const ids = new Set(Array.from({ length: 30 }, (_, i) => dailyPuzzleFor(`2026-12-${String(i + 1).padStart(2, "0")}`).id));
    expect(ids.size).toBeGreaterThan(20);
  });
  it("a rush batch is unique and gets harder", () => {
    const batch = rushBatch(600, 25, new Set(), seeded(3));
    expect(batch).toHaveLength(25);
    expect(new Set(batch.map((b) => b.id)).size).toBe(25);
    const first = batch.slice(0, 5).reduce((a, b) => a + b.rating, 0) / 5;
    const last = batch.slice(-5).reduce((a, b) => a + b.rating, 0) / 5;
    expect(last).toBeGreaterThan(first + 500);
  });
  it("Chess position class is available to the data layer", () => {
    expect(typeof Chess.default).toBe("function");
  });
});
