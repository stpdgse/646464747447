"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useEngine } from "@/components/computer/useEngine";
import { analyseGame, type Review } from "@/lib/analysis/analysis";
import { tryUci, viewAt, type GameRecord } from "@/lib/chess/game";

export type LiveLine = { cp: number; mate: number | null; sans: string[]; first: [string, string] | null };
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const keyOf = (r: GameRecord, ply: number) => `${r.variant}|${r.initialFen}|${r.moves.slice(0, ply).join(" ")}`;

/** Principal variation in SAN, plus the squares of its first move (for an arrow). */
function describePv(record: GameRecord, ply: number, pv: string[]): { sans: string[]; first: [string, string] | null } {
  let rec: GameRecord = { ...record, moves: record.moves.slice(0, ply) };
  const sans: string[] = [];
  let first: [string, string] | null = null;
  for (const uci of pv.slice(0, 6)) {
    const res = tryUci(rec, uci);
    if (!res.ok) break;
    rec = res.record;
    const v = viewAt(rec);
    sans.push(v.sans[v.sans.length - 1]);
    if (!first && v.lastMove) first = v.lastMove;
  }
  return { sans, first };
}

/**
 * Engine features for a board: live evaluation with the best lines, and a full-game review.
 * The engine (a Web Worker) is only started when one of them is switched on.
 */
export function useAnalysis({ record, ply }: { record: GameRecord; ply: number }) {
  const [engineOn, setEngineOn] = useState(false);
  const [wantReview, setWantReview] = useState(false);
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [reviewed, setReviewed] = useState<{ key: string; data: Review } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [live, setLive] = useState<{ key: string; lines: LiveLine[] } | null>(null);
  const running = useRef<AbortController | null>(null);

  const engineState = useEngine(engineOn || wantReview || progress !== null);
  const engine = engineState.status === "ready" ? engineState.engine : null;
  const fullKey = keyOf(record, record.moves.length);
  const posKey = keyOf(record, ply);

  // ---- full-game review ------------------------------------------------------------------------
  useEffect(() => {
    if (!wantReview || !engine || running.current) return;
    const ac = new AbortController();
    running.current = ac;
    const target = record;
    const key = keyOf(target, target.moves.length);
    setProgress({ done: 0, total: target.moves.length + 1 });
    setError(null);
    (async () => {
      let bookPly = 0;
      try {
        const lib = await import("@/lib/openings");
        bookPly = lib.openingAt(target)?.ply ?? 0;
      } catch { /* names are optional */ }
      try {
        const data = await analyseGame(engine, target, { depth: 12, movetime: 220, bookPly, signal: ac.signal, onProgress: (done, total) => setProgress({ done, total }) });
        setReviewed({ key, data });
      } catch (e) {
        if ((e as Error).name !== "AbortError") setError("The review could not finish. Please try again.");
      } finally {
        running.current = null;
        setProgress(null);
        setWantReview(false);
      }
    })();
    // The review runs on the record as it was when requested; later edits do not restart it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wantReview, engine]);

  useEffect(() => () => running.current?.abort(), []);

  const requestReview = useCallback(() => { setError(null); setWantReview(true); }, []);
  const cancelReview = useCallback(() => { running.current?.abort(); setWantReview(false); }, []);
  const hideReview = useCallback(() => setReviewed(null), []);

  // ---- live lines for the position on the board --------------------------------------------------
  useEffect(() => {
    if (!engineOn || !engine || progress !== null || wantReview) return;
    if (viewAt(record, ply).outcome) return;
    const ac = new AbortController();
    const key = posKey;
    (async () => {
      await sleep(220); // wait for stepping through moves to settle
      if (ac.signal.aborted) return;
      try {
        const res = await engine.search({ fen: record.initialFen, moves: record.moves.slice(0, ply), movetime: 900, depth: 20, multipv: 3 }, ac.signal);
        if (ac.signal.aborted) return;
        const turn = viewAt(record, ply).turn;
        const sign = turn === "white" ? 1 : -1;
        const lines = res.lines.map((l) => ({ cp: sign * l.cp, mate: l.mate !== null ? sign * l.mate : null, ...describePv(record, ply, l.pv) }));
        setLive({ key, lines });
      } catch { /* engine shut down */ }
    })();
    return () => ac.abort();
  }, [engineOn, engine, progress, wantReview, record, ply, posKey]);

  const review = reviewed && reviewed.key === fullKey ? reviewed.data : null;
  const lines = useMemo(() => (engineOn && live && live.key === posKey ? live.lines : []), [engineOn, live, posKey]);

  const evalNow = useMemo<{ cp: number; mate: number | null } | null>(() => {
    if (review) return { cp: review.evals[Math.min(ply, review.evals.length - 1)].cp, mate: review.evals[Math.min(ply, review.evals.length - 1)].mate };
    if (lines[0]) return { cp: lines[0].cp, mate: lines[0].mate };
    return null;
  }, [review, lines, ply]);

  const shapes = useMemo(() => {
    const out: { orig: never; dest?: never; brush?: "green" | "blue" }[] = [];
    lines.forEach((l, i) => { if (l.first) out.push({ orig: l.first[0] as never, dest: l.first[1] as never, brush: i === 0 ? "green" : "blue" }); });
    return out;
  }, [lines]);

  return {
    engineOn, setEngineOn, engineStatus: engineState.status, engineError: engineState.status === "error" ? engineState.message : null,
    lines, review, progress, requestReview, cancelReview, hideReview, error, evalNow, shapes,
  };
}
