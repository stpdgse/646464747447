import { describe, expect, it } from "vitest";
import { afterMove, clocksRunning, formatClock, remaining } from "./clock";

describe("clock maths", () => {
  const clocks = { white: 60_000, black: 60_000 };
  it("does not run before both sides have moved", () => {
    expect(clocksRunning(0)).toBe(false);
    expect(clocksRunning(1)).toBe(false);
    expect(clocksRunning(2)).toBe(true);
    expect(remaining(clocks, "white", "white", 0, 5_000, 0)).toBe(60_000);
    expect(afterMove(clocks, "white", 0, 5_000, 0, 2_000)).toEqual(clocks);
  });
  it("counts down only the side to move", () => {
    expect(remaining(clocks, "white", "white", 1_000, 4_000, 4)).toBe(57_000);
    expect(remaining(clocks, "white", "black", 1_000, 4_000, 4)).toBe(60_000);
  });
  it("charges elapsed time and adds the increment on a move", () => {
    expect(afterMove(clocks, "white", 1_000, 4_000, 4, 2_000)).toEqual({ white: 59_000, black: 60_000 });
  });
  it("never goes below zero and ignores clock skew", () => {
    expect(remaining({ white: 1_000, black: 1 }, "white", "white", 0, 9_999, 4)).toBe(0);
    expect(remaining(clocks, "white", "white", 5_000, 1_000, 4)).toBe(60_000);
    expect(afterMove({ white: 1_000, black: 0 }, "white", 0, 9_000, 4, 0).white).toBe(0);
  });
  it("formats like a chess clock", () => {
    expect(formatClock(65_000)).toBe("1:05");
    expect(formatClock(600_000)).toBe("10:00");
    expect(formatClock(9_400)).toBe("9.4");
    expect(formatClock(0)).toBe("0.0");
    expect(formatClock(-5)).toBe("0.0");
  });
});
