import { describe, expect, it } from "vitest";
import {
  EXTRA_GAMES, botElo, bracketOrder, champion, firstRound, knockoutBracket, matchState, maxGames, nextGame, roundName, roundRobinSchedule, standings, validateSpec,
  type GameRow, type Participant, type Spec,
} from "./botTournament";

const bot = (id: string, rating: number): Participant => ({ id, name: id.toUpperCase(), rating });
const four = [bot("a", 1000), bot("b", 900), bot("c", 800), bot("d", 700)];
const eight = [1, 2, 3, 4, 5, 6, 7, 8].map((i) => bot("b" + i, 2000 - i * 100));
const game = (round: number, slot: number, game_no: number, w: string, b: string, result: GameRow["result"], status: GameRow["status"] = "finished"): GameRow => ({
  round, slot, game_no, white_bot_id: w, black_bot_id: b, status, result,
});

describe("validation", () => {
  it("accepts 4, 8 and 16 for knockouts and 3 to 8 for round-robins, and refuses duplicates", () => {
    expect(validateSpec({ format: "knockout", bestOf: 1, bots: four })).toBeNull();
    expect(validateSpec({ format: "knockout", bestOf: 1, bots: eight })).toBeNull();
    expect(validateSpec({ format: "knockout", bestOf: 1, bots: four.slice(0, 3) })).toMatch(/4, 8 or 16/);
    expect(validateSpec({ format: "roundrobin", bestOf: 1, bots: four.slice(0, 3) })).toBeNull();
    expect(validateSpec({ format: "roundrobin", bestOf: 1, bots: four.slice(0, 2) })).toMatch(/3 to 8/);
    expect(validateSpec({ format: "knockout", bestOf: 1, bots: [four[0], four[0], four[2], four[3]] })).toMatch(/only enter once/);
  });
});

describe("seeding", () => {
  it("seeds so 1 plays the last seed and the top seeds only meet late", () => {
    expect(bracketOrder(4)).toEqual([1, 4, 2, 3]);
    expect(bracketOrder(8)).toEqual([1, 8, 4, 5, 2, 7, 3, 6]);
    expect(new Set(bracketOrder(16)).size).toBe(16);
    expect(firstRound(four).map((b) => b.id)).toEqual(["a", "d", "b", "c"]);
  });
  it("names the rounds", () => {
    expect([1, 2, 3].map((r) => roundName(r, 3))).toEqual(["Quarter-finals", "Semi-finals", "Final"]);
    expect(roundName(1, 4)).toBe("Round of 16");
  });
});

describe("a knockout match", () => {
  const A = bot("a", 1000), Bb = bot("b", 900);
  it("is open until a game is finished", () => {
    expect(matchState(A, Bb, [], 1).winner).toBeNull();
    expect(matchState(A, Bb, [game(1, 0, 1, "a", "b", null, "playing")], 1).live).not.toBeNull();
  });
  it("best of 1: one win decides it, and colours do not matter", () => {
    expect(matchState(A, Bb, [game(1, 0, 1, "a", "b", "1-0")], 1).winner?.id).toBe("a");
    expect(matchState(A, Bb, [game(1, 0, 1, "a", "b", "0-1")], 1).winner?.id).toBe("b");
  });
  it("best of 1: a draw adds games, up to two extra, then the higher rating goes through", () => {
    const d = (n: number) => game(1, 0, n, n % 2 ? "a" : "b", n % 2 ? "b" : "a", "1/2-1/2");
    expect(matchState(A, Bb, [d(1)], 1).winner).toBeNull();
    expect(matchState(A, Bb, [d(1), d(2)], 1).winner).toBeNull();
    const after = matchState(A, Bb, [d(1), d(2), d(3)], 1);
    expect(after.played).toBe(1 + EXTRA_GAMES);
    expect(after.winner?.id).toBe("a"); // higher rating
    expect(matchState(A, Bb, [d(1), d(2), game(1, 0, 3, "a", "b", "0-1")], 1).winner?.id).toBe("b"); // an extra game can still decide it
  });
  it("best of 3: two wins end it early; otherwise the total score decides", () => {
    expect(matchState(A, Bb, [game(1, 0, 1, "a", "b", "1-0"), game(1, 0, 2, "b", "a", "0-1")], 3).winner?.id).toBe("a");
    expect(matchState(A, Bb, [game(1, 0, 1, "a", "b", "1-0"), game(1, 0, 2, "b", "a", "1-0")], 3).winner).toBeNull();
    const third = matchState(A, Bb, [game(1, 0, 1, "a", "b", "1-0"), game(1, 0, 2, "b", "a", "1-0"), game(1, 0, 3, "a", "b", "0-1")], 3);
    expect(third.winner?.id).toBe("b");
  });
  it("without both entrants nothing is decided", () => {
    expect(matchState(A, null, [], 1).winner).toBeNull();
  });
});

describe("running a knockout", () => {
  const spec: Spec = { format: "knockout", bestOf: 1, bots: four };
  it("starts with the first semi-final, higher seed on white", () => {
    expect(nextGame(spec, [])).toEqual({ kind: "new", round: 1, slot: 0, gameNo: 1, whiteId: "a", blackId: "d" });
  });
  it("then the second semi-final, then the final between the winners", () => {
    const g1 = game(1, 0, 1, "a", "d", "1-0");
    expect(nextGame(spec, [g1])).toMatchObject({ kind: "new", round: 1, slot: 1, whiteId: "b", blackId: "c" });
    const g2 = game(1, 1, 1, "b", "c", "0-1");
    expect(nextGame(spec, [g1, g2])).toMatchObject({ kind: "new", round: 2, slot: 0, gameNo: 1, whiteId: "a", blackId: "c" });
    expect(champion(spec, [g1, g2])).toBeNull();
    const g3 = game(2, 0, 1, "a", "c", "1-0");
    expect(nextGame(spec, [g1, g2, g3])).toBeNull();
    expect(champion(spec, [g1, g2, g3])?.id).toBe("a");
  });
  it("a game that is still being played is resumed, not restarted", () => {
    const live = game(1, 0, 1, "a", "d", null, "playing");
    expect(nextGame(spec, [live])).toEqual({ kind: "resume", game: live });
  });
  it("a drawn match replays with colours swapped", () => {
    const d = game(1, 0, 1, "a", "d", "1/2-1/2");
    expect(nextGame(spec, [d])).toMatchObject({ round: 1, slot: 0, gameNo: 2, whiteId: "d", blackId: "a" });
  });
  it("the bracket shows who is where", () => {
    const b = knockoutBracket(spec, [game(1, 0, 1, "a", "d", "1-0")]);
    expect(b).toHaveLength(2);
    expect(b[0][0].winner?.id).toBe("a");
    expect(b[1][0].a?.id).toBe("a");
    expect(b[1][0].b).toBeNull();
  });
  it("an 8-bot knockout has seven matches and the right number of games at most", () => {
    const b = knockoutBracket({ format: "knockout", bestOf: 3, bots: eight }, []);
    expect(b.map((r) => r.length)).toEqual([4, 2, 1]);
    expect(maxGames({ format: "knockout", bestOf: 3, bots: eight })).toBe(7 * 5);
  });
});

describe("round-robin", () => {
  const spec: Spec = { format: "roundrobin", bestOf: 1, bots: four };
  it("schedules every pair twice, once with each colour, and each bot at most once per round", () => {
    const s = roundRobinSchedule(four);
    expect(s).toHaveLength(12);
    const key = (x: string, y: string) => [x, y].sort().join("");
    const count = new Map<string, number>();
    for (const g of s) count.set(key(g.whiteId, g.blackId), (count.get(key(g.whiteId, g.blackId)) ?? 0) + 1);
    expect([...count.values()].every((c) => c === 2)).toBe(true);
    for (const r of new Set(s.map((g) => g.round))) {
      const ids = s.filter((g) => g.round === r).flatMap((g) => [g.whiteId, g.blackId]);
      expect(new Set(ids).size).toBe(ids.length);
    }
    expect(maxGames(spec)).toBe(12);
  });
  it("handles an odd number of bots", () => {
    expect(roundRobinSchedule(four.slice(0, 3))).toHaveLength(6);
  });
  it("plays the schedule in order and names a champion at the end", () => {
    const games: GameRow[] = [];
    for (let i = 0; i < 12; i++) {
      const n = nextGame(spec, games);
      expect(n?.kind).toBe("new");
      if (n?.kind !== "new") throw new Error("unexpected");
      games.push(game(n.round, n.slot, n.gameNo, n.whiteId, n.blackId, "1-0"));
    }
    expect(nextGame(spec, games)).toBeNull();
    // white always wins, so everyone has 3 wins; ties break on wins, then rating
    expect(champion(spec, games)?.id).toBe("a");
    const table = standings(four, games);
    expect(table.every((r) => r.played === 6 && r.wins === 3 && r.losses === 3)).toBe(true);
  });
  it("scores 1 for a win and a half for a draw", () => {
    const t = standings(four, [game(1, 0, 1, "a", "b", "1-0"), game(1, 1, 1, "c", "d", "1/2-1/2")]);
    expect(t[0]).toMatchObject({ points: 1, wins: 1 });
    expect(t.find((r) => r.bot.id === "c")).toMatchObject({ points: 0.5, draws: 1 });
  });
});

describe("bot Elo", () => {
  it("moves ratings towards the result, by the same amount in opposite directions", () => {
    const r = botElo({ a: 1000, b: 1000 }, [{ white_bot_id: "a", black_bot_id: "b", result: "1-0" }]);
    expect(r.a.elo).toBeCloseTo(1012, 5);
    expect(r.b.elo).toBeCloseTo(988, 5);
    expect(r.a.games).toBe(1);
  });
  it("an upset moves more than an expected win, and a draw between equals changes nothing", () => {
    const upset = botElo({ a: 800, b: 1200 }, [{ white_bot_id: "a", black_bot_id: "b", result: "1-0" }]);
    const normal = botElo({ a: 1200, b: 800 }, [{ white_bot_id: "a", black_bot_id: "b", result: "1-0" }]);
    expect(upset.a.elo - 800).toBeGreaterThan(normal.a.elo - 1200);
    const draw = botElo({ a: 1000, b: 1000 }, [{ white_bot_id: "a", black_bot_id: "b", result: "1/2-1/2" }]);
    expect(draw.a.elo).toBeCloseTo(1000, 5);
  });
  it("ignores unfinished games and unknown bots", () => {
    const r = botElo({ a: 1000, b: 1000 }, [{ white_bot_id: "a", black_bot_id: "b", result: null }, { white_bot_id: "a", black_bot_id: "zzz", result: "1-0" }]);
    expect(r.a.games).toBe(0);
  });
});
