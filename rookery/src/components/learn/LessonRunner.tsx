"use client";

import Link from "next/link";
import { useCallback, useMemo, useRef, useState } from "react";
import { Board } from "@/components/chess/Board";
import { BoardSettingsPanel } from "@/components/chess/BoardSettingsPanel";
import { useBoardSettings } from "@/components/chess/settings";
import { canonical } from "@/lib/puzzles/solve";
import { needsPromotion, tryUci, viewAt, type GameRecord, type PromotionRole } from "@/lib/chess/game";
import { afterOpponentOpening, judgeMove, puzzleStart, solutionMoves, type ClientPuzzle } from "@/lib/puzzles/solve";
import type { Step } from "@/lib/lessons/lessons";
import { createClient } from "@/lib/supabase/client";

export type LessonItem = { id: string; puzzle?: ClientPuzzle; step?: Step };
type Props = {
  lesson: { id: string; title: string; icon: string; intro: string[]; ideas: string[]; solved: string };
  items: LessonItem[];
  done: string[];
  viewerId: string | null;
  next: { id: string; title: string } | null;
};

const ROLE_CHAR = { queen: "q", rook: "r", bishop: "b", knight: "n" } as const;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const btn = "rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-medium hover:bg-surface-2 disabled:opacity-40";
const primary = "rounded-md bg-brand px-4 py-2 text-sm font-semibold text-brand-ink hover:bg-brand-hi";

type Phase = "opening" | "solving" | "solved";

/** Intro, then exercises one by one with hints; finished exercises are remembered for logged-in players. */
export function LessonRunner({ lesson, items, done: initialDone, viewerId, next }: Props) {
  const [settings, updateSettings] = useBoardSettings();
  const [doneSet, setDoneSet] = useState(() => new Set(initialDone));
  const firstOpen = items.findIndex((i) => !initialDone.includes(i.id));
  const [stage, setStage] = useState<"intro" | "practice" | "complete">("intro");
  const [index, setIndex] = useState(Math.max(0, firstOpen));
  const [record, setRecord] = useState<GameRecord | null>(null);
  const [k, setK] = useState(0);
  const [phase, setPhase] = useState<Phase>("opening");
  const [message, setMessage] = useState<string | null>(null);
  const [hintLevel, setHintLevel] = useState(0);
  const [cheated, setCheated] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const token = useRef(0);

  const item = items[index];

  const load = useCallback(async (i: number) => {
    const my = ++token.current;
    const it = items[i];
    setMessage(null); setHintLevel(0); setCheated(false); setK(0); setIndex(i);
    if (it.puzzle) {
      setRecord(puzzleStart(it.puzzle)); setPhase("opening");
      await sleep(650);
      if (my !== token.current) return;
      setRecord(afterOpponentOpening(it.puzzle)); setPhase("solving");
    } else if (it.step) {
      setRecord({ variant: "standard", initialFen: it.step.fen, moves: [] }); setPhase("solving");
    }
  }, [items]);

  const begin = () => {
    const open = items.findIndex((i) => !doneSet.has(i.id));
    if (open === -1) {
      setStage("complete");
      return;
    }
    setStage("practice");
    load(open);
  };

  const markDone = useCallback(async (id: string) => {
    setDoneSet((s) => new Set(s).add(id));
    if (!viewerId) return;
    await createClient().from("lesson_progress").insert({ user_id: viewerId, lesson_id: lesson.id, item_id: id });
  }, [viewerId, lesson.id]);

  const view = useMemo(() => (record ? viewAt(record) : null), [record]);
  const player = useMemo(() => {
    if (!item) return "white" as const;
    if (item.puzzle) return viewAt(puzzleStart(item.puzzle)).turn === "white" ? ("black" as const) : ("white" as const);
    return viewAt({ variant: "standard", initialFen: item.step!.fen, moves: [] }).turn;
  }, [item]);

  const expected = useMemo(() => {
    if (!item || !record) return null;
    if (item.puzzle) return item.puzzle.moves[1 + 2 * k] ?? null;
    const a = item.step!.accept;
    return a === "any" ? null : a[0];
  }, [item, record, k]);

  const onMove = useCallback((from: string, to: string, promotion?: PromotionRole) => {
    if (phase !== "solving" || !item || !record) return false;
    const uci = `${from}${to}${promotion ? ROLE_CHAR[promotion] : ""}`;
    if (item.puzzle) {
      const step = judgeMove(item.puzzle, k, record, uci);
      if (step.kind === "wrong") { setMessage("Not quite. Try again, or ask for a hint."); return false; }
      setMessage(null); setHintLevel(0);
      setRecord(step.record);
      if (step.kind === "mate" || step.done) {
        setPhase("solved");
        if (!cheated) markDone(item.id);
        return true;
      }
      const my = token.current;
      sleep(500).then(() => {
        if (my !== token.current) return;
        const reply = tryUci(step.record, step.reply!);
        if (reply.ok) { setRecord(reply.record); setK((x) => x + 1); }
      });
      return true;
    }
    const st = item.step!;
    const res = tryUci(record, uci);
    const good = res.ok && (st.accept === "any" || st.accept.some((a) => canonical(record, a) === res.record.moves.at(-1)));
    if (!res.ok || !good) { setMessage("Not quite. Try again, or ask for a hint."); return false; }
    setMessage(null);
    setRecord(res.record);
    setPhase("solved");
    if (!cheated) markDone(item.id);
    return true;
  }, [phase, item, record, k, cheated, markDone]);

  const isPromotion = useCallback((from: string, to: string) => (record ? needsPromotion(record, from as never, to as never) : false), [record]);

  const showSolution = async () => {
    if (!item || !record || phase !== "solving") return;
    const my = token.current;
    setCheated(true);
    if (item.step) {
      const acc = item.step.accept === "any" ? [...(view?.dests.entries() ?? [])].flatMap(([f, ts]) => [...ts].map((t) => `${f}${t}`))[0] : item.step.accept[0];
      const res = tryUci(record, acc);
      if (res.ok) setRecord(res.record);
      setPhase("solved");
      return;
    }
    let rec = record;
    const sol = solutionMoves(item.puzzle!);
    for (let i = k; i < sol.length; i++) {
      await sleep(650);
      if (my !== token.current) return;
      const mine = tryUci(rec, sol[i]);
      if (!mine.ok) break;
      rec = mine.record; setRecord(rec);
      const reply = item.puzzle!.moves[2 + 2 * i];
      if (reply) {
        await sleep(650);
        if (my !== token.current) return;
        const r = tryUci(rec, reply);
        if (r.ok) { rec = r.record; setRecord(rec); }
      }
    }
    setPhase("solved");
  };

  const goNext = () => {
    const open = items.findIndex((i, n) => n > index && !doneSet.has(i.id));
    const any = open !== -1 ? open : items.findIndex((i) => !doneSet.has(i.id));
    if (any === -1) { setStage("complete"); return; }
    load(any);
  };

  const shapes = useMemo(() => {
    if (phase !== "solving" || !expected || hintLevel === 0) return undefined;
    const orig = expected.slice(0, 2) as never;
    return hintLevel === 1 ? [{ orig, brush: "yellow" as const }] : [{ orig, dest: expected.slice(2, 4) as never, brush: "green" as const }];
  }, [phase, expected, hintLevel]);

  const total = items.length;
  const doneCount = items.filter((i) => doneSet.has(i.id)).length;

  if (stage === "intro" || stage === "complete") {
    return (
      <div className="grid max-w-2xl gap-5">
        <div className="flex items-center gap-3">
          <span aria-hidden className="text-5xl">{lesson.icon}</span>
          <div>
            <h2 className="text-2xl font-extrabold">{stage === "complete" ? "Lesson complete!" : lesson.title}</h2>
            <p className="text-sm text-dim">{doneCount} of {total} exercises done{!viewerId ? " · log in to save your progress" : ""}</p>
          </div>
        </div>
        {stage === "intro" ? (
          <>
            {lesson.intro.map((p, i) => <p key={i} className="text-base leading-relaxed">{p}</p>)}
            <div className="rounded-xl border border-line bg-surface p-4">
              <h3 className="font-bold">Key ideas</h3>
              <ul className="mt-2 grid list-disc gap-1 pl-5 text-sm">{lesson.ideas.map((i) => <li key={i}>{i}</li>)}</ul>
            </div>
            <div className="flex flex-wrap gap-3">
              <button className={primary} onClick={begin}>{doneCount === 0 ? "Start the exercises" : doneCount >= total ? "Practise again" : "Continue"}</button>
              <Link href="/learn" className={`${btn} inline-block`}>All lessons</Link>
            </div>
          </>
        ) : (
          <>
            <p>You finished every exercise in &ldquo;{lesson.title}&rdquo;. Nicely done.</p>
            <div className="flex flex-wrap gap-3">
              {next && <Link href={`/learn/lessons/${next.id}`} className={`${primary} inline-block`}>Next lesson: {next.title}</Link>}
              <Link href="/learn" className={`${btn} inline-block`}>All lessons</Link>
              <button className={btn} onClick={() => { setStage("practice"); load(0); }}>Practise again</button>
            </div>
          </>
        )}
      </div>
    );
  }

  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(0,640px)_minmax(0,1fr)]">
      <div className="grid min-w-0 grid-cols-1 content-start gap-3" data-phase={phase} data-item={item?.id}>
        {record && view ? (
          <Board view={view} variant="standard" orientation={player} movable={phase === "solving" ? player : "none"} settings={settings} isPromotion={isPromotion} onMove={onMove} shapes={shapes} />
        ) : (
          <div className="aspect-square w-full animate-pulse rounded-xl bg-surface" />
        )}
        <div className="flex flex-wrap gap-2">
          <button className={btn} onClick={() => setShowSettings((s) => !s)} aria-expanded={showSettings}>Appearance</button>
        </div>
        {showSettings && <BoardSettingsPanel settings={settings} update={updateSettings} />}
      </div>
      <div className="grid min-w-0 grid-cols-1 content-start gap-4">
        <div className="flex items-center gap-2" aria-label="Progress">
          <div className="h-2 flex-1 overflow-hidden rounded-full bg-surface-2" role="progressbar" aria-valuemin={0} aria-valuemax={total} aria-valuenow={doneCount}>
            <div className="h-full bg-brand" style={{ width: `${(doneCount / total) * 100}%` }} />
          </div>
          <span className="text-xs text-dim">{doneCount}/{total}</span>
        </div>
        <div aria-live="polite" className={`rounded-xl border px-4 py-3 ${phase === "solved" ? "border-brand bg-brand/15" : "border-line bg-surface"}`}>
          <p className="text-xs font-semibold text-dim">{lesson.title} · exercise {index + 1} of {total}</p>
          {phase === "opening" && <p className="font-semibold">Watch the opponent&apos;s move…</p>}
          {phase === "solving" && (
            <p className="font-semibold">{item?.step ? item.step.prompt : `Your move: find the best ${player} move`}</p>
          )}
          {phase === "solved" && (
            <div className="grid gap-1">
              <p className="text-lg font-bold">{cheated ? "Here is the answer" : "Correct!"}</p>
              <p className="text-sm">{item?.step ? item.step.explain : lesson.solved}</p>
              {cheated && <p className="text-xs text-dim">This one doesn&apos;t count as done. Try it again later.</p>}
            </div>
          )}
        </div>
        {message && phase === "solving" && <p role="status" className="rounded-md border border-warn/50 bg-warn/10 px-3 py-2 text-sm">{message}</p>}
        {phase === "solving" && hintLevel > 0 && item?.step && <p className="text-sm text-dim">Hint: {item.step.hint}</p>}
        <div className="flex flex-wrap gap-2">
          {phase === "solving" && <button className={btn} onClick={() => setHintLevel((h) => Math.min(2, h + 1))} disabled={!expected}>{hintLevel === 0 ? "Hint" : "Show the move"}</button>}
          {phase === "solving" && <button className={btn} onClick={showSolution}>Show answer</button>}
          {phase === "solved" && <button className={primary} onClick={goNext}>{doneCount >= total ? "Finish" : "Next exercise"}</button>}
          {phase === "solved" && cheated && <button className={btn} onClick={() => load(index)}>Try again</button>}
          <Link href="/learn" className={`${btn} inline-block`}>All lessons</Link>
        </div>
      </div>
    </div>
  );
}
