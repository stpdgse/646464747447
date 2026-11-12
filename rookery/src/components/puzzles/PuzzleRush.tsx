"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { finishRush, moreRushPuzzles, startRush } from "@/app/actions/puzzles";
import { Board } from "@/components/chess/Board";
import { useBoardSettings } from "@/components/chess/settings";
import { needsPromotion, tryUci, viewAt, type GameRecord, type PromotionRole } from "@/lib/chess/game";
import { afterOpponentOpening, judgeMove, puzzleStart, type ClientPuzzle } from "@/lib/puzzles/solve";

const RUN_MS = 180_000;
const MAX_STRIKES = 3;
const ROLE_CHAR = { queen: "q", rook: "r", bishop: "b", knight: "n" } as const;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const clock = (ms: number) => `${Math.floor(Math.max(0, ms) / 60000)}:${String(Math.ceil(Math.max(0, ms) / 1000) % 60).padStart(2, "0")}`;

type Phase = "idle" | "starting" | "running" | "over";
type Outcome = { kind: "saved"; best: number } | { kind: "rejected" } | { kind: "error"; text: string } | { kind: "anonymous" };

export function PuzzleRush({ signedIn, best }: { signedIn: boolean; best: number | null }) {
  const [settings] = useBoardSettings();
  const [phase, setPhase] = useState<Phase>("idle");
  const [score, setScore] = useState(0);
  const [strikes, setStrikes] = useState(0);
  const [left, setLeft] = useState(RUN_MS);
  const [record, setRecord] = useState<GameRecord | null>(null);
  const [puzzle, setPuzzle] = useState<ClientPuzzle | null>(null);
  const [k, setK] = useState(0);
  const [flash, setFlash] = useState<"good" | "bad" | null>(null);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  const [error, setError] = useState<string | null>(null);

  const queue = useRef<ClientPuzzle[]>([]);
  const seen = useRef<string[]>([]);
  const session = useRef<string | null>(null);
  const endsAt = useRef(0);
  const run = useRef(0); // identifies the current run; stale callbacks do nothing
  const live = useRef({ score: 0, strikes: 0 });
  const fetching = useRef(false);

  const loadNext = useCallback(async (my: number) => {
    const p = queue.current.shift();
    if (!p) return;
    seen.current.push(p.id);
    setPuzzle(p); setK(0);
    setRecord(puzzleStart(p));
    await sleep(250);
    if (my !== run.current) return;
    setRecord(afterOpponentOpening(p));
    if (queue.current.length < 8 && !fetching.current) {
      fetching.current = true;
      const lastRating = p.rating;
      moreRushPuzzles(lastRating, seen.current.slice(-300)).then((r) => { if (r.ok && my === run.current) queue.current.push(...r.puzzles); }).finally(() => { fetching.current = false; });
    }
  }, []);

  const end = useCallback(async (my: number) => {
    if (my !== run.current) return;
    run.current++;
    setPhase("over");
    setRecord(null);
    const finalScore = live.current.score;
    if (!signedIn || !session.current) { setOutcome({ kind: "anonymous" }); return; }
    const res = await finishRush(session.current, finalScore);
    if (!res.ok) setOutcome({ kind: "error", text: res.error });
    else if ("rejected" in res) setOutcome({ kind: "rejected" });
    else setOutcome({ kind: "saved", best: res.best });
  }, [signedIn]);

  const begin = async () => {
    setError(null); setOutcome(null); setPhase("starting");
    let puzzles: ClientPuzzle[] = [];
    if (signedIn) {
      const res = await startRush();
      if (!res.ok) { setError(res.error); setPhase("idle"); return; }
      session.current = res.sessionId; puzzles = res.puzzles;
    } else {
      const res = await moreRushPuzzles(500, []);
      if (!res.ok) { setError(res.error); setPhase("idle"); return; }
      session.current = null; puzzles = res.puzzles;
    }
    const my = ++run.current;
    queue.current = puzzles; seen.current = [];
    live.current = { score: 0, strikes: 0 };
    setScore(0); setStrikes(0); setLeft(RUN_MS);
    endsAt.current = Date.now() + RUN_MS;
    setPhase("running");
    await loadNext(my);
  };

  useEffect(() => {
    if (phase !== "running") return;
    const my = run.current;
    const id = setInterval(() => {
      const remaining = endsAt.current - Date.now();
      setLeft(remaining);
      if (remaining <= 0) end(my);
    }, 100);
    return () => clearInterval(id);
  }, [phase, end]);

  useEffect(() => () => { run.current++; }, []);

  const view = useMemo(() => (record ? viewAt(record) : null), [record]);
  const player = puzzle ? (viewAt(puzzleStart(puzzle)).turn === "white" ? "black" : "white") : "white";

  const onMove = useCallback((from: string, to: string, promotion?: PromotionRole) => {
    if (phase !== "running" || !puzzle || !record || flash) return false;
    const my = run.current;
    const step = judgeMove(puzzle, k, record, `${from}${to}${promotion ? ROLE_CHAR[promotion] : ""}`);
    if (step.kind === "wrong") {
      live.current.strikes++;
      setStrikes(live.current.strikes);
      setFlash("bad");
      sleep(600).then(() => {
        if (my !== run.current) return;
        setFlash(null);
        if (live.current.strikes >= MAX_STRIKES) end(my); else loadNext(my);
      });
      return false;
    }
    setRecord(step.record);
    if (step.kind === "mate" || step.done) {
      live.current.score++;
      setScore(live.current.score);
      setFlash("good");
      sleep(350).then(() => { if (my !== run.current) return; setFlash(null); loadNext(my); });
      return true;
    }
    sleep(350).then(() => {
      if (my !== run.current) return;
      const reply = tryUci(step.record, step.reply!);
      if (reply.ok) { setRecord(reply.record); setK((x) => x + 1); }
    });
    return true;
  }, [phase, puzzle, record, k, flash, end, loadNext]);

  const isPromotion = useCallback((from: string, to: string) => (record ? needsPromotion(record, from as never, to as never) : false), [record]);

  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(0,640px)_minmax(0,1fr)]">
      <div className="grid min-w-0 grid-cols-1 content-start gap-3" data-puzzle-id={phase === "running" ? puzzle?.id : undefined}>
        <div className={`rounded-xl ${flash === "good" ? "ring-4 ring-brand" : flash === "bad" ? "ring-4 ring-danger" : ""}`}>
          {record && view && phase === "running" ? (
            <Board view={view} variant="standard" orientation={player} movable={flash ? "none" : player} settings={settings} isPromotion={isPromotion} onMove={onMove} />
          ) : (
            <div className="grid aspect-square w-full place-items-center rounded-xl border border-line bg-surface p-6 text-center">
              {phase === "over" ? (
                <div className="grid gap-2">
                  <p className="text-sm font-semibold text-dim">Time&apos;s up</p>
                  <p className="text-6xl font-extrabold tabular-nums">{score}</p>
                  <p className="text-dim">{score === 1 ? "puzzle solved" : "puzzles solved"}</p>
                </div>
              ) : (
                <div className="grid max-w-sm gap-3">
                  <p className="text-2xl font-extrabold">Puzzle Rush</p>
                  <p className="text-dim">Solve as many puzzles as you can in 3 minutes. They get harder as you go. Three mistakes and you&apos;re out.</p>
                  {best !== null && <p className="text-sm text-dim">Your best: <strong className="text-ink">{best}</strong></p>}
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      <div className="grid min-w-0 grid-cols-1 content-start gap-4">
        <div className="grid grid-cols-3 gap-3 text-center" aria-live="polite">
          <div className="rounded-xl border border-line bg-surface p-3"><p className="text-xs font-semibold text-dim">Time</p><p className={`font-mono text-2xl font-bold tabular-nums ${phase === "running" && left < 20_000 ? "text-danger" : ""}`}>{clock(phase === "idle" || phase === "starting" ? RUN_MS : left)}</p></div>
          <div className="rounded-xl border border-line bg-surface p-3"><p className="text-xs font-semibold text-dim">Solved</p><p className="text-2xl font-bold tabular-nums">{score}</p></div>
          <div className="rounded-xl border border-line bg-surface p-3"><p className="text-xs font-semibold text-dim">Strikes</p><p className="text-2xl font-bold" aria-label={`${strikes} of ${MAX_STRIKES} strikes`}>{Array.from({ length: MAX_STRIKES }, (_, i) => (i < strikes ? "✗" : "·")).join(" ")}</p></div>
        </div>

        {error && <p role="alert" className="rounded-md border border-danger/50 bg-danger/10 px-3 py-2 text-sm text-danger">{error}</p>}

        {(phase === "idle" || phase === "over") && (
          <div className="grid gap-3">
            {phase === "over" && outcome && (
              <p role="status" className="rounded-xl border border-brand bg-brand/10 px-4 py-3 text-sm">
                {outcome.kind === "saved" && <>Score saved. Your best is <strong>{outcome.best}</strong>.</>}
                {outcome.kind === "anonymous" && <><Link href="/login" className="font-semibold text-brand hover:underline">Log in</Link> to save scores to the leaderboard.</>}
                {outcome.kind === "rejected" && <>That run could not be verified, so it wasn&apos;t saved.</>}
                {outcome.kind === "error" && <>{outcome.text}</>}
              </p>
            )}
            <button className="justify-self-start rounded-md bg-brand px-5 py-2.5 font-semibold text-brand-ink hover:bg-brand-hi" onClick={begin}>{phase === "over" ? "Play again" : "Start"}</button>
            {!signedIn && <p className="text-xs text-dim">You can play without an account, but only logged-in players get on the leaderboard.</p>}
          </div>
        )}
        {phase === "starting" && <p className="font-semibold">Getting ready…</p>}
        {phase === "running" && <p className="text-sm text-dim">Find the best move for {player === "white" ? "White" : "Black"}. Mistakes cost a strike.</p>}
        <div className="flex gap-2"><Link href="/puzzles" className="rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-medium hover:bg-surface-2">Puzzle home</Link><Link href="/leaderboard" className="rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-medium hover:bg-surface-2">Leaderboard</Link></div>
      </div>
    </div>
  );
}
