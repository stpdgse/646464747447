"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { BotMatchBoard } from "@/components/computer/BotMatchBoard";
import { useBots } from "@/components/computer/useBots";
import { useEngine } from "@/components/computer/useEngine";
import { newRecord, tryUci, viewAt, type GameRecord } from "@/lib/chess/game";
import { pickBotMove, SPEEDS, type Speed } from "@/lib/engine/botMove";
import type { Bot } from "@/lib/engine/bots";
import { ui } from "@/lib/social";
import { createClient } from "@/lib/supabase/client";

const PLY_LIMIT = 300;
type Variant = "standard" | "chess960";
type Status = "running" | "paused" | "finished";
type Match = { id: string; white: Bot; black: Bot; variant: Variant; record: GameRecord; speed: Speed; status: Status; result: string | null; reason: string | null };
type Saved = { id: string; white_bot_id: string; black_bot_id: string; variant: Variant; initial_fen: string; moves: string[]; speed: Speed; updated_at: string };

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Pick two bots and watch them fight. This page runs both bots in your browser and publishes the game for everyone on the Watch tab. */
export function BotArena() {
  const bots = useBots();
  const engineState = useEngine();
  const engine = engineState.status === "ready" ? engineState.engine : null;
  const [setup, setSetup] = useState<{ white: string; black: string; variant: Variant; speed: Speed }>({ white: "cam", black: "bert", variant: "standard", speed: "normal" });
  const [match, setMatch] = useState<Match | null>(null);
  const [uid, setUid] = useState<string | null>(null);
  const [saved, setSaved] = useState<Saved | null>(null);
  const [error, setError] = useState<string | null>(null);
  const chain = useRef<Promise<unknown>>(Promise.resolve());
  const created = useRef(new Set<string>());

  useEffect(() => {
    (async () => {
      const supabase = createClient();
      const { data: sess } = await supabase.auth.getSession();
      const id = sess.session?.user.id ?? null;
      setUid(id);
      if (!id) return;
      const since = new Date(Date.now() - 6 * 3600_000).toISOString();
      const { data } = await supabase.from("bot_matches").select("id, white_bot_id, black_bot_id, variant, initial_fen, moves, speed, updated_at")
        .eq("creator_id", id).eq("status", "playing").gt("updated_at", since).order("updated_at", { ascending: false }).limit(1);
      setSaved((data?.[0] as Saved | undefined) ?? null);
    })().catch(() => {});
  }, []);

  // ---- publishing ----------------------------------------------------------------------------------------------
  const id = match?.id ?? null;
  const movesKey = match ? match.record.moves.join(" ") : "";
  const finishedKey = match?.status === "finished" ? `${match.result}:${match.reason}` : "";
  const speed = match?.speed ?? "normal";
  useEffect(() => {
    if (!match || !uid) return;
    const m = match;
    const supabase = createClient();
    chain.current = chain.current.then(async () => {
      try {
        if (!created.current.has(m.id)) {
          created.current.add(m.id);
          const { error: err } = await supabase.from("bot_matches").insert({
            id: m.id, creator_id: uid, white_bot_id: m.white.id, white_bot_name: m.white.name, white_bot_rating: m.white.rating,
            black_bot_id: m.black.id, black_bot_name: m.black.name, black_bot_rating: m.black.rating, variant: m.variant,
            initial_fen: m.record.initialFen, speed: m.speed, moves: m.record.moves, status: "playing",
          });
          if (err && err.code !== "23505") {
            created.current.delete(m.id);
            if (err.message.includes("row-level security")) setError("You can't start new battles right now.");
            else if (err.message.includes("slow_down")) setError("You're starting battles too fast. Wait a minute.");
            return;
          }
        }
        await supabase.from("bot_matches").update({
          moves: m.record.moves, speed: m.speed, status: m.status === "finished" ? "finished" : "playing", result: m.result, reason: m.reason,
        }).eq("id", m.id);
      } catch {
        /* publishing is a bonus: the battle keeps running locally */
      }
    });
    // `match` also carries pause state; only these fields matter to the database.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [uid, id, movesKey, finishedKey, speed]);

  // ---- running the battle --------------------------------------------------------------------------------------
  const finish = useCallback((result: string | null, reason: string) => {
    setMatch((m) => (m && m.status !== "finished" ? { ...m, status: "finished", result, reason } : m));
  }, []);

  useEffect(() => {
    if (!match || !engine || match.status !== "running") return;
    const v = viewAt(match.record);
    if (v.outcome) {
      queueMicrotask(() => finish(v.outcome!.result, v.outcome!.reason));
      return;
    }
    if (match.record.moves.length >= PLY_LIMIT) {
      queueMicrotask(() => finish("1/2-1/2", "move limit"));
      return;
    }
    let cancelled = false;
    const ac = new AbortController();
    const bot = v.turn === "white" ? match.white : match.black;
    const sp = SPEEDS[match.speed];
    const record = match.record;
    (async () => {
      const began = performance.now();
      const uci = await pickBotMove(engine, bot, record, { movetimeCap: sp.capMs, signal: ac.signal });
      if (cancelled || !uci) return;
      const wait = Math.max(0, sp.pauseMs - (performance.now() - began));
      if (wait > 0) await sleep(wait);
      if (cancelled) return;
      const next = tryUci(record, uci);
      if (!next.ok) return;
      setMatch((m) => (m && m.id === match.id && m.record.moves.length === record.moves.length && m.status === "running" ? { ...m, record: next.record } : m));
    })().catch(() => {});
    return () => {
      cancelled = true;
      ac.abort();
    };
    // The position (move count), pause state and engine decide when to think; the match object also changes for other reasons.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [match?.id, match?.record.moves.length, match?.status, match?.speed, engine]);

  const start = (whiteId: string, blackId: string, variant: Variant, speedChoice: Speed) => {
    const white = bots.find((b) => b.id === whiteId), black = bots.find((b) => b.id === blackId);
    if (!white || !black) return setError("Pick two bots.");
    setError(null);
    engine?.newGame();
    if (match && match.status !== "finished") finish(null, "replaced");
    setSaved(null);
    window.scrollTo({ top: 0 });
    setMatch({ id: crypto.randomUUID(), white, black, variant, record: newRecord(variant), speed: speedChoice, status: "running", result: null, reason: null });
  };

  const continueSaved = () => {
    if (!saved) return;
    const white = bots.find((b) => b.id === saved.white_bot_id), black = bots.find((b) => b.id === saved.black_bot_id);
    if (!white || !black) return setError("One of those bots isn't available any more.");
    const record: GameRecord = { variant: saved.variant, initialFen: saved.initial_fen, moves: saved.moves };
    try {
      viewAt(record);
    } catch {
      return setError("That battle can't be continued.");
    }
    created.current.add(saved.id);
    setSaved(null);
    setMatch({ id: saved.id, white, black, variant: saved.variant, record, speed: saved.speed, status: "running", result: null, reason: null });
  };

  const surprise = () => {
    const pool = bots.filter((b) => b.id !== setup.white);
    const w = bots[Math.floor(Math.random() * bots.length)];
    const near = pool.filter((b) => b.id !== w.id && Math.abs(b.rating - w.rating) <= 500);
    const b = (near.length ? near : pool.filter((x) => x.id !== w.id))[Math.floor(Math.random() * Math.max(1, (near.length ? near : pool).length))] ?? bots[0];
    setSetup({ ...setup, white: w.id, black: b.id });
  };

  const view = useMemo(() => (match ? viewAt(match.record) : null), [match]);

  if (!match) {
    const pick = (value: string, onChange: (v: string) => void, label: string) => (
      <label className="grid gap-1.5 text-sm font-medium">{label}
        <select className={ui.field} value={value} onChange={(e) => onChange(e.target.value)}>
          {bots.map((b) => <option key={b.id} value={b.id}>{b.avatar} {b.name} (≈{b.rating})</option>)}
        </select>
      </label>
    );
    return (
      <div className="grid max-w-2xl gap-5">
        {error && <p role="alert" className={ui.err}>{error}</p>}
        {saved && (
          <div className="flex flex-wrap items-center gap-3 rounded-xl border border-brand bg-brand/10 px-4 py-3">
            <span className="font-semibold">You have an unfinished bot battle ({Math.ceil(saved.moves.length / 2)} moves).</span>
            <button onClick={continueSaved} className={ui.primary}>Continue it</button>
          </div>
        )}
        <section className={`${ui.card} grid gap-4`}>
          <h2 className="text-xl font-bold">Pick the fighters</h2>
          <div className="grid gap-4 sm:grid-cols-2">
            {pick(setup.white, (v) => setSetup({ ...setup, white: v }), "White")}
            {pick(setup.black, (v) => setSetup({ ...setup, black: v }), "Black")}
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <label className="grid gap-1.5 text-sm font-medium">Variant
              <select className={ui.field} value={setup.variant} onChange={(e) => setSetup({ ...setup, variant: e.target.value as Variant })}>
                <option value="standard">Standard</option><option value="chess960">Chess960</option>
              </select>
            </label>
            <label className="grid gap-1.5 text-sm font-medium">Speed
              <select className={ui.field} value={setup.speed} onChange={(e) => setSetup({ ...setup, speed: e.target.value as Speed })}>
                {(Object.keys(SPEEDS) as Speed[]).map((s) => <option key={s} value={s}>{SPEEDS[s].label}</option>)}
              </select>
            </label>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <button className={ui.primary} disabled={!engine || bots.length < 2} onClick={() => start(setup.white, setup.black, setup.variant, setup.speed)}>
              {engine ? "Start the battle" : "Loading the engine…"}
            </button>
            <button className={ui.btn} onClick={surprise}>Surprise me</button>
          </div>
          <p className="text-xs text-dim">
            The battle runs in <strong>your browser</strong>, so keep this page open. Everyone can watch it live on the Watch tab
            {uid ? "." : ", but you need to log in for others to see it."} At the fastest speed strong bots think for less time, so they play a little weaker.
          </p>
        </section>
        {!uid && <p className={ui.info}><Link href="/login" className="font-semibold text-brand underline">Log in</Link> so your battle shows up on the Watch tab.</p>}
      </div>
    );
  }

  const outcome = match.status === "finished" ? (match.result === "1-0" ? `${match.white.name} wins` : match.result === "0-1" ? `${match.black.name} wins` : match.result === "1/2-1/2" ? "Draw" : "Stopped") : null;
  const turnBot = view && !view.outcome ? (view.turn === "white" ? match.white : match.black) : null;
  return (
    <div className="grid gap-4">
      {error && <p role="alert" className={ui.err}>{error}</p>}
      <BotMatchBoard
        record={match.record}
        white={{ id: match.white.id, name: match.white.name, rating: match.white.rating }}
        black={{ id: match.black.id, name: match.black.name, rating: match.black.rating }}
        bots={bots}
        status={
          match.status === "finished" ? (
            <div className="grid gap-1">
              <p className="text-lg font-bold">{outcome}</p>
              <p className="text-sm text-dim">{match.reason ? `(${match.reason})` : ""} {uid ? "Everyone who watched has seen the whole game on the Watch tab." : ""}</p>
            </div>
          ) : (
            <div className="grid gap-1">
              <p className="font-semibold">{match.status === "paused" ? "Paused" : turnBot ? `${turnBot.name} is thinking…` : "…"}</p>
              <p className="text-xs text-dim">{match.white.name} vs {match.black.name} · move {Math.floor(match.record.moves.length / 2) + 1} · keep this page open{uid ? "" : " (log in to let others watch)"}</p>
            </div>
          )
        }
        controls={
          <div className="flex flex-wrap items-center gap-2">
            {match.status === "running" && <button className={ui.btn} onClick={() => setMatch({ ...match, status: "paused" })}>Pause</button>}
            {match.status === "paused" && <button className={ui.primary} onClick={() => setMatch({ ...match, status: "running" })}>Resume</button>}
            {match.status !== "finished" && (
              <label className="flex items-center gap-2 text-sm">Speed
                <select className={ui.field} value={match.speed} onChange={(e) => setMatch({ ...match, speed: e.target.value as Speed })}>
                  {(Object.keys(SPEEDS) as Speed[]).map((s) => <option key={s} value={s}>{SPEEDS[s].label}</option>)}
                </select>
              </label>
            )}
            {match.status !== "finished" && <button className={ui.danger} onClick={() => finish(null, "stopped")}>Stop</button>}
            {match.status === "finished" && <button className={ui.primary} onClick={() => start(match.black.id, match.white.id, match.variant, match.speed)}>Rematch (swap colours)</button>}
            {match.status === "finished" && <button className={ui.btn} onClick={() => setMatch(null)}>New battle</button>}
            {uid && <Link href={`/watch/bots/${match.id}`} className={`${ui.btn} inline-block`}>Spectator view</Link>}
          </div>
        }
      />
    </div>
  );
}
