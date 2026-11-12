"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { formatEval, winPercent, type Review } from "@/lib/analysis/analysis";

const H = 150;
const M = { l: 34, r: 12, t: 10, b: 22 };

/** White's chance of winning over the game. Hover or use the arrow keys to inspect a move; click to jump to it. */
export function EvalGraph({ review, ply, onSelect }: { review: Review; ply: number; onSelect: (ply: number) => void }) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(560);
  const [hover, setHover] = useState<number | null>(null);

  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setWidth(Math.max(260, Math.floor(el.clientWidth))));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const n = review.evals.length;
  const geo = useMemo(() => {
    const wins = review.evals.map((e) => winPercent(e.cp));
    const innerW = width - M.l - M.r;
    const innerH = H - M.t - M.b;
    const x = (i: number) => M.l + (n === 1 ? innerW / 2 : (i / (n - 1)) * innerW);
    const y = (w: number) => M.t + (1 - w / 100) * innerH;
    const line = wins.map((w, i) => `${i === 0 ? "M" : "L"}${x(i).toFixed(1)},${y(w).toFixed(1)}`).join("");
    const area = `${line}L${x(n - 1).toFixed(1)},${(M.t + innerH).toFixed(1)}L${x(0).toFixed(1)},${(M.t + innerH).toFixed(1)}Z`;
    return { wins, x, y, line, area, innerW, innerH };
  }, [review, width, n]);

  if (n < 2) return null;
  const shown = hover ?? ply;
  const cx = geo.x(shown);
  const e = review.evals[shown];
  const move = shown > 0 ? review.moves[shown - 1] : null;
  const tipLeft = cx > width * 0.6;

  const pick = (clientX: number, rect: DOMRect) => {
    const rel = (clientX - rect.left - M.l) / geo.innerW;
    return Math.max(0, Math.min(n - 1, Math.round(rel * (n - 1))));
  };

  return (
    <figure className="grid min-w-0 grid-cols-1 gap-2" aria-label="Evaluation graph">
      <div ref={wrapRef} className="relative min-w-0">
        <svg
          width={width} height={H} role="img" tabIndex={0}
          aria-label={`White's winning chances over ${n - 1} moves. Use the left and right arrow keys to move through the game.`}
          className="block cursor-pointer overflow-visible rounded-md outline-offset-4"
          onPointerMove={(ev) => setHover(pick(ev.clientX, ev.currentTarget.getBoundingClientRect()))}
          onPointerLeave={() => setHover(null)}
          onClick={(ev) => onSelect(pick(ev.clientX, ev.currentTarget.getBoundingClientRect()))}
          onKeyDown={(ev) => {
            if (ev.key === "ArrowLeft") { onSelect(Math.max(0, ply - 1)); ev.preventDefault(); }
            else if (ev.key === "ArrowRight") { onSelect(Math.min(n - 1, ply + 1)); ev.preventDefault(); }
          }}
        >
          {[0, 50, 100].map((t) => (
            <g key={t}>
              <line x1={M.l} x2={width - M.r} y1={geo.y(t)} y2={geo.y(t)} stroke="var(--line)" strokeWidth={1} />
              <text x={M.l - 6} y={geo.y(t)} textAnchor="end" dominantBaseline="middle" fontSize={10} fill="var(--ink-dim)">{t}%</text>
            </g>
          ))}
          <text x={M.l} y={H - 6} fontSize={10} fill="var(--ink-dim)">Start</text>
          <text x={width - M.r} y={H - 6} textAnchor="end" fontSize={10} fill="var(--ink-dim)">Move {Math.ceil((n - 1) / 2)}</text>
          <path d={geo.area} fill="var(--brand)" opacity={0.1} />
          <path d={geo.line} fill="none" stroke="var(--brand-hi)" strokeWidth={2} strokeLinejoin="round" strokeLinecap="round" />
          {/* Errors: a ring for a blunder, a diamond for a mistake, so colour is never the only signal */}
          {review.moves.map((m) => m.judgement === "blunder" || m.judgement === "mistake" ? (
            m.judgement === "blunder"
              ? <circle key={m.ply} cx={geo.x(m.ply)} cy={geo.y(geo.wins[m.ply])} r={4.5} fill="var(--danger)" stroke="var(--surface)" strokeWidth={2} />
              : <rect key={m.ply} x={geo.x(m.ply) - 4} y={geo.y(geo.wins[m.ply]) - 4} width={8} height={8} fill="var(--warn)" stroke="var(--surface)" strokeWidth={2} transform={`rotate(45 ${geo.x(m.ply)} ${geo.y(geo.wins[m.ply])})`} />
          ) : null)}
          <line x1={cx} x2={cx} y1={M.t} y2={M.t + geo.innerH} stroke="var(--ink-dim)" strokeWidth={1} />
          <circle cx={cx} cy={geo.y(geo.wins[shown])} r={4.5} fill="var(--brand-hi)" stroke="var(--surface)" strokeWidth={2} />
          <rect x={M.l} y={0} width={geo.innerW} height={H} fill="transparent" />
        </svg>
        {(hover !== null) && (
          <div role="status" className="pointer-events-none absolute top-0 z-10 rounded-lg border border-line bg-surface-2 px-3 py-2 text-xs shadow-lg"
            style={{ left: tipLeft ? undefined : cx + 12, right: tipLeft ? width - cx + 12 : undefined }}>
            <div className="text-base font-bold text-ink">{formatEval(e)}</div>
            <div className="text-dim">{move ? `${Math.ceil(move.ply / 2)}${move.mover === "white" ? "." : "..."} ${move.san}` : "Start position"}</div>
            <div className="text-dim">White {Math.round(geo.wins[shown])}%</div>
          </div>
        )}
      </div>
      <p className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-dim">
        <span className="inline-flex items-center gap-1.5"><span aria-hidden className="inline-block size-2.5 rounded-full" style={{ background: "var(--danger)" }} />Blunder</span>
        <span className="inline-flex items-center gap-1.5"><span aria-hidden className="inline-block size-2.5 rotate-45" style={{ background: "var(--warn)" }} />Mistake</span>
        <span>Line: White&apos;s winning chances</span>
      </p>
    </figure>
  );
}
