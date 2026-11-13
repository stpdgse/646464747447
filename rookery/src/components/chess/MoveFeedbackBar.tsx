"use client";

import { RATING_UI } from "@/lib/analysis/moveRating";
import type { MoveFeedback } from "@/components/computer/useMoveFeedback";

/** Sits under the board: how your last move was rated. */
export function MoveFeedbackBar({ feedback, pending, enabled, onToggle, hint }: { feedback: MoveFeedback | null; pending: boolean; enabled: boolean; onToggle: () => void; hint: string }) {
  const ui = feedback ? RATING_UI[feedback.rating] : null;
  return (
    <div className="flex h-14 items-center gap-3 overflow-hidden rounded-xl border border-line bg-surface px-3" role="status" aria-label="Move feedback">
      {!enabled ? (
        <span className="text-sm text-dim">Move feedback is off.</span>
      ) : feedback && ui ? (
        <>
          <span className="inline-flex shrink-0 items-center gap-1.5 rounded-full px-3 py-1 text-sm font-extrabold text-[#151515]" style={{ background: ui.bg }}>
            <span aria-hidden>{ui.symbol}</span>
            {ui.label}
          </span>
          <span className="min-w-0 truncate text-sm" title={feedback.bestSan ? `${feedback.playedSan}: ${ui.blurb} Better was ${feedback.bestSan}.` : `${feedback.playedSan}: ${ui.blurb}`}>
            <strong>{feedback.playedSan}</strong> <span className="text-dim">{ui.blurb}</span>
            {feedback.bestSan && (feedback.rating === "inaccuracy" || feedback.rating === "mistake" || feedback.rating === "blunder") && (
              <span className="text-dim"> Better was <strong className="text-ink">{feedback.bestSan}</strong>.</span>
            )}
          </span>
        </>
      ) : (
        <span className="text-sm text-dim">{pending ? "Rating your move…" : hint}</span>
      )}
      {enabled && pending && feedback && <span className="hidden shrink-0 text-xs text-dim sm:inline">rating…</span>}
      <button type="button" onClick={onToggle} aria-pressed={enabled} className="ml-auto shrink-0 text-xs text-dim underline hover:text-ink">
        {enabled ? "Turn off feedback" : "Turn on feedback"}
      </button>
    </div>
  );
}
