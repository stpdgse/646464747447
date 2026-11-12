import { describe, expect, it } from "vitest";
import { fromPgn, newRecord, tryUci, type GameRecord } from "@/lib/chess/game";
import { bookMoves, describeOpening, keysAlong, openingAt, openingCount } from "./openings";

const play = (...ucis: string[]): GameRecord => {
  let r = newRecord("standard");
  for (const u of ucis) {
    const res = tryUci(r, u);
    if (!res.ok) throw new Error(`illegal ${u}`);
    r = res.record;
  }
  return r;
};

describe("opening names", () => {
  it("knows thousands of openings", () => {
    expect(openingCount()).toBeGreaterThan(3500);
  });
  it("names famous openings", () => {
    expect(openingAt(play("e2e4"))?.opening).toEqual({ eco: "B00", name: "King's Pawn Game" });
    expect(openingAt(play("e2e4", "e7e5", "g1f3", "b8c6", "f1b5"))?.opening.name).toBe("Ruy Lopez");
    expect(openingAt(play("d2d4", "d7d5", "c2c4"))?.opening.name).toBe("Queen's Gambit");
    expect(openingAt(play("e2e4", "c7c5"))?.opening.name).toBe("Sicilian Defense");
    expect(openingAt(play("e2e4", "e7e6"))?.opening.name).toBe("French Defense");
  });
  it("has nothing to say about the start position", () => {
    expect(openingAt(play())).toBeNull();
  });
  it("falls back to the deepest named ancestor and reports where it matched", () => {
    const r = play("e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "a7a6", "b5a4", "g8f6", "e1h1", "f8e7", "a2a3", "h7h6", "h2h3");
    const hit = openingAt(r)!;
    expect(hit.opening.name).toMatch(/Ruy Lopez/);
    expect(hit.ply).toBeLessThanOrEqual(r.moves.length);
    expect(hit.ply).toBeGreaterThanOrEqual(5);
  });
  it("is the same for transposed move orders", () => {
    const a = openingAt(play("d2d4", "g8f6", "c2c4", "e7e6"))!.opening;
    const b = openingAt(play("c2c4", "g8f6", "d2d4", "e7e6"))!.opening;
    expect(a).toEqual(b);
  });
  it("finds the Najdorf and describes it with its ECO code", () => {
    const r = play("e2e4", "c7c5", "g1f3", "d7d6", "d2d4", "c5d4", "f3d4", "g8f6", "b1c3", "a7a6");
    expect(describeOpening(openingAt(r)!.opening)).toBe("Sicilian Defense: Najdorf Variation (B90)");
  });
  it("works at an earlier ply of the same game", () => {
    const r = play("e2e4", "c7c5", "g1f3", "d7d6");
    expect(openingAt(r, 2)?.opening.name).toBe("Sicilian Defense");
  });
  it("does not name other variants or custom starts", () => {
    expect(openingAt({ ...newRecord("chess960", 518), moves: [] })).toBeNull();
    expect(openingAt({ variant: "standard", initialFen: "8/8/8/8/8/8/8/K6k w - - 0 1", moves: [] })).toBeNull();
  });
  it("recognises castling written in our canonical notation", () => {
    const { record } = fromPgn("1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O Be7");
    expect(openingAt(record)?.opening.name).toMatch(/Ruy Lopez/);
  });
});

describe("book moves", () => {
  it("offers e4 and d4 first from the start, with names", () => {
    const moves = bookMoves(play());
    expect(moves.slice(0, 2).map((m) => m.uci).sort()).toEqual(["d2d4", "e2e4"]);
    const e4 = moves.find((m) => m.uci === "e2e4")!;
    expect(e4.reaches?.name).toBe("King's Pawn Game");
    expect(e4.lines).toBeGreaterThan(1000);
    expect(moves.every((m, i) => i === 0 || moves[i - 1].lines >= m.lines)).toBe(true);
  });
  it("shows what an unnamed move leads to", () => {
    // 1.e4 e5 2.Nf3 Nc6 is itself named; pick a move whose exact position has no name of its own.
    const moves = bookMoves(play("e2e4", "e7e5", "g1f3", "b8c6"));
    expect(moves.length).toBeGreaterThan(3);
    for (const m of moves) expect(m.reaches ?? m.leadsTo).toBeTruthy();
  });
  it("is empty once the theory runs out and for illegal prefixes", () => {
    expect(bookMoves(play("a2a3", "a7a6", "h2h3", "h7h6", "b2b3", "b7b6", "g2g3", "g7g6"))).toEqual([]);
  });
  it("only uses legal moves", () => {
    const r = play("e2e4", "e7e5", "g1f3");
    for (const m of bookMoves(r)) expect(tryUci(r, m.uci).ok, m.uci).toBe(true);
  });
  it("keysAlong has one key per position", () => {
    const r = play("e2e4", "e7e5");
    expect(keysAlong(r)).toHaveLength(3);
    expect(new Set(keysAlong(r)).size).toBe(3);
  });
});
