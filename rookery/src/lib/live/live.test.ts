import { describe, expect, it } from "vitest";
import { categoryOf, describeTimeControl, getLiveTimeControl, LIVE_TIME_CONTROLS } from "./timecontrols";
import { clockLeft, deadlineLeft, isNewer, needsTimeoutCheck, sideOf, type GameRow } from "./types";

const base: GameRow = {
  id: "g", variant: "standard", initial_fen: "x", moves: [], white_id: "w", black_id: "b", status: "started", result: null,
  reason: null, time_initial_ms: 180000, time_inc_ms: 2000, white_clock_ms: 180000, black_clock_ms: 180000,
  clock_updated_at: new Date(1_000_000).toISOString(), draw_offer_by: null, rated: false, white_rating: 1500, black_rating: 1500, mode: "live", days_per_move: null, move_deadline: null, created_at: "", ended_at: null,
};
const row = (patch: Partial<GameRow>): GameRow => ({ ...base, ...patch });

describe("clockLeft", () => {
  it("does not run before both sides have moved", () => {
    expect(clockLeft(row({ moves: [] }), "white", 1_050_000)).toBe(180000);
    expect(clockLeft(row({ moves: ["e2e4"] }), "black", 1_050_000)).toBe(180000);
  });
  it("charges only the side to move", () => {
    const r = row({ moves: ["e2e4", "e7e5"] }); // white to move
    expect(clockLeft(r, "white", 1_010_000)).toBe(170000);
    expect(clockLeft(r, "black", 1_010_000)).toBe(180000);
  });
  it("is frozen when the game is over and never negative", () => {
    expect(clockLeft(row({ status: "finished", moves: ["a", "b"] }), "white", 9_999_999)).toBe(180000);
    expect(clockLeft(row({ moves: ["a", "b"], white_clock_ms: 500 }), "white", 1_100_000)).toBe(0);
  });
  it("tolerates a client clock slightly behind the server", () => {
    expect(clockLeft(row({ moves: ["a", "b"] }), "white", 900_000)).toBe(180000);
  });
});

describe("needsTimeoutCheck", () => {
  it("fires when the flag has fallen", () => {
    expect(needsTimeoutCheck(row({ moves: ["a", "b"], white_clock_ms: 1000 }), 1_002_000)).toBe(true);
    expect(needsTimeoutCheck(row({ moves: ["a", "b"], white_clock_ms: 5000 }), 1_002_000)).toBe(false);
  });
  it("fires when nobody made the first move for a minute, never for finished games", () => {
    expect(needsTimeoutCheck(row({ moves: [] }), 1_061_000)).toBe(true);
    expect(needsTimeoutCheck(row({ moves: [] }), 1_030_000)).toBe(false);
    expect(needsTimeoutCheck(row({ status: "finished", moves: [] }), 9_999_999)).toBe(false);
  });
});

describe("daily deadlines", () => {
  const daily = (patch: Partial<GameRow> = {}) => row({ mode: "daily", days_per_move: 3, time_initial_ms: 0, time_inc_ms: 0, white_clock_ms: 0, black_clock_ms: 0, moves: ["a", "b"], move_deadline: new Date(2_000_000).toISOString(), ...patch });
  it("counts down to the deadline and flags overdue games", () => {
    expect(deadlineLeft(daily(), 1_500_000)).toBe(500_000);
    expect(deadlineLeft(daily(), 2_500_000)).toBe(-500_000);
    expect(needsTimeoutCheck(daily(), 1_500_000)).toBe(false);
    expect(needsTimeoutCheck(daily(), 2_000_001)).toBe(true);
  });
  it("ignores finished games and live games", () => {
    expect(needsTimeoutCheck(daily({ status: "finished" }), 9_999_999)).toBe(false);
    expect(deadlineLeft(row({}), 5)).toBeNull();
  });
});

describe("isNewer", () => {
  it("accepts more moves, rejects fewer, accepts same-length status changes", () => {
    expect(isNewer(row({ moves: ["a"] }), row({ moves: ["a", "b"] }))).toBe(true);
    expect(isNewer(row({ moves: ["a", "b"] }), row({ moves: ["a"] }))).toBe(false);
    expect(isNewer(row({ moves: ["a"] }), row({ moves: ["a"], draw_offer_by: "white" }))).toBe(true);
    expect(isNewer(row({ moves: ["a"], status: "finished" }), row({ moves: ["a"], status: "started" }))).toBe(false);
  });
});

describe("sideOf and time controls", () => {
  it("identifies the viewer's side", () => {
    expect(sideOf(base, "w")).toBe("white");
    expect(sideOf(base, "b")).toBe("black");
    expect(sideOf(base, "x")).toBeNull();
    expect(sideOf(base, null)).toBeNull();
  });
  it("categorises like Lichess", () => {
    expect(categoryOf(60_000, 0)).toBe("Bullet");
    expect(categoryOf(180_000, 0)).toBe("Blitz");
    expect(categoryOf(180_000, 2_000)).toBe("Blitz");
    expect(categoryOf(600_000, 0)).toBe("Rapid");
    expect(describeTimeControl(180_000, 2_000)).toBe("3 + 2");
  });
  it("every preset fits the database limits", () => {
    for (const t of LIVE_TIME_CONTROLS) {
      expect(t.ms).toBeGreaterThanOrEqual(15_000);
      expect(t.ms).toBeLessThanOrEqual(10_800_000);
      expect(t.inc).toBeLessThanOrEqual(180_000);
      expect(getLiveTimeControl(t.id)).toBe(t);
    }
  });
});
