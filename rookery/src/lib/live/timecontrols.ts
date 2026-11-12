export type LiveTimeControl = { id: string; label: string; ms: number; inc: number; group: "Bullet" | "Blitz" | "Rapid" };

export const LIVE_TIME_CONTROLS: LiveTimeControl[] = [
  { id: "1+0", label: "1 + 0", ms: 60_000, inc: 0, group: "Bullet" },
  { id: "2+1", label: "2 + 1", ms: 120_000, inc: 1_000, group: "Bullet" },
  { id: "3+0", label: "3 + 0", ms: 180_000, inc: 0, group: "Blitz" },
  { id: "3+2", label: "3 + 2", ms: 180_000, inc: 2_000, group: "Blitz" },
  { id: "5+0", label: "5 + 0", ms: 300_000, inc: 0, group: "Blitz" },
  { id: "5+3", label: "5 + 3", ms: 300_000, inc: 3_000, group: "Blitz" },
  { id: "10+0", label: "10 + 0", ms: 600_000, inc: 0, group: "Rapid" },
  { id: "10+5", label: "10 + 5", ms: 600_000, inc: 5_000, group: "Rapid" },
  { id: "15+10", label: "15 + 10", ms: 900_000, inc: 10_000, group: "Rapid" },
];

export const getLiveTimeControl = (id: string) => LIVE_TIME_CONTROLS.find((t) => t.id === id);

export const describeTimeControl = (ms: number, inc: number) => `${Math.round(ms / 60_000 * 100) / 100} + ${Math.round(inc / 1000)}`;

/** Bullet < 3 min, Blitz < 10 min, Rapid otherwise, using base + 40 x increment like Lichess. */
export function categoryOf(ms: number, inc: number): "Bullet" | "Blitz" | "Rapid" {
  const total = ms / 1000 + 40 * (inc / 1000);
  return total < 180 ? "Bullet" : total < 480 ? "Blitz" : "Rapid";
}
