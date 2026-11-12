export type TimeControl = { id: string; label: string; initialMs: number; incMs: number };

export const TIME_CONTROLS: TimeControl[] = [
  { id: "unlimited", label: "Unlimited", initialMs: 0, incMs: 0 },
  { id: "1+0", label: "1 min (Bullet)", initialMs: 60_000, incMs: 0 },
  { id: "3+0", label: "3 min (Blitz)", initialMs: 180_000, incMs: 0 },
  { id: "3+2", label: "3 | 2 (Blitz)", initialMs: 180_000, incMs: 2_000 },
  { id: "5+0", label: "5 min (Blitz)", initialMs: 300_000, incMs: 0 },
  { id: "5+3", label: "5 | 3 (Blitz)", initialMs: 300_000, incMs: 3_000 },
  { id: "10+0", label: "10 min (Rapid)", initialMs: 600_000, incMs: 0 },
  { id: "10+5", label: "10 | 5 (Rapid)", initialMs: 600_000, incMs: 5_000 },
  { id: "15+10", label: "15 | 10 (Rapid)", initialMs: 900_000, incMs: 10_000 },
];

export type Side = "white" | "black";
export type Clocks = { white: number; black: number };

/** A side's clock only runs once both players have moved once, as on most sites. */
export const clocksRunning = (plyCount: number) => plyCount >= 2;

/** Time on `side`'s clock at `now`, given the clock snapshot taken at `startedAt` for the side to move. */
export function remaining(clocks: Clocks, turn: Side, side: Side, startedAt: number, now: number, plyCount: number): number {
  const base = clocks[side];
  if (side !== turn || !clocksRunning(plyCount) || now <= startedAt) return Math.max(0, base);
  return Math.max(0, base - (now - startedAt));
}

/** New clock snapshot after `mover` plays at time `t`. */
export function afterMove(clocks: Clocks, mover: Side, startedAt: number, t: number, plyCountBefore: number, incMs: number): Clocks {
  if (!clocksRunning(plyCountBefore)) return clocks;
  return { ...clocks, [mover]: Math.max(0, clocks[mover] - Math.max(0, t - startedAt)) + incMs };
}

export function formatClock(ms: number): string {
  const total = Math.max(0, ms);
  if (total < 10_000) return `${Math.floor(total / 1000)}.${Math.floor((total % 1000) / 100)}`;
  const s = Math.ceil(total / 1000);
  const m = Math.floor(s / 60);
  return `${m}:${String(s % 60).padStart(2, "0")}`;
}
