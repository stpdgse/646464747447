import { formatClock } from "@/components/computer/clock";

export function ClockBar({ label, avatar, ms, active, timed, thinking, note }: { label: string; avatar?: string; ms: number; active: boolean; timed: boolean; thinking?: boolean; note?: string }) {
  return (
    <div className={`flex items-center justify-between rounded-lg border px-3 py-2 ${active ? "border-brand bg-brand/10" : "border-line bg-surface"}`}>
      <span className="flex items-center gap-2 font-semibold">
        {avatar && <span aria-hidden className="text-xl">{avatar}</span>}
        {label}
        {thinking && <span className="text-xs font-normal text-dim" role="status">thinking…</span>}
      </span>
      {!timed && note && <span className="text-sm font-medium text-dim" aria-label={`${label} deadline`}>{note}</span>}
      {timed && <span className={`font-mono text-xl tabular-nums ${ms < 10_000 ? "text-danger" : ""}`} aria-label={`${label} clock`}>{formatClock(ms)}</span>}
    </div>
  );
}
