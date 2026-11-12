"use client";

import { useEffect, useRef } from "react";
import { SYMBOL, formatEval, type Judgement, type Review } from "@/lib/analysis/analysis";
import { plural } from "@/lib/ratings";
import { EvalGraph } from "./EvalGraph";
import type { LiveLine } from "./useAnalysis";

const COLOUR: Record<Judgement, string> = {
  best: "text-ink", good: "text-ink", book: "text-dim", inaccuracy: "text-warn", mistake: "text-warn", blunder: "text-danger",
};
const NAME: Record<Judgement, string> = { best: "Best", good: "Good", book: "Book", inaccuracy: "Inaccuracy", mistake: "Mistake", blunder: "Blunder" };
const PLURAL: Record<Judgement, string> = { best: "Best", good: "Good", book: "Book", inaccuracy: "Inaccuracies", mistake: "Mistakes", blunder: "Blunders" };

export function LiveLines({ lines }: { lines: LiveLine[] }) {
  if (lines.length === 0) return <p className="px-3 py-2 text-sm text-dim">Thinking…</p>;
  return (
    <ol className="grid gap-1 p-2" aria-label="Engine lines">
      {lines.map((l, i) => (
        <li key={i} className="flex items-baseline gap-3 rounded px-2 py-1 text-sm">
          <span className="w-12 shrink-0 font-mono font-bold tabular-nums">{formatEval(l)}</span>
          <span className="truncate text-dim">{l.sans.join(" ")}</span>
        </li>
      ))}
    </ol>
  );
}

export function ReviewSummary({ review }: { review: Review }) {
  const cols = [
    { side: "white" as const, label: "White" },
    { side: "black" as const, label: "Black" },
  ];
  const rows: Judgement[] = ["blunder", "mistake", "inaccuracy"];
  return (
    <div className="grid min-w-0 grid-cols-2 gap-3" aria-label="Review summary">
      {cols.map(({ side, label }) => {
        const acc = review.accuracy[side];
        const c = review.counts[side];
        return (
          <div key={side} className="rounded-xl border border-line bg-surface p-4">
            <p className="text-xs font-semibold text-dim">{label}</p>
            <p className="text-3xl font-extrabold tabular-nums" aria-label={`${label} accuracy`}>{acc === null ? "-" : `${acc.toFixed(1)}%`}</p>
            <p className="text-xs text-dim">accuracy</p>
            <ul className="mt-3 grid gap-1 text-sm">
              {rows.map((j) => (
                <li key={j} className="flex justify-between"><span className={COLOUR[j]}>{plural(c[j], NAME[j], PLURAL[j])}</span></li>
              ))}
              <li className="text-xs text-dim">{c.best + c.good + c.book} solid {c.best + c.good + c.book === 1 ? "move" : "moves"}</li>
            </ul>
          </div>
        );
      })}
    </div>
  );
}

/** The move list with ?!, ? and ?? marks. Selecting a move shows what the engine preferred. */
export function AnnotatedMoves({ review, ply, onSelect }: { review: Review; ply: number; onSelect: (ply: number) => void }) {
  const activeRef = useRef<HTMLButtonElement>(null);
  useEffect(() => { activeRef.current?.scrollIntoView({ block: "nearest" }); }, [ply]);
  const rows: { n: number; w?: number; b?: number }[] = [];
  for (let i = 0; i < review.moves.length; i += 2) rows.push({ n: i / 2 + 1, w: i + 1, b: i + 2 <= review.moves.length ? i + 2 : undefined });

  const cell = (p?: number) => {
    if (!p) return <span />;
    const m = review.moves[p - 1];
    return (
      <button type="button" ref={ply === p ? activeRef : undefined} onClick={() => onSelect(p)} aria-current={ply === p ? "step" : undefined}
        aria-label={`${m.san}${SYMBOL[m.judgement] ? `, ${NAME[m.judgement].toLowerCase()}` : ""}`}
        className={`rounded px-2 py-1 text-left font-medium hover:bg-surface-2 ${ply === p ? "bg-brand/30" : ""} ${COLOUR[m.judgement]}`}>
        {m.san}<span className="ml-0.5 font-bold">{SYMBOL[m.judgement]}</span>
      </button>
    );
  };
  const sel = ply > 0 ? review.moves[ply - 1] : null;
  return (
    <div>
      <ol aria-label="Moves with review marks" className="grid max-h-72 grid-cols-[2.5rem_1fr_1fr] overflow-y-auto text-sm">
        {rows.map((r) => (
          <li key={r.n} className="contents"><span className="px-2 py-1 text-dim">{r.n}.</span>{cell(r.w)}{cell(r.b)}</li>
        ))}
      </ol>
      {sel && (sel.judgement === "inaccuracy" || sel.judgement === "mistake" || sel.judgement === "blunder") && (
        <p className="border-t border-line px-3 py-2 text-sm" role="status">
          <strong className={COLOUR[sel.judgement]}>{sel.san}{SYMBOL[sel.judgement]}</strong> was {sel.judgement === "inaccuracy" ? "an inaccuracy" : `a ${sel.judgement}`}
          {sel.bestSan ? <>. The engine preferred <strong>{sel.bestSan}</strong>.</> : "."}
        </p>
      )}
    </div>
  );
}

export function ReviewGraph({ review, ply, onSelect }: { review: Review; ply: number; onSelect: (ply: number) => void }) {
  return <EvalGraph review={review} ply={ply} onSelect={onSelect} />;
}
