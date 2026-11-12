"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { signed } from "@/lib/ratings";

export type ChartPoint = { rating: number; delta: number; created_at: string };

const H = 220;
const M = { l: 44, r: 52, t: 14, b: 26 };

function niceStep(raw: number) {
  for (const s of [10, 20, 25, 50, 100, 200, 500]) if (raw <= s) return s;
  return 1000;
}

/** A single-series line chart of rating over games played. Hover or arrow keys inspect points; a table view mirrors the data. */
export function RatingChart({ points, title }: { points: ChartPoint[]; title: string }) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(640);
  const [active, setActive] = useState<number | null>(null);
  const [focused, setFocused] = useState(false);

  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setWidth(Math.max(280, Math.floor(el.clientWidth))));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const n = points.length;
  const geo = useMemo(() => {
    const ratings = points.map((p) => p.rating);
    let lo = Math.floor((Math.min(...ratings) - 15) / 25) * 25;
    let hi = Math.ceil((Math.max(...ratings) + 15) / 25) * 25;
    if (hi - lo < 50) { lo -= 25; hi += 25; }
    const step = niceStep((hi - lo) / 4);
    const ticks: number[] = [];
    for (let t = Math.ceil(lo / step) * step; t <= hi; t += step) ticks.push(t);
    const innerW = width - M.l - M.r;
    const innerH = H - M.t - M.b;
    const x = (i: number) => M.l + (n === 1 ? innerW / 2 : (i / (n - 1)) * innerW);
    const y = (v: number) => M.t + (1 - (v - lo) / (hi - lo)) * innerH;
    const line = points.map((p, i) => `${i === 0 ? "M" : "L"}${x(i).toFixed(1)},${y(p.rating).toFixed(1)}`).join("");
    const area = `${line}L${x(n - 1).toFixed(1)},${(M.t + innerH).toFixed(1)}L${x(0).toFixed(1)},${(M.t + innerH).toFixed(1)}Z`;
    return { ticks, x, y, line, area, innerW, innerH };
  }, [points, width, n]);

  if (n < 2) {
    return <p className="rounded-xl border border-line bg-surface px-4 py-6 text-center text-sm text-dim">Play at least two rated games to see a rating graph.</p>;
  }

  const shown = active ?? n - 1;
  const showCursor = active !== null || focused;
  const p = points[shown];
  const cx = geo.x(shown);
  const tipOnLeft = cx > width * 0.6;
  const first = points[0];
  const last = points[n - 1];

  const onMove = (clientX: number, rect: DOMRect) => {
    const rel = (clientX - rect.left - M.l) / geo.innerW;
    setActive(Math.max(0, Math.min(n - 1, Math.round(rel * (n - 1)))));
  };

  return (
    <figure className="grid gap-2" aria-label={`${title} over ${n} games`}>
      <div ref={wrapRef} className="relative">
        <svg
          width={width}
          height={H}
          role="img"
          tabIndex={0}
          aria-label={`${title}: ${Math.round(first.rating)} to ${Math.round(last.rating)} over ${n} games. Use the left and right arrow keys to inspect games.`}
          className="block overflow-visible rounded-md outline-offset-4"
          onPointerMove={(e) => onMove(e.clientX, e.currentTarget.getBoundingClientRect())}
          onPointerLeave={() => setActive(null)}
          onFocus={() => setFocused(true)}
          onBlur={() => { setFocused(false); setActive(null); }}
          onKeyDown={(e) => {
            if (e.key === "ArrowLeft") { setActive(Math.max(0, shown - 1)); e.preventDefault(); }
            else if (e.key === "ArrowRight") { setActive(Math.min(n - 1, shown + 1)); e.preventDefault(); }
            else if (e.key === "Home") { setActive(0); e.preventDefault(); }
            else if (e.key === "End") { setActive(n - 1); e.preventDefault(); }
          }}
        >
          {geo.ticks.map((t) => (
            <g key={t}>
              <line x1={M.l} x2={width - M.r} y1={geo.y(t)} y2={geo.y(t)} stroke="var(--line)" strokeWidth={1} />
              <text x={M.l - 8} y={geo.y(t)} textAnchor="end" dominantBaseline="middle" fontSize={11} fill="var(--ink-dim)">{t}</text>
            </g>
          ))}
          <text x={M.l} y={H - 6} fontSize={11} fill="var(--ink-dim)">Game 1</text>
          <text x={width - M.r} y={H - 6} textAnchor="end" fontSize={11} fill="var(--ink-dim)">Game {n}</text>

          <path d={geo.area} fill="var(--brand)" opacity={0.1} />
          <path d={geo.line} fill="none" stroke="var(--brand-hi)" strokeWidth={2} strokeLinejoin="round" strokeLinecap="round" />

          {showCursor && <line x1={cx} x2={cx} y1={M.t} y2={M.t + geo.innerH} stroke="var(--ink-dim)" strokeWidth={1} />}
          {/* End marker: always shown, direct-labelled with the latest value */}
          <circle cx={geo.x(n - 1)} cy={geo.y(last.rating)} r={4.5} fill="var(--brand-hi)" stroke="var(--surface)" strokeWidth={2} />
          {!showCursor && (
            <text x={geo.x(n - 1) + 10} y={geo.y(last.rating)} dominantBaseline="middle" fontSize={12} fontWeight={600} fill="var(--ink)">{Math.round(last.rating)}</text>
          )}
          {showCursor && <circle cx={cx} cy={geo.y(p.rating)} r={5} fill="var(--brand-hi)" stroke="var(--surface)" strokeWidth={2} />}
          {/* transparent hit area: the pointer only has to be near, not on the line */}
          <rect x={M.l} y={0} width={geo.innerW} height={H} fill="transparent" />
        </svg>

        {showCursor && (
          <div
            role="status"
            className="pointer-events-none absolute top-0 z-10 rounded-lg border border-line bg-surface-2 px-3 py-2 text-xs shadow-lg"
            style={{ left: tipOnLeft ? undefined : cx + 12, right: tipOnLeft ? width - cx + 12 : undefined }}
          >
            <div className="flex items-center gap-2">
              <span aria-hidden className="inline-block h-0.5 w-4 rounded" style={{ background: "var(--brand-hi)" }} />
              <span className="text-base font-bold text-ink">{Math.round(p.rating)}</span>
            </div>
            <div className="text-dim">
              Game {shown + 1}{shown > 0 ? ` · ${signed(Math.round(p.rating) - Math.round(points[shown - 1].rating))}` : ""}
            </div>
          </div>
        )}
      </div>

      <details className="text-sm">
        <summary className="cursor-pointer text-dim hover:text-ink">Show as table</summary>
        <div className="mt-2 max-h-56 overflow-auto rounded-lg border border-line">
          <table className="w-full text-left text-sm">
            <thead className="sticky top-0 bg-surface-2 text-dim"><tr><th className="px-3 py-1.5 font-medium">Game</th><th className="px-3 py-1.5 font-medium">Rating</th><th className="px-3 py-1.5 font-medium">Change</th></tr></thead>
            <tbody>
              {points.map((pt, i) => (
                <tr key={i} className="border-t border-line">
                  <td className="px-3 py-1">{i + 1}</td>
                  <td className="px-3 py-1 font-semibold">{Math.round(pt.rating)}</td>
                  <td className="px-3 py-1 text-dim">{pt.delta === 0 ? "±0" : signed(pt.delta)}</td>
                </tr>
              )).reverse()}
            </tbody>
          </table>
        </div>
      </details>
    </figure>
  );
}
