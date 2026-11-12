"use client";

import { useEffect, useRef } from "react";

export function MoveList({ sans, ply, onSelect }: { sans: string[]; ply: number; onSelect?: (ply: number) => void }) {
  const activeRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    activeRef.current?.scrollIntoView({ block: "nearest" });
  }, [ply]);

  const rows: { n: number; white?: string; black?: string }[] = [];
  for (let i = 0; i < sans.length; i += 2) rows.push({ n: i / 2 + 1, white: sans[i], black: sans[i + 1] });

  const cell = (text: string | undefined, idx: number) =>
    text ? (
      <button
        type="button"
        ref={ply === idx ? activeRef : undefined}
        disabled={!onSelect}
        onClick={() => onSelect?.(idx)}
        aria-current={ply === idx ? "step" : undefined}
        className={`rounded px-2 py-1 text-left font-medium hover:bg-surface-2 ${ply === idx ? "bg-brand/30 text-ink" : "text-ink"}`}
      >
        {text}
      </button>
    ) : (
      <span />
    );

  if (sans.length === 0) return <p className="p-3 text-sm text-dim">No moves yet.</p>;

  return (
    <ol aria-label="Moves" className="grid max-h-72 grid-cols-[2.5rem_1fr_1fr] overflow-y-auto text-sm">
      {rows.map((r) => (
        <li key={r.n} className="contents">
          <span className="px-2 py-1 text-dim">{r.n}.</span>
          {cell(r.white, r.n * 2 - 1)}
          {cell(r.black, r.n * 2)}
        </li>
      ))}
    </ol>
  );
}
