"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { dailyPuzzle, nextPuzzle, submitPuzzle, type PuzzleResponse } from "@/app/actions/puzzles";
import { Board } from "@/components/chess/Board";
import { BoardSettingsPanel } from "@/components/chess/BoardSettingsPanel";
import { useBoardSettings } from "@/components/chess/settings";
import { needsPromotion, toPgn, tryUci, viewAt, type GameRecord, type PromotionRole } from "@/lib/chess/game";
import { signed } from "@/lib/ratings";
import { afterOpponentOpening, judgeMove, puzzleStart, solutionMoves, type ClientPuzzle } from "@/lib/puzzles/solve";

const THEMES = [
  ["", "Any theme"], ["mateIn1", "Mate in 1"], ["mateIn2", "Mate in 2"], ["mateIn3", "Mate in 3"], ["fork", "Fork"], ["pin", "Pin"], ["skewer", "Skewer"],
  ["discoveredAttack", "Discovered attack"], ["hangingPiece", "Hanging piece"], ["backRankMate", "Back-rank mate"], ["sacrifice", "Sacrifice"],
  ["deflection", "Deflection"], ["quietMove", "Quiet move"], ["endgame", "Endgame"], ["opening", "Opening"], ["middlegame", "Middlegame"],
  ["promotion", "Promotion"], ["trappedPiece", "Trapped piece"],
] as const;
const THEME_LABEL = Object.fromEntries(THEMES.map(([id, label]) => [id, label]));
const humanTheme = (t: string) => THEME_LABEL[t] ?? t.replace(/([A-Z])/g, " $1").toLowerCase();
const ROLE_CHAR = { queen: "q", rook: "r", bishop: "b", knight: "n" } as const;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

type Phase = "loading" | "opening" | "solving" | "solved" | "revealing" | "empty";
const btn = "rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-medium hover:bg-surface-2 disabled:opacity-40";

export function PuzzleTrainer({ mode }: { mode: "train" | "daily" }) {
  const router = useRouter();
  const [settings, updateSettings] = useBoardSettings();
  const [theme, setTheme] = useState("");
  const [puzzle, setPuzzle] = useState<ClientPuzzle | null>(null);
  const [rated, setRated] = useState(false);
  const [myRating, setMyRating] = useState<number | null>(null);
  const [record, setRecord] = useState<GameRecord | null>(null);
  const [k, setK] = useState(0);
  const [phase, setPhase] = useState<Phase>("loading");
  const [failed, setFailed] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [result, setResult] = useState<{ before?: number; after?: number; repeat: boolean } | null>(null);
  const [dayInfo, setDayInfo] = useState<{ date?: string; done?: boolean } | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const played = useRef<string[]>([]);
  const submitted = useRef(false);
  const token = useRef(0); // ignore late answers from an abandoned puzzle

  const submit = useCallback(async (p: ClientPuzzle, didFail: boolean) => {
    if (submitted.current) return;
    submitted.current = true;
    const res = await submitPuzzle({ puzzleId: p.id, played: played.current, failed: didFail });
    if (res.ok) {
      setResult({ before: res.before, after: res.after, repeat: res.repeat });
      if (res.after !== undefined) setMyRating(res.after);
    }
  }, []);

  const start = useCallback(async (themeId: string) => {
    const my = ++token.current;
    setPhase("loading"); setMessage(null); setResult(null); setFailed(false); setK(0);
    played.current = []; submitted.current = false;
    let res: PuzzleResponse;
    if (mode === "daily") {
      const d = await dailyPuzzle();
      if (d.ok) setDayInfo({ date: d.date, done: d.done });
      res = d;
    } else {
      res = await nextPuzzle(themeId);
    }
    if (my !== token.current) return;
    if (!res.ok) { setMessage(res.error); setPhase("empty"); return; }
    setPuzzle(res.puzzle); setRated(res.rated); setMyRating(res.rating);
    submitted.current = !res.rated; // unrated puzzles never submit
    setRecord(puzzleStart(res.puzzle)); setPhase("opening");
    await sleep(700);
    if (my !== token.current) return;
    setRecord(afterOpponentOpening(res.puzzle)); setPhase("solving");
  }, [mode]);

  useEffect(() => {
    const t = token; // the ref object itself, so the cleanup bumps the latest value
    queueMicrotask(() => start(""));
    return () => { t.current++; };
  }, [start]);

  const view = useMemo(() => (record ? viewAt(record) : null), [record]);
  const player = view && puzzle ? (viewAt(puzzleStart(puzzle)).turn === "white" ? "black" : "white") : "white";

  const onMove = useCallback((from: string, to: string, promotion?: PromotionRole) => {
    if (phase !== "solving" || !puzzle || !record) return false;
    const uci = `${from}${to}${promotion ? ROLE_CHAR[promotion] : ""}`;
    const step = judgeMove(puzzle, k, record, uci);
    if (step.kind === "wrong") {
      setMessage("That's not the move. Try again.");
      if (!failed) { setFailed(true); submit(puzzle, true); }
      return false;
    }
    setMessage(null);
    played.current = [...played.current, step.record.moves[step.record.moves.length - 1]];
    setRecord(step.record);
    const finished = step.kind === "mate" || step.done;
    if (finished) {
      setPhase("solved");
      if (!failed) submit(puzzle, false);
      return true;
    }
    const my = token.current;
    sleep(500).then(() => {
      if (my !== token.current) return;
      const reply = tryUci(step.record, step.reply!);
      if (reply.ok) { setRecord(reply.record); setK(k + 1); }
    });
    return true;
  }, [phase, puzzle, record, k, failed, submit]);

  const isPromotion = useCallback((from: string, to: string) => (record ? needsPromotion(record, from as never, to as never) : false), [record]);

  const showSolution = async () => {
    if (!puzzle || !record || phase !== "solving") return;
    const my = token.current;
    setPhase("revealing");
    if (!failed) { setFailed(true); submit(puzzle, true); }
    let rec = record;
    const sol = solutionMoves(puzzle);
    for (let i = k; i < sol.length; i++) {
      await sleep(650);
      if (my !== token.current) return;
      const mine = tryUci(rec, sol[i]);
      if (!mine.ok) break;
      rec = mine.record; setRecord(rec);
      const reply = puzzle.moves[2 + 2 * i];
      if (reply) {
        await sleep(650);
        if (my !== token.current) return;
        const r = tryUci(rec, reply);
        if (r.ok) { rec = r.record; setRecord(rec); }
      }
    }
    setPhase("solved");
  };

  const analyse = () => {
    if (!puzzle || !record) return;
    try { window.sessionStorage.setItem("rookery.import", toPgn(record, { Event: `Puzzle ${puzzle.id}`, White: "?", Black: "?" })); } catch { /* ignore */ }
    router.push("/play/board");
  };

  const clean = phase === "solved" && !failed;
  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(0,640px)_minmax(0,1fr)]">
      <div className="grid min-w-0 grid-cols-1 content-start gap-3" data-puzzle-id={puzzle?.id} data-phase={phase}>
        {record && view ? (
          <Board view={view} variant="standard" orientation={player} movable={phase === "solving" ? player : "none"} settings={settings} isPromotion={isPromotion} onMove={onMove} />
        ) : (
          <div className="aspect-square w-full animate-pulse rounded-xl bg-surface" />
        )}
        <div className="flex flex-wrap gap-2">
          <button className={btn} onClick={() => setShowSettings((s) => !s)} aria-expanded={showSettings}>Appearance</button>
        </div>
        {showSettings && <BoardSettingsPanel settings={settings} update={updateSettings} />}
      </div>

      <div className="grid min-w-0 grid-cols-1 content-start gap-4">
        <div aria-live="polite" className={`rounded-xl border px-4 py-3 ${clean ? "border-brand bg-brand/15" : phase === "solved" ? "border-warn/60 bg-warn/10" : "border-line bg-surface"}`}>
          {phase === "loading" && <p className="font-semibold">Finding a puzzle…</p>}
          {phase === "empty" && <p className="font-semibold">{message}</p>}
          {phase === "opening" && <p className="font-semibold">Watch the opponent&apos;s move…</p>}
          {phase === "solving" && (
            <div className="grid gap-1">
              <p className="font-semibold">Your move: find the best {player === "white" ? "white" : "black"} move</p>
              <p className="text-xs text-dim">{failed ? "Counted as a miss, but keep going to finish it." : rated ? `Rated · you are ${myRating ?? "unrated"}` : "Unrated: log in to get a puzzle rating."}</p>
            </div>
          )}
          {phase === "revealing" && <p className="font-semibold">Here is the solution…</p>}
          {phase === "solved" && (
            <div className="grid gap-1">
              <p className="text-lg font-bold">{clean ? "Puzzle solved!" : "Puzzle finished"}</p>
              {result && !result.repeat && result.before !== undefined && result.after !== undefined && (
                <p className="text-sm" aria-label="Rating change">Puzzle rating {result.before} → {result.after} ({signed(result.after - result.before)})</p>
              )}
              {result?.repeat && <p className="text-sm text-dim">You have done this puzzle before, so your rating didn&apos;t change.</p>}
              {!rated && <p className="text-sm text-dim"><Link href="/login" className="font-semibold text-brand underline">Log in</Link> to rate your puzzles.</p>}
              {puzzle && <p className="text-xs text-dim">Puzzle rating {puzzle.rating} · {puzzle.themes.filter((t) => t in THEME_LABEL && t !== "").slice(0, 4).map(humanTheme).join(", ") || "tactics"}</p>}
            </div>
          )}
        </div>
        {message && phase === "solving" && <p role="status" className="rounded-md border border-warn/50 bg-warn/10 px-3 py-2 text-sm">{message}</p>}

        {mode === "train" && (
          <label className="grid max-w-xs gap-1 text-sm font-medium">Theme
            <select className="rounded-md border border-line bg-bg px-3 py-2 text-sm text-ink focus:border-brand focus:outline-none" value={theme}
              onChange={(e) => { setTheme(e.target.value); start(e.target.value); }}>
              {THEMES.map(([id, label]) => <option key={id} value={id}>{label}</option>)}
            </select>
          </label>
        )}
        {mode === "daily" && dayInfo?.date && <p className="text-sm text-dim">Daily puzzle for {dayInfo.date}. Everyone gets the same one.{dayInfo.done ? " You've already done it." : ""}</p>}

        <div className="flex flex-wrap gap-2">
          {phase === "solving" && <button className={btn} onClick={showSolution}>Show solution</button>}
          {phase === "solved" && mode === "train" && <button className="rounded-md bg-brand px-4 py-2 text-sm font-semibold text-brand-ink hover:bg-brand-hi" onClick={() => start(theme)}>Next puzzle</button>}
          {phase === "solved" && <button className={btn} onClick={analyse}>Analyse</button>}
          {mode === "train" && phase === "solving" && <button className={btn} onClick={() => start(theme)}>Skip</button>}
          <Link href="/puzzles" className={`${btn} inline-block`}>Puzzle home</Link>
        </div>
        <p className="text-xs text-dim">Puzzles come from the open Lichess puzzle database (CC0).</p>
      </div>
    </div>
  );
}
