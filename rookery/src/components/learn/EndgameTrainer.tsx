"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Board } from "@/components/chess/Board";
import { BoardSettingsPanel } from "@/components/chess/BoardSettingsPanel";
import { useBoardSettings } from "@/components/chess/settings";
import { useEngine } from "@/components/computer/useEngine";
import { needsPromotion, tryMove, tryUci, viewAt, type GameRecord, type PromotionRole } from "@/lib/chess/game";
import type { Endgame } from "@/lib/lessons/lessons";
import { createClient } from "@/lib/supabase/client";

const btn = "rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-medium hover:bg-surface-2 disabled:opacity-40";
const primary = "rounded-md bg-brand px-4 py-2 text-sm font-semibold text-brand-ink hover:bg-brand-hi";
const WIN_MOVES = 50;
const DRAW_MOVES = 30;

type Status = "playing" | "success" | "fail";

/** Play a classic ending against Stockfish. Win drills: mate (or win) within 50 moves. Draw drills: hold on for 30 moves or force a draw. */
export function EndgameTrainer({ endgame, viewerId, done, next }: { endgame: Endgame; viewerId: string | null; done: boolean; next: { id: string; title: string } | null }) {
  const engineState = useEngine();
  const engine = engineState.status === "ready" ? engineState.engine : null;
  const [settings, updateSettings] = useBoardSettings();
  const [record, setRecord] = useState<GameRecord>({ variant: "standard", initialFen: endgame.fen, moves: [] });
  const [status, setStatus] = useState<Status>("playing");
  const [reason, setReason] = useState("");
  const [thinking, setThinking] = useState(false);
  const [showTip, setShowTip] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [saved, setSaved] = useState(done);
  const gen = useRef(0);

  const view = useMemo(() => viewAt(record), [record]);
  // who moves first in the starting position decides which plies are the player's
  const firstMover = useMemo(() => viewAt({ variant: "standard", initialFen: endgame.fen, moves: [] }).turn, [endgame.fen]);
  const movesPlayed = record.moves.filter((_, i) => (i % 2 === 0) === (firstMover === endgame.player)).length;

  const succeed = useCallback(async (why: string) => {
    setStatus("success");
    setReason(why);
    if (viewerId && !saved) {
      setSaved(true);
      await createClient().from("lesson_progress").insert({ user_id: viewerId, lesson_id: "endgames", item_id: endgame.id });
    }
  }, [viewerId, saved, endgame.id]);

  // Judge the position after every move.
  useEffect(() => {
    if (status !== "playing") return;
    const out = view.outcome;
    if (out) {
      const winner = out.result === "1-0" ? "white" : out.result === "0-1" ? "black" : null;
      if (endgame.goal === "win") {
        if (winner === endgame.player) queueMicrotask(() => succeed("Checkmate! You won the ending."));
        else queueMicrotask(() => { setStatus("fail"); setReason(winner ? "The opponent won." : `It's a draw (${out.reason}). You needed to win.`); });
      } else if (winner === null) queueMicrotask(() => succeed(`Drawn (${out.reason}). You held it.`));
      else queueMicrotask(() => { setStatus("fail"); setReason(winner === endgame.player ? "That's a win for you!" : "You were checkmated."); });
      return;
    }
    if (endgame.goal === "draw" && movesPlayed >= DRAW_MOVES) queueMicrotask(() => succeed(`You survived ${DRAW_MOVES} moves without losing: the ending is held.`));
    if (endgame.goal === "win" && movesPlayed >= WIN_MOVES) queueMicrotask(() => { setStatus("fail"); setReason(`Out of moves: you had ${WIN_MOVES} moves to win.`); });
  }, [view, status, endgame, movesPlayed, succeed]);

  // The engine answers.
  useEffect(() => {
    if (status !== "playing" || view.outcome || view.turn === endgame.player || !engine) return;
    const my = ++gen.current;
    const ac = new AbortController();
    queueMicrotask(() => setThinking(true));
    (async () => {
      try {
        const res = await engine.search({ fen: record.initialFen, moves: record.moves, depth: 14, movetime: 250 }, ac.signal);
        if (my !== gen.current || !res.best) return;
        const next = tryUci(record, res.best);
        if (next.ok) setRecord(next.record);
      } finally {
        if (my === gen.current) setThinking(false);
      }
    })().catch(() => {});
    return () => ac.abort();
  }, [record, engine, status, view, endgame.player]);

  const onMove = useCallback((from: string, to: string, promotion?: PromotionRole) => {
    if (status !== "playing" || view.turn !== endgame.player || thinking) return false;
    const res = tryMove(record, from as never, to as never, promotion);
    if (!res.ok) return false;
    setRecord(res.record);
    return true;
  }, [status, view.turn, endgame.player, thinking, record]);
  const isPromotion = useCallback((from: string, to: string) => needsPromotion(record, from as never, to as never), [record]);

  const reset = () => {
    gen.current++;
    setRecord({ variant: "standard", initialFen: endgame.fen, moves: [] });
    setStatus("playing"); setReason(""); setThinking(false);
  };
  const takeBack = () => {
    if (status !== "playing" || thinking || record.moves.length === 0) return;
    gen.current++;
    const cut = view.turn === endgame.player ? 2 : 1;
    setRecord({ ...record, moves: record.moves.slice(0, Math.max(0, record.moves.length - cut)) });
  };

  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(0,640px)_minmax(0,1fr)]">
      <div className="grid min-w-0 grid-cols-1 content-start gap-3" data-status={status}>
        <Board view={view} variant="standard" orientation={endgame.player} movable={status === "playing" && view.turn === endgame.player && !thinking ? endgame.player : "none"} settings={settings} isPromotion={isPromotion} onMove={onMove} />
        <div className="flex flex-wrap gap-2">
          <button className={btn} onClick={takeBack} disabled={status !== "playing" || thinking || record.moves.length === 0}>Take back</button>
          <button className={btn} onClick={reset}>Start over</button>
          <button className={btn} onClick={() => setShowSettings((s) => !s)} aria-expanded={showSettings}>Appearance</button>
        </div>
        {showSettings && <BoardSettingsPanel settings={settings} update={updateSettings} />}
      </div>
      <div className="grid min-w-0 grid-cols-1 content-start gap-4">
        <div aria-live="polite" className={`rounded-xl border px-4 py-3 ${status === "success" ? "border-brand bg-brand/15" : status === "fail" ? "border-warn/60 bg-warn/10" : "border-line bg-surface"}`}>
          <p className="text-xs font-semibold text-dim">{endgame.title}</p>
          {status === "playing" && (
            <div className="grid gap-1">
              <p className="font-semibold">{!engine ? "Loading the engine…" : thinking ? "The engine is thinking…" : view.turn === endgame.player ? "Your move" : "Waiting…"}</p>
              <p className="text-xs text-dim">
                Goal: {endgame.goal === "win" ? `win within ${WIN_MOVES} moves` : `hold the draw for ${DRAW_MOVES} moves, or force one`} · you play {endgame.player} · move {movesPlayed + (view.turn === endgame.player ? 1 : 0)}
              </p>
            </div>
          )}
          {status === "success" && <div className="grid gap-1"><p className="text-lg font-bold">Well done!</p><p className="text-sm">{reason}</p>{viewerId ? <p className="text-xs text-dim">Saved to your progress.</p> : <p className="text-xs text-dim"><Link href="/login" className="font-semibold text-brand underline">Log in</Link> to save your progress.</p>}</div>}
          {status === "fail" && <div className="grid gap-1"><p className="text-lg font-bold">Not this time</p><p className="text-sm">{reason}</p></div>}
        </div>
        <p className="text-sm">{endgame.blurb}</p>
        <button className={`${btn} justify-self-start`} onClick={() => setShowTip((s) => !s)} aria-expanded={showTip}>{showTip ? "Hide the tip" : "Show a tip"}</button>
        {showTip && <p className="rounded-xl border border-line bg-surface p-4 text-sm">{endgame.tip}</p>}
        <div className="flex flex-wrap gap-2">
          {status !== "playing" && <button className={primary} onClick={reset}>Try again</button>}
          {status === "success" && next && <Link href={`/learn/endgames/${next.id}`} className={`${primary} inline-block`}>Next: {next.title}</Link>}
          <Link href="/learn/endgames" className={`${btn} inline-block`}>All endgames</Link>
        </div>
      </div>
    </div>
  );
}
