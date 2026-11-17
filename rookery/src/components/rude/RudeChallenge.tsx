"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { reportRudeGame } from "@/app/actions/rude";
import { Board } from "@/components/chess/Board";
import { BoardSettingsPanel } from "@/components/chess/BoardSettingsPanel";
import { ClockBar } from "@/components/chess/ClockBar";
import { MoveList } from "@/components/chess/MoveList";
import { useBoardSettings } from "@/components/chess/settings";
import { useBotBroadcast } from "@/components/computer/useBotBroadcast";
import { useEngine } from "@/components/computer/useEngine";
import { useMoveSounds } from "@/components/useMoveSounds";
import { needsPromotion, newRecord, tryMove, viewAt, type GameRecord, type PromotionRole } from "@/lib/chess/game";
import { pickBotMove } from "@/lib/engine/botMove";
import { announceProgress } from "@/lib/progress";
import { EVENT_KEY, LEVELS, MAX_ATTEMPTS, PLY_LIMIT, TOP_LEVEL, colorForLevel, hueFor, rudeBot } from "@/lib/rude";
import { ui } from "@/lib/social";
import { createClient } from "@/lib/supabase/client";
import { tryUci } from "@/lib/chess/game";

export type Attempt = { id: string; attempt_no: number; level: number; best_level: number; cleared: boolean; status: "playing" | "finished" };
type Save = { attemptId: string; level: number; gameId: string; moves: string[] };
type Phase = "lobby" | "playing" | "between" | "over";
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const EMPTY: string[] = [];

/** The ladder, the attempt counter and the game itself. */
export function RudeChallenge({ phase: eventPhase, viewerId, initialAttempts }: { phase: "upcoming" | "live" | "ended"; viewerId: string | null; initialAttempts: Attempt[] }) {
  const [attempts, setAttempts] = useState(initialAttempts);
  const [phase, setPhase] = useState<Phase>("lobby");
  const [attempt, setAttempt] = useState<Attempt | null>(null);
  const [level, setLevel] = useState(100);
  const [record, setRecord] = useState<GameRecord>(() => newRecord("standard"));
  const [gameId, setGameId] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [thinking, setThinking] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [settings, updateSettings] = useBoardSettings();
  const reported = useRef(false);
  const gameTop = useRef<HTMLDivElement>(null);
  const engineState = useEngine(phase === "playing");
  const engine = engineState.status === "ready" ? engineState.engine : null;

  const saveKey = `rookery.rude.v1:${viewerId ?? "anon"}`;
  const color = colorForLevel(level);
  const bot = useMemo(() => rudeBot(level), [level]);
  const view = useMemo(() => viewAt(record), [record]);
  const open = attempts.find((a) => a.status === "playing") ?? null;
  const used = attempts.length;
  const left = MAX_ATTEMPTS - used;
  const best = attempts.reduce((m, a) => Math.max(m, a.best_level), 0);
  const playing = phase === "playing";

  useMoveSounds(view.sans.length ? view.sans : EMPTY, phase === "over" || phase === "between");

  // ---- publishing the game so everyone can watch it on the Watch tab ----------------------------------------------
  useBotBroadcast(
    playing || phase === "between" || phase === "over"
      ? { id: gameId, botId: bot.id, botName: bot.name, botRating: level, player: color, variant: "standard", initialFen: record.initialFen, moves: record.moves, watchable: true, timeControl: "unlimited",
          end: view.outcome ? { result: view.outcome.result, how: view.outcome.reason }
            : record.moves.length >= PLY_LIMIT ? { result: "1/2-1/2", how: "move limit" }
            : phase === "over" || phase === "between" ? { result: color === "white" ? "0-1" : "1-0", how: "resignation" } : null }
      : null,
    !!gameId && !!viewerId,
  );

  // ---- saving, so a closed tab or lost connection can carry on ----------------------------------------------------
  useEffect(() => {
    if (!playing || !attempt) return;
    try {
      window.localStorage.setItem(saveKey, JSON.stringify({ attemptId: attempt.id, level, gameId, moves: record.moves } satisfies Save));
    } catch {
      /* storage blocked: the game still works */
    }
  }, [playing, attempt, level, gameId, record.moves, saveKey]);

  // keep the attempt alive during a long game
  useEffect(() => {
    if (!playing || !attempt) return;
    const timer = setInterval(() => { void createClient().rpc("rude_beat", { p_attempt: attempt.id }); }, 60_000);
    return () => clearInterval(timer);
  }, [playing, attempt]);

  const begin = useCallback((a: Attempt, lvl: number, restore?: Save | null) => {
    reported.current = false;
    setError(null); setMessage(null); setThinking(false);
    setAttempt(a); setLevel(lvl);
    const base = newRecord("standard");
    let rec = base;
    if (restore && restore.attemptId === a.id && restore.level === lvl) {
      for (const m of restore.moves) {
        const r = tryUci(rec, m);
        if (!r.ok) { rec = base; break; }
        rec = r.record;
      }
    }
    setRecord(rec);
    setGameId(restore && rec !== base ? restore.gameId : crypto.randomUUID());
    setPhase("playing");
    // bring the board into view (the holiday banner above it is tall)
    setTimeout(() => gameTop.current?.scrollIntoView({ block: "start" }), 60);
  }, []);

  const start = async () => {
    if (!viewerId) return;
    setBusy(true); setError(null);
    const { data, error: err } = await createClient().rpc("rude_start", { p_event: EVENT_KEY });
    setBusy(false);
    if (err || !data) {
      const m = err?.message ?? "";
      return setError(m.includes("no_attempts") ? "You have used all 8 attempts." : m.includes("event_over") ? "The event is over." : m.includes("not_started") ? "The event hasn't started yet." : m.includes("banned") ? "Your account is suspended." : "Could not start. Please try again.");
    }
    const a = data as Attempt;
    setAttempts((cur) => (cur.some((x) => x.id === a.id) ? cur.map((x) => (x.id === a.id ? a : x)) : [...cur, a]));
    let saved: Save | null = null;
    try { saved = JSON.parse(window.localStorage.getItem(saveKey) ?? "null") as Save | null; } catch { /* ignore */ }
    begin(a, a.level, saved);
  };

  // ---- the Rüde's moves ------------------------------------------------------------------------------------------
  useEffect(() => {
    if (!playing || !engine || view.outcome || view.turn === color || record.moves.length >= PLY_LIMIT) return;
    let cancelled = false;
    const ac = new AbortController();
    queueMicrotask(() => setThinking(true));
    (async () => {
      const began = performance.now();
      const uci = await pickBotMove(engine, bot, record, { signal: ac.signal });
      if (cancelled || !uci) return;
      const wait = Math.max(0, 550 - (performance.now() - began));
      if (wait > 0) await sleep(wait);
      if (cancelled) return;
      const next = tryUci(record, uci);
      if (next.ok) setRecord(next.record);
    })().catch(() => {}).finally(() => { if (!cancelled) setThinking(false); });
    return () => {
      cancelled = true;
      ac.abort();
    };
  }, [playing, engine, view, color, bot, record]);

  const onMove = useCallback((from: string, to: string, promotion?: PromotionRole) => {
    if (!playing || view.outcome || view.turn !== color || thinking) return false;
    const res = tryMove(record, from as never, to as never, promotion);
    if (!res.ok) return false;
    setRecord(res.record);
    return true;
  }, [playing, view, color, thinking, record]);
  const isPromotion = useCallback((from: string, to: string) => needsPromotion(record, from as never, to as never), [record]);

  // ---- the end of a game: the server checks it, then moves you up (or ends the attempt) ---------------------------------
  const finish = useCallback(async (how: "finished" | "resign" | "limit") => {
    if (!attempt || reported.current) return;
    reported.current = true;
    setBusy(true);
    const res = await reportRudeGame({ attemptId: attempt.id, level, color, moves: record.moves, how });
    setBusy(false);
    if (!res.ok) { reported.current = false; setError(res.error); return; }
    try { window.localStorage.removeItem(saveKey); } catch { /* ignore */ }
    const updated: Attempt = { ...attempt, level: res.level, best_level: res.best, status: res.status, cleared: res.cleared };
    setAttempt(updated);
    setAttempts((cur) => cur.map((x) => (x.id === updated.id ? updated : x)));
    announceProgress();
    if (res.outcome === "win" && res.status === "playing") { setPhase("between"); setMessage(`You beat Rüde ${level}!`); }
    else if (res.outcome === "win") { setPhase("over"); setMessage(`You beat all 20 Rüden. You are the Rüdenmeister!`); }
    else { setPhase("over"); setMessage(res.outcome === "draw" ? `A draw is not enough: you must win. Rüde ${level} survives.` : how === "resign" ? `You resigned against Rüde ${level}.` : `Rüde ${level} beat you.`); }
  }, [attempt, level, color, record.moves, saveKey]);

  useEffect(() => {
    if (!playing) return;
    if (view.outcome) queueMicrotask(() => finish("finished"));
    else if (record.moves.length >= PLY_LIMIT) queueMicrotask(() => finish("limit"));
  }, [playing, view.outcome, record.moves.length, finish]);

  const resign = () => finish("resign");

  const ladder = (
    <ol aria-label="The Rüden" className="flex flex-wrap gap-1.5">
      {LEVELS.map((l) => {
        const cleared = (attempt ? attempt.best_level : best) >= l;
        const current = attempt && attempt.status === "playing" && attempt.level === l;
        return (
          <li key={l} className={`grid h-9 min-w-[3.2rem] place-items-center rounded-md border px-1.5 text-xs font-bold tabular-nums ${current ? "border-brand bg-brand text-brand-ink" : cleared ? "border-brand bg-brand/15 text-ink" : "border-line bg-surface text-dim"}`}
            aria-label={`Rüde ${l}${cleared ? " beaten" : current ? " current" : ""}`}>
            {cleared ? "✓ " : ""}{l}
          </li>
        );
      })}
    </ol>
  );

  if (phase === "lobby") {
    return (
      <div className="grid gap-5">
        {error && <p role="alert" className={ui.err}>{error}</p>}
        <div className={`${ui.card} grid gap-4`}>
          <div className="flex flex-wrap items-baseline justify-between gap-2">
            <h2 className="text-xl font-bold">The ladder</h2>
            <p className="text-sm text-dim">Attempts used: <strong className="text-ink">{used}</strong> of {MAX_ATTEMPTS}{best > 0 ? <> · your best: <strong className="text-ink">Rüde {best}</strong></> : null}</p>
          </div>
          {ladder}
          {!viewerId ? (
            <p className={ui.info}><Link href="/login" className="font-semibold text-brand underline">Log in</Link> to take on the Rüden. Anyone can watch.</p>
          ) : eventPhase !== "live" ? (
            <p className={ui.info}>{eventPhase === "upcoming" ? "The challenge hasn't started yet." : "The challenge is over. Thanks for playing!"}</p>
          ) : open ? (
            <div className="flex flex-wrap items-center gap-3">
              <button disabled={busy} onClick={start} className={ui.primary}>Continue attempt {open.attempt_no}: face Rüde {open.level}</button>
              <span className="text-xs text-dim">Attempts you leave alone for 30 minutes end by themselves.</span>
            </div>
          ) : left > 0 ? (
            <div className="flex flex-wrap items-center gap-3">
              <button disabled={busy} onClick={start} className={ui.primary}>{used === 0 ? "Start your first attempt" : `Start attempt ${used + 1} of ${MAX_ATTEMPTS}`}</button>
              <span className="text-xs text-dim">{left} attempt{left === 1 ? "" : "s"} left</span>
            </div>
          ) : (
            <p className={ui.info}>You&apos;ve used all {MAX_ATTEMPTS} attempts. Your best is Rüde {best || "none"}. Watch the others on the leaderboard!</p>
          )}
        </div>
        {attempts.length > 0 && (
          <div className={`${ui.card} grid gap-2`}>
            <h3 className="font-bold">Your attempts</h3>
            <ul className="grid gap-1 text-sm">
              {attempts.map((a) => (
                <li key={a.id} className="flex justify-between gap-3"><span>Attempt {a.attempt_no}</span><span className="text-dim">{a.cleared ? "Cleared everything!" : a.best_level > 0 ? `reached Rüde ${a.best_level}` : "no Rüde beaten"}{a.status === "playing" ? " (in progress)" : ""}</span></li>
              ))}
            </ul>
          </div>
        )}
      </div>
    );
  }

  return (
    <div ref={gameTop} className="grid scroll-mt-16 gap-4">
      {error && <p role="alert" className={ui.err}>{error}</p>}
      <div className={`${ui.card} grid gap-3`}>
        <p className="text-sm text-dim">Attempt {attempt?.attempt_no} of {MAX_ATTEMPTS} · you play {color} against Rüde {level}</p>
        {ladder}
      </div>
      <div className="grid gap-6 lg:grid-cols-[minmax(0,640px)_minmax(0,1fr)]">
        <div className="grid min-w-0 grid-cols-1 content-start gap-3" data-phase={phase} data-level={level} data-moves={record.moves.join(" ")} data-color={color}>
          <div style={{ filter: `hue-rotate(${hueFor(level)}deg)` }}>
            <ClockBar label={`Rüde ${level} (≈${level})`} avatar="🐺" avatarImage="/bots/ruedenrache.png" ms={0} active={playing && view.turn !== color && !view.outcome} timed={false} thinking={thinking} />
          </div>
          <Board view={view} variant="standard" orientation={color} movable={playing && view.turn === color && !thinking && !view.outcome ? color : "none"} settings={settings} isPromotion={isPromotion} onMove={onMove} />
          <ClockBar label="You" ms={0} active={playing && view.turn === color && !view.outcome} timed={false} />
          <div className="flex flex-wrap gap-2">
            {playing && <button className={ui.danger} onClick={resign} disabled={busy}>Resign (ends this attempt)</button>}
            <button className={ui.btn} onClick={() => setShowSettings((s) => !s)} aria-expanded={showSettings}>Appearance</button>
          </div>
          {showSettings && <BoardSettingsPanel settings={settings} update={updateSettings} />}
        </div>
        <div className="grid min-w-0 grid-cols-1 content-start gap-4">
          <div aria-live="polite" className={`rounded-xl border px-4 py-3 ${phase === "between" ? "border-brand bg-brand/15" : phase === "over" ? "border-warn/60 bg-warn/10" : "border-line bg-surface"}`}>
            {playing && (
              <div className="grid gap-1">
                <p className="font-semibold">{!engine ? "The wolf is waking up…" : thinking ? `Rüde ${level} is thinking…` : view.turn === color ? "Your move" : "…"}</p>
                <p className="text-xs text-dim">Win to move up. A draw or a loss ends the attempt. {bot.greeting}</p>
              </div>
            )}
            {phase === "between" && (
              <div className="grid gap-2">
                <p className="text-lg font-bold">{message}</p>
                <p className="text-sm text-dim">Next: Rüde {attempt?.level}.</p>
                <button onClick={() => attempt && begin(attempt, attempt.level, null)} className={`${ui.primary} justify-self-start`}>Face Rüde {attempt?.level}</button>
              </div>
            )}
            {phase === "over" && (
              <div className="grid gap-2">
                <p className="text-lg font-bold">{attempt?.cleared ? "🏔️ Rüdenmeister!" : "Attempt over"}</p>
                <p className="text-sm">{message}</p>
                <p className="text-sm text-dim">This attempt: {attempt && attempt.best_level > 0 ? `Rüde ${attempt.best_level}` : "no Rüde beaten"} · attempts left: {left}</p>
                <button onClick={() => { setPhase("lobby"); setError(null); }} className={`${ui.primary} justify-self-start`}>Back to the ladder</button>
              </div>
            )}
          </div>
          <section className="rounded-xl border border-line bg-surface" aria-label="Move list"><MoveList sans={view.sans} ply={view.sans.length} /></section>
          <p className="text-xs text-dim">{viewerId ? "Everyone can watch this game live on the Watch tab." : ""} Top rung: Rüde {TOP_LEVEL}.</p>
        </div>
      </div>
    </div>
  );
}
