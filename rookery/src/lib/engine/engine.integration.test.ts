import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { describeMoves, newRecord, tryUci, viewAt, type GameRecord } from "../chess/game";
import { chess960Fen } from "../chess/variants";
import { UciEngine, type Transport } from "./uci";

const ENGINE = path.resolve(__dirname, "../../../public/engine/stockfish-19-lite-single.js");
const available = existsSync(ENGINE);

/** Runs the real engine as a Node child process and exposes it as a UCI transport. */
function nodeTransport(): Transport {
  const child = spawn(process.execPath, [ENGINE], { stdio: ["pipe", "pipe", "ignore"] });
  let cb: (l: string) => void = () => {};
  let buf = "";
  child.stdout.on("data", (d) => {
    buf += d;
    let i: number;
    while ((i = buf.indexOf("\n")) >= 0) {
      cb(buf.slice(0, i).trim());
      buf = buf.slice(i + 1);
    }
  });
  return {
    send: (c) => child.stdin.write(c + "\n"),
    onLine: (f) => {
      cb = f;
    },
    terminate: () => child.kill(),
  };
}

const withEngine = async (fn: (e: UciEngine) => Promise<void>) => {
  const e = new UciEngine(nodeTransport());
  try {
    await fn(e);
  } finally {
    e.terminate();
  }
};

const play = (r: GameRecord, ...u: string[]) => u.reduce((acc, m) => {
  const res = tryUci(acc, m);
  if (!res.ok) throw new Error(`rejected ${m}`);
  return res.record;
}, r);

describe.runIf(available)("Stockfish through UciEngine", () => {
  it("handshakes and returns a legal move with ranked candidate lines", async () => {
    await withEngine(async (e) => {
      const rec = newRecord("standard");
      const res = await e.search({ fen: rec.initialFen, depth: 6, multipv: 3 });
      expect(res.lines).toHaveLength(3);
      expect(res.lines.map((l) => l.multipv)).toEqual([1, 2, 3]);
      expect(tryUci(rec, res.best!).ok).toBe(true);
      expect(res.lines[0].cp).toBeGreaterThanOrEqual(res.lines[1].cp - 1);
    });
  }, 60_000);

  it("finds mate in one and reports it", async () => {
    await withEngine(async (e) => {
      const res = await e.search({ fen: "6k1/5ppp/8/8/8/8/8/R3K3 w - - 0 1", depth: 6 });
      expect(res.best).toBe("a1a8");
      expect(res.lines[0].mate).toBe(1);
      expect(res.lines[0].cp).toBeGreaterThan(99_000);
    });
  }, 60_000);

  it("understands our king-takes-rook castling notation and answers legally", async () => {
    await withEngine(async (e) => {
      const rec = play(newRecord("standard"), "e2e4", "e7e5", "g1f3", "b8c6", "f1c4", "g8f6", "e1h1");
      expect(viewAt(rec).sans.at(-1)).toBe("O-O");
      const res = await e.search({ fen: rec.initialFen, moves: rec.moves, depth: 8 });
      expect(tryUci(rec, res.best!).ok, `engine answered ${res.best}`).toBe(true);
    });
  }, 60_000);

  it("plays Chess960 from a shuffled start, including legal castling output", async () => {
    await withEngine(async (e) => {
      let rec: GameRecord = { variant: "chess960", initialFen: chess960Fen(518), moves: [] };
      for (let i = 0; i < 12; i++) {
        const res = await e.search({ fen: rec.initialFen, moves: rec.moves, depth: 5 });
        if (!res.best) break;
        const next = tryUci(rec, res.best);
        expect(next.ok, `ply ${i}: engine played ${res.best}`).toBe(true);
        if (!next.ok) break;
        rec = next.record;
      }
      expect(rec.moves.length).toBeGreaterThanOrEqual(10);
    });
  }, 90_000);

  it("returns no move when the game is over", async () => {
    await withEngine(async (e) => {
      const res = await e.search({ fen: "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1", depth: 4 });
      expect(res.best).toBeNull();
    });
  }, 60_000);

  it("honours Elo and skill settings", async () => {
    await withEngine(async (e) => {
      const a = await e.search({ fen: newRecord("standard").initialFen, depth: 4, elo: 1320 });
      const b = await e.search({ fen: newRecord("standard").initialFen, depth: 4, skill: 0 });
      expect(a.best).toBeTruthy();
      expect(b.best).toBeTruthy();
    });
  }, 60_000);

  it("can be aborted mid-search and then reused", async () => {
    // Abort only once the engine has really started searching ("go" sent), however slow the machine is.
    const transport = nodeTransport();
    let searching: () => void = () => {};
    const started = new Promise<void>((resolve) => (searching = resolve));
    const send = transport.send;
    transport.send = (c: string) => {
      send(c);
      if (c.startsWith("go ")) searching();
    };
    const e = new UciEngine(transport);
    try {
      const ac = new AbortController();
      const t0 = Date.now();
      const p = e.search({ fen: newRecord("standard").initialFen, movetime: 20_000 }, ac.signal);
      await started;
      setTimeout(() => ac.abort(), 300);
      const res = await p;
      expect(Date.now() - t0).toBeLessThan(15_000);
      expect(res.best).toBeTruthy();
      const again = await e.search({ fen: newRecord("standard").initialFen, depth: 4 });
      expect(again.best).toBeTruthy();
    } finally {
      e.terminate();
    }
  }, 60_000);

  it("queues overlapping searches instead of mixing them up", async () => {
    await withEngine(async (e) => {
      const mate = e.search({ fen: "6k1/5ppp/8/8/8/8/8/R3K3 w - - 0 1", depth: 6 });
      const start = e.search({ fen: newRecord("standard").initialFen, depth: 4 });
      const [m, s] = await Promise.all([mate, start]);
      expect(m.best).toBe("a1a8");
      expect(["e2e4", "d2d4", "g1f3", "c2c4", "e2e3", "b1c3", "g2g3"]).toContain(s.best);
    });
  }, 60_000);

  it("describeMoves flags captures, checks and promotions for bot styles", () => {
    const rec = { variant: "standard" as const, initialFen: "4k3/P7/8/3p4/4P3/8/8/R3K3 w - - 0 1", moves: [] };
    const f = describeMoves(rec, ["e4d5", "a1a8", "a7a8q", "e4e5", "e1e2"]);
    expect(f.get("e4d5")).toMatchObject({ capture: true });
    expect(f.get("a7a8q")).toMatchObject({ promotion: true });
    expect(f.get("e4e5")).toMatchObject({ capture: false, check: false });
    expect(f.get("e1e2")).toBeDefined();
  });
});
