import { describe, expect, it } from "vitest";
import { fromPgn, needsPromotion, newRecord, parseFenOrThrow, toPgn, tryMove, tryUci, viewAt, type GameRecord } from "./game";
import { chess960Fen, startFen, VARIANT_IDS } from "./variants";

/** Play a list of UCI moves, failing loudly if any is rejected. */
function play(record: GameRecord, ...ucis: string[]): GameRecord {
  let r = record;
  for (const uci of ucis) {
    const res = tryUci(r, uci);
    if (!res.ok) throw new Error(`rejected ${uci}: ${res.reason}`);
    r = res.record;
  }
  return r;
}

const count = (r: GameRecord) => [...viewAt(r).dests.values()].reduce((n, d) => n + d.length, 0);

describe("Chess960 start positions", () => {
  it("number 518 is the classical setup and 0 is BBQNNRKR", () => {
    expect(chess960Fen(518).split(" ")[0]).toBe("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR");
    expect(chess960Fen(0).split(" ")[0]).toBe("bbqnnrkr/pppppppp/8/8/8/8/PPPPPPPP/BBQNNRKR");
  });
  it("produces 960 distinct, legal, rule-abiding positions", () => {
    const seen = new Set<string>();
    for (let id = 0; id < 960; id++) {
      const fen = chess960Fen(id);
      const back = fen.split(" ")[0].split("/")[7];
      seen.add(back);
      const b1 = back.indexOf("B");
      const b2 = back.lastIndexOf("B");
      expect((b1 + b2) % 2, `bishops on opposite colours for ${id}`).toBe(1);
      expect(back.indexOf("R")).toBeLessThan(back.indexOf("K"));
      expect(back.lastIndexOf("R")).toBeGreaterThan(back.indexOf("K"));
      expect([...back].sort().join("")).toBe("BBKNNQRR");
      expect(() => viewAt({ variant: "chess960", initialFen: fen, moves: [] })).not.toThrow();
    }
    expect(seen.size).toBe(960);
  });
  it("rejects out-of-range ids", () => {
    expect(() => chess960Fen(960)).toThrow();
    expect(() => chess960Fen(-1)).toThrow();
    expect(() => chess960Fen(1.5)).toThrow();
  });
});

describe("move generation", () => {
  it("has 20 legal moves from every variant's standard start", () => {
    for (const v of VARIANT_IDS.filter((v) => v !== "chess960")) {
      expect(count(newRecord(v)), v).toBe(20);
    }
    expect(count({ variant: "chess960", initialFen: chess960Fen(518), moves: [] })).toBe(20);
  });
  it("rejects illegal moves and wrong-turn moves", () => {
    const r = newRecord("standard");
    expect(tryMove(r, "e2", "e5")).toEqual({ ok: false, reason: "illegal" });
    expect(tryMove(r, "e7", "e5")).toEqual({ ok: false, reason: "illegal" });
    expect(tryMove(r, "e4", "e5")).toEqual({ ok: false, reason: "illegal" });
  });
  it("produces SAN, including check and mate suffixes", () => {
    const r = play(newRecord("standard"), "e2e4", "e7e5", "g1f3", "b8c6", "f1b5");
    expect(viewAt(r).sans).toEqual(["e4", "e5", "Nf3", "Nc6", "Bb5"]);
    const mate = play(newRecord("standard"), "f2f3", "e7e5", "g2g4", "d8h4");
    expect(viewAt(mate).sans.at(-1)).toBe("Qh4#");
  });
});

describe("castling", () => {
  const ready = () => play(newRecord("standard"), "e2e4", "e7e5", "g1f3", "b8c6", "f1c4", "g8f6");
  it("works in standard chess using the king's destination square", () => {
    const r = play(ready(), "e1g1");
    expect(viewAt(r).sans.at(-1)).toBe("O-O");
    expect(viewAt(r).fen.split(" ")[0]).toBe("r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQ1RK1");
  });
  it("also accepts king-takes-rook notation, and highlights the king's real destination", () => {
    const r = play(ready(), "e1h1");
    expect(viewAt(r).sans.at(-1)).toBe("O-O");
    expect(viewAt(r).lastMove).toEqual(["e1", "g1"]);
  });
  it("is refused through check and after the king has moved", () => {
    const moved = play(ready(), "e1e2", "a7a6", "e2e1", "a6a5");
    expect(tryUci(moved, "e1g1").ok).toBe(false);
  });
  it("works from a Chess960 start (king takes rook), landing on c/g files", () => {
    // #518 is classical; clear the f1/g1 pieces and castle short.
    const r = play({ variant: "chess960", initialFen: chess960Fen(518), moves: [] }, "g1f3", "g8f6", "e2e4", "e7e5", "f1c4", "f8c5", "e1h1");
    expect(viewAt(r).sans.at(-1)).toBe("O-O");
    expect(viewAt(r).fen.split(" ")[0].split("/")[7]).toBe("RNBQ1RK1");
  });
  it("handles a non-classical Chess960 castle (king already on g-file side)", () => {
    // White: rooks b1 and f1, king e1. Queenside castle sends king to c1, rook to d1.
    const fen = "1r2kr2/8/8/8/8/8/8/1R2KR2 w BFbf - 0 1";
    const rec = { variant: "chess960" as const, initialFen: fen, moves: [] };
    const r = play(rec, "e1b1");
    expect(viewAt(r).fen.split(" ")[0].split("/")[7]).toBe("2KR1R2");
  });
});

describe("promotion", () => {
  const rec = parseFenOrThrow("standard", "8/P6k/8/8/8/8/8/K7 w - - 0 1");
  it("asks for a piece, then promotes", () => {
    expect(needsPromotion(rec, "a7", "a8")).toBe(true);
    expect(tryMove(rec, "a7", "a8")).toEqual({ ok: false, reason: "promotion" });
    const res = tryMove(rec, "a7", "a8", "knight");
    expect(res.ok && viewAt(res.record).fen.startsWith("N7/")).toBe(true);
  });
  it("does not ask for ordinary moves", () => {
    expect(needsPromotion(newRecord("standard"), "e2", "e4")).toBe(false);
  });
});

describe("game results", () => {
  it("detects checkmate with the right winner", () => {
    const r = play(newRecord("standard"), "f2f3", "e7e5", "g2g4", "d8h4");
    expect(viewAt(r).outcome).toEqual({ result: "0-1", reason: "checkmate" });
    expect(viewAt(r).dests.size).toBe(0);
    expect(tryMove(r, "a2", "a3")).toEqual({ ok: false, reason: "finished" });
  });
  it("detects stalemate", () => {
    const rec = parseFenOrThrow("standard", "7k/8/6K1/8/8/8/8/5Q2 w - - 0 1");
    const r = play(rec, "f1f7");
    expect(viewAt(r).outcome).toEqual({ result: "1/2-1/2", reason: "stalemate" });
  });
  it("detects insufficient material", () => {
    const rec = parseFenOrThrow("standard", "7k/8/8/8/8/8/1N6/K7 w - - 0 1");
    expect(viewAt(rec).outcome?.reason).toBe("insufficient");
  });
  it("draws on the fifty-move rule", () => {
    const rec = parseFenOrThrow("standard", "7k/8/8/8/8/8/8/R3K3 w - - 99 80");
    const r = play(rec, "a1a2");
    expect(viewAt(r).outcome).toEqual({ result: "1/2-1/2", reason: "fifty" });
  });
  it("draws on threefold repetition", () => {
    const shuffle = ["g1f3", "g8f6", "f3g1", "f6g8"];
    const r2 = play(newRecord("standard"), ...shuffle);
    expect(viewAt(r2).outcome).toBeUndefined();
    const r3 = play(r2, ...shuffle);
    expect(viewAt(r3).outcome).toEqual({ result: "1/2-1/2", reason: "threefold" });
  });
  it("King of the Hill: walking to the centre wins", () => {
    const rec = parseFenOrThrow("kingOfTheHill", "7k/8/8/8/8/4K3/8/8 w - - 0 1");
    const r = play(rec, "e3e4");
    expect(viewAt(r).outcome).toEqual({ result: "1-0", reason: "variant" });
  });
  it("Three-check: the third check wins and the counter is tracked", () => {
    const rec = parseFenOrThrow("threeCheck", "4k3/8/8/8/8/8/8/R3K3 w - - 3+3 0 1");
    let r = play(rec, "a1a8");
    expect(viewAt(r).remainingChecks).toMatchObject({ white: 2, black: 3 });
    r = play(r, "e8e7", "a8a7", "e7e8", "a7a8");
    const v = viewAt(r);
    expect(v.outcome).toEqual({ result: "1-0", reason: "variant" });
  });
});

describe("viewAt", () => {
  it("can show any earlier position, and is pure", () => {
    const r = play(newRecord("standard"), "e2e4", "e7e5", "g1f3");
    expect(viewAt(r, 0).fen).toBe(startFen("standard"));
    expect(viewAt(r, 1).lastMove).toEqual(["e2", "e4"]);
    expect(viewAt(r, 99).ply).toBe(3);
    expect(r.moves).toEqual(["e2e4", "e7e5", "g1f3"]);
  });
  it("flags the checked king's square", () => {
    const r = play(newRecord("standard"), "e2e4", "f7f6", "d1h5");
    expect(viewAt(r).check).toBe(true);
    expect(viewAt(r).checkSquare).toBe("e8");
  });
  it("throws on a corrupt record instead of silently continuing", () => {
    expect(() => viewAt({ variant: "standard", initialFen: startFen("standard"), moves: ["e2e5"] })).toThrow(/Illegal move/);
  });
});

describe("PGN", () => {
  it("round-trips a standard game with headers", () => {
    const r = play(newRecord("standard"), "e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "a7a6");
    const pgn = toPgn(r, { White: "Tom", Black: "Anna", Date: "2026.11.12" });
    expect(pgn).toContain('[White "Tom"]');
    expect(pgn).toContain("1. e4 e5 2. Nf3 Nc6 3. Bb5 a6");
    const back = fromPgn(pgn);
    expect(back.record.moves).toEqual(r.moves);
    expect(back.record.variant).toBe("standard");
    expect(back.headers.Black).toBe("Anna");
  });
  it("round-trips Chess960 including a castling move", () => {
    const start = { variant: "chess960" as const, initialFen: chess960Fen(518), moves: [] };
    const r = play(start, "g1f3", "g8f6", "e2e4", "e7e5", "f1c4", "f8c5", "e1h1");
    const pgn = toPgn(r);
    expect(pgn).toContain('[Variant "Chess960"]');
    expect(pgn).toContain("[FEN ");
    const back = fromPgn(pgn);
    expect(back.record.variant).toBe("chess960");
    expect(viewAt(back.record).fen).toBe(viewAt(r).fen);
  });
  it("keeps a from-position start", () => {
    const rec = parseFenOrThrow("standard", "8/P6k/8/8/8/8/8/K7 w - - 0 1");
    const r = play(rec, "a7a8q");
    const back = fromPgn(toPgn(r));
    expect(viewAt(back.record).fen).toBe(viewAt(r).fen);
  });
  it("round-trips King of the Hill", () => {
    const r = play(newRecord("kingOfTheHill"), "e2e4", "e7e5");
    expect(fromPgn(toPgn(r)).record.variant).toBe("kingOfTheHill");
  });
  it("reads pasted PGN with comments, NAGs and a result", () => {
    const { record } = fromPgn('[Event "x"]\n\n1. e4 {best by test} e5 $1 2. Nf3 Nc6 1-0');
    expect(record.moves).toEqual(["e2e4", "e7e5", "g1f3", "b8c6"]);
  });
  it("explains bad input", () => {
    expect(() => fromPgn("")).toThrow(/No game/);
    expect(() => fromPgn("1. e4 e5 2. Ke4")).toThrow(/Illegal or unreadable move "Ke4"/);
  });
});

describe("FEN input", () => {
  it("rejects nonsense and illegal positions", () => {
    expect(() => parseFenOrThrow("standard", "not a fen")).toThrow(/valid FEN/);
    expect(() => parseFenOrThrow("standard", "8/8/8/8/8/8/8/8 w - - 0 1")).toThrow(/not legal/);
  });
});
