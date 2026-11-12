import { describe, expect, it } from "vitest";
import { formatRating, isProvisional, perfOf, signed, tally } from "./ratings";

describe("perfOf (must agree with private.perf_of in SQL)", () => {
  it.each([
    ["standard", 60000, 0, "bullet"],
    ["standard", 120000, 1000, "bullet"],
    ["standard", 180000, 0, "blitz"],
    ["standard", 180000, 2000, "blitz"],
    ["standard", 300000, 3000, "blitz"],
    ["standard", 600000, 0, "rapid"],
    ["standard", 900000, 10000, "rapid"],
    ["chess960", 60000, 0, "chess960"],
    ["kingOfTheHill", 600000, 0, "kingOfTheHill"],
    ["threeCheck", 180000, 2000, "threeCheck"],
  ] as const)("%s %i+%i -> %s", (variant, ms, inc, perf) => {
    expect(perfOf(variant, ms, inc)).toBe(perf);
  });
});

describe("rating display", () => {
  it("marks ratings with high uncertainty as provisional", () => {
    expect(isProvisional(350)).toBe(true);
    expect(isProvisional(110)).toBe(false);
    expect(formatRating(1662.4, 290)).toBe("1662?");
    expect(formatRating(1500.2, 60)).toBe("1500");
  });
  it("formats changes with a real minus sign", () => {
    expect(signed(16)).toBe("+16");
    expect(signed(-9)).toBe("−9");
    expect(signed(0)).toBe("±0");
  });
});

describe("tally", () => {
  const games = [
    { white_id: "a", black_id: "b", result: "1-0" },
    { white_id: "b", black_id: "a", result: "1-0" },
    { white_id: "a", black_id: "b", result: "1/2-1/2" },
    { white_id: "b", black_id: "a", result: "0-1" },
    { white_id: "a", black_id: "b", result: null },
  ];
  it("counts results from the player's side and ignores unfinished games", () => {
    expect(tally(games, "a")).toEqual({ wins: 2, draws: 1, losses: 1, total: 4 });
    expect(tally(games, "b")).toEqual({ wins: 1, draws: 1, losses: 2, total: 4 });
  });
});
