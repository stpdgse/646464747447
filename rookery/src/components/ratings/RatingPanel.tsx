"use client";

import { useState } from "react";
import { perfLabel } from "@/lib/ratings";
import { RatingChart, type ChartPoint } from "./RatingChart";

/** Perf tabs above one chart. Filters sit in a single row above the content they scope. */
export function RatingPanel({ series }: { series: Record<string, ChartPoint[]> }) {
  const perfs = Object.keys(series);
  const [perf, setPerf] = useState(perfs[0]);
  if (perfs.length === 0) {
    return <p className="rounded-xl border border-line bg-surface px-4 py-6 text-center text-sm text-dim">No rated games yet.</p>;
  }
  return (
    <div className="grid gap-3">
      <div role="tablist" aria-label="Rating type" className="flex flex-wrap gap-2">
        {perfs.map((p) => (
          <button key={p} role="tab" aria-selected={perf === p} onClick={() => setPerf(p)}
            className={`rounded-full border px-3 py-1 text-sm font-medium ${perf === p ? "border-brand bg-brand/15" : "border-line bg-surface hover:bg-surface-2"}`}>
            {perfLabel(p)}
          </button>
        ))}
      </div>
      <RatingChart points={series[perf] ?? []} title={`${perfLabel(perf)} rating`} />
    </div>
  );
}
