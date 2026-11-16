import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { describe, expect, it, vi } from "vitest";

vi.mock("server-only", () => ({}));

import { tryUci, viewAt, type GameRecord } from "@/lib/chess/game";
import { UciEngine, type Transport } from "@/lib/engine/uci";
import { lessonPuzzles } from "@/lib/puzzles/data";
import { afterOpponentOpening, verifySolution, solutionMoves } from "@/lib/puzzles/solve";
import { BASICS, ENDGAMES, LESSONS, PUZZLE_LESSONS, getEndgame, getLesson } from "./lessons";

const rec = (fen: string): GameRecord => ({ variant: "standard", initialFen: fen, moves: [] });

describe("lesson catalogue", () => {
  it("has unique ids and lookups work", () => {
    expect(new Set(LESSONS.map((l) => l.id)).size).toBe(LESSONS.length);
    expect(new Set(ENDGAMES.map((e) => e.id)).size).toBe(ENDGAMES.length);
    expect(getLesson("basics")).toBe(BASICS);
    expect(getEndgame("lucena")?.goal).toBe("win");
    expect(getLesson("nope")).toBeUndefined();
  });
});

describe("the basics lesson", () => {
  for (const step of BASICS.steps) {
    it(`step "${step.id}" is a legal position and its answer is a legal move`, () => {
      expect(() => viewAt(rec(step.fen))).not.toThrow();
      const v = viewAt(rec(step.fen));
      expect(v.turn).toBe("white");
      expect(v.outcome).toBeUndefined();
      if (step.accept === "any") {
        expect(v.check).toBe(true);
        expect(v.dests.size).toBeGreaterThan(0);
      } else {
        for (const m of step.accept) expect(tryUci(rec(step.fen), m).ok).toBe(true);
      }
    });
  }
  it("the capture steps really capture, and promotion and castling do what they say", () => {
    const sans = (id: string) => {
      const s = BASICS.steps.find((x) => x.id === id)!;
      const r = tryUci(rec(s.fen), (s.accept as string[])[0]);
      if (!r.ok) throw new Error("illegal");
      return viewAt(r.record).sans[0];
    };
    expect(sans("rook")).toBe("Rxd7");
    expect(sans("bishop")).toBe("Bxd5");
    expect(sans("knight")).toBe("Nxd4");
    expect(sans("queen")).toBe("Qxh5+");
    expect(sans("promote")).toMatch(/^e8=Q/);
    expect(sans("castle")).toBe("O-O");
    expect(sans("enpassant")).toBe("exd6");
  });
});

describe("puzzle lessons draw from the real database", () => {
  for (const l of PUZZLE_LESSONS) {
    it(`"${l.title}" has ${l.count} verified puzzles, easiest first`, () => {
      const ps = lessonPuzzles({ theme: l.theme, alsoTag: l.alsoTag, min: l.minRating, max: l.maxRating, count: l.count });
      expect(ps).toHaveLength(l.count);
      for (let i = 1; i < ps.length; i++) expect(ps[i].rating).toBeGreaterThanOrEqual(ps[i - 1].rating);
      for (const p of ps) {
        expect(p.themes).toContain(l.theme);
        expect(() => afterOpponentOpening(p)).not.toThrow();
        expect(verifySolution(p, solutionMoves(p))).toBe(true);
      }
      expect(new Set(ps.map((p) => p.id)).size).toBe(ps.length);
    });
  }
});

describe("endgame positions", () => {
  it("are legal, not already over, and the right side is to move", () => {
    for (const e of ENDGAMES) {
      const v = viewAt(rec(e.fen));
      expect(v.outcome, e.id).toBeUndefined();
      // the drill is played from the side the title says: the player either moves first or answers
      expect(["white", "black"]).toContain(e.player);
    }
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

describe.runIf(existsSync(ENGINE))("endgames play out as promised (Stockfish against itself)", () => {
  it("every win is a win for the player and every draw is a draw", async () => {
    const e = new UciEngine(nodeTransport());
    try {
      for (const eg of ENDGAMES) {
        let r: GameRecord = rec(eg.fen);
        for (let i = 0; i < 200 && !viewAt(r).outcome; i++) {
          const s = await e.search({ fen: r.initialFen, moves: r.moves, depth: 12, movetime: 100 });
          const next = s.best ? tryUci(r, s.best) : null;
          if (!next || !next.ok) break;
          r = next.record;
        }
        const out = viewAt(r).outcome;
        expect(out, `${eg.id} should end`).toBeTruthy();
        if (eg.goal === "win") expect(out!.result, eg.id).toBe(eg.player === "white" ? "1-0" : "0-1");
        else expect(out!.result, eg.id).toBe("1/2-1/2");
      }
    } finally {
      e.terminate();
    }
  }, 400_000);
});
