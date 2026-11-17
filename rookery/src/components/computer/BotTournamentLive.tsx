"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useState } from "react";
import { BotBracket } from "@/components/computer/BotBracket";
import { BotMatchBoard } from "@/components/computer/BotMatchBoard";
import { useBots } from "@/components/computer/useBots";
import { useEngine } from "@/components/computer/useEngine";
import { useTournamentRunner } from "@/components/computer/useTournamentRunner";
import { champion, type Spec } from "@/lib/botTournament";
import { GAME_SELECT, RUNNER_TIMEOUT_MS, TOURNAMENT_SELECT, type BotTournamentRow, type StoredGame } from "@/lib/botTournamentRows";
import { SPEEDS } from "@/lib/engine/botMove";
import { fmtDate, ui } from "@/lib/social";
import { createClient } from "@/lib/supabase/client";

/** One bot tournament: live board, bracket or league table, champion, and the button that lets a viewer run it. */
export function BotTournamentLive({ initial, initialGames, viewerId, isMod }: { initial: BotTournamentRow; initialGames: StoredGame[]; viewerId: string | null; isMod: boolean }) {
  const router = useRouter();
  const bots = useBots();
  const [t, setT] = useState(initial);
  const [games, setGames] = useState(initialGames);
  const [selected, setSelected] = useState<string | null>(null);
  const [claimed, setClaimed] = useState(false);
  const [now, setNow] = useState(0);
  const [message, setMessage] = useState<string | null>(null);

  const spec: Spec = useMemo(() => ({ format: t.format, bestOf: t.best_of, bots: t.bots }), [t.format, t.best_of, t.bots]);
  const runningHere = claimed && t.status === "running";
  const engineState = useEngine(runningHere);
  const engine = engineState.status === "ready" ? engineState.engine : null;
  const runner = useTournamentRunner({ tournament: t, bots, engine, active: runningHere, onGames: setGames });

  const load = useCallback(async () => {
    setNow(Date.now());
    const supabase = createClient();
    const [a, b] = await Promise.all([
      supabase.from("bot_tournaments").select(TOURNAMENT_SELECT).eq("id", initial.id).maybeSingle(),
      supabase.from("bot_tournament_games").select(GAME_SELECT).eq("tournament_id", initial.id).order("created_at", { ascending: true }),
    ]);
    if (a.data) setT(a.data as unknown as BotTournamentRow);
    if (b.data) setGames((cur) => ((b.data as StoredGame[]).length >= cur.length || runner.lost ? (b.data as StoredGame[]) : cur));
  }, [initial.id, runner.lost]);

  useEffect(() => {
    queueMicrotask(load);
    const supabase = createClient();
    const channel = supabase.channel(`bottournament-${initial.id}`)
      .on("postgres_changes", { event: "*", schema: "public", table: "bot_tournament_games", filter: `tournament_id=eq.${initial.id}` }, () => load())
      .on("postgres_changes", { event: "UPDATE", schema: "public", table: "bot_tournaments", filter: `id=eq.${initial.id}` }, () => load())
      .subscribe();
    const timer = setInterval(() => document.visibilityState === "visible" && load(), 2500);
    const tick = setInterval(() => setNow(Date.now()), 1000);
    return () => {
      clearInterval(timer); clearInterval(tick);
      supabase.removeChannel(channel);
    };
  }, [initial.id, load]);

  const runnerFresh = !!t.runner_id && !!t.runner_beat_at && now > 0 && now - new Date(t.runner_beat_at).getTime() < RUNNER_TIMEOUT_MS;
  const iAmRunner = t.runner_id === viewerId && runnerFresh;

  const claim = useCallback(async () => {
    setMessage(null);
    const { data } = await createClient().rpc("claim_bot_tournament", { p_id: initial.id });
    if (data === true) setClaimed(true);
    else setMessage("Someone else is already running it.");
    load();
  }, [initial.id, load]);

  // The creator starts it automatically; anyone who was already the runner (page reload) picks it up again.
  useEffect(() => {
    if (!viewerId || t.status !== "running" || claimed || now === 0) return;
    if (runnerFresh && t.runner_id !== viewerId) return;
    if (t.creator_id === viewerId || t.runner_id === viewerId) queueMicrotask(claim);
  }, [viewerId, t.status, t.creator_id, t.runner_id, runnerFresh, claimed, now, claim]);

  useEffect(() => {
    if (runner.lost) queueMicrotask(() => { setClaimed(false); setMessage("Another browser took over running this tournament."); });
  }, [runner.lost]);
  useEffect(() => {
    if (runner.error) queueMicrotask(() => { setClaimed(false); setMessage(runner.error); });
  }, [runner.error]);

  // Leaving the page hands the claim back so someone else can continue straight away.
  useEffect(() => {
    if (!claimed) return;
    const supabase = createClient();
    return () => {
      void supabase.rpc("release_bot_tournament", { p_id: initial.id });
    };
  }, [claimed, initial.id]);

  const champ = t.status === "finished" ? (t.bots.find((b) => b.id === t.champion_id) ?? champion(spec, games)) : null;
  const live = games.find((g) => g.status === "playing") ?? null;
  const shown = games.find((g) => g.id === selected) ?? live ?? games.at(-1) ?? null;
  const moves = shown ? (runner.liveMoves[shown.id] && shown.status === "playing" ? runner.liveMoves[shown.id] : shown.moves) : [];
  const name = (id: string) => t.bots.find((b) => b.id === id)?.name ?? id;
  const sideOf = (id: string) => ({ id, name: name(id), rating: t.bots.find((b) => b.id === id)?.rating ?? 0 });
  const total = spec.format === "roundrobin" ? t.bots.length * (t.bots.length - 1) : null;
  const finishedCount = games.filter((g) => g.status === "finished").length;

  const remove = async () => {
    if (!confirm("Delete this tournament for everyone?")) return;
    await createClient().from("bot_tournaments").delete().eq("id", t.id);
    router.push("/bot-tournaments");
  };


  return (
    <div className="grid min-w-0 gap-5">
      <header className="grid gap-1">
        <p className="text-sm"><Link href="/bot-tournaments" className="text-dim hover:text-ink">← Bot tournaments</Link></p>
        <h1 className="text-3xl font-extrabold tracking-tight">{t.name}</h1>
        <p className="text-sm text-dim">
          {t.format === "knockout" ? `Knockout · best of ${t.best_of}` : "Round-robin · every pair plays twice"} · {t.bots.length} bots · {SPEEDS[t.speed].label} speed{t.variant === "chess960" ? " · Chess960" : ""} · started {fmtDate(t.created_at)} by {t.creator?.username ?? "someone"}
        </p>
      </header>

      {champ && (
        <div className="flex items-center gap-4 rounded-xl border border-brand bg-brand/15 p-5" role="status">
          <span aria-hidden className="text-5xl">🏆</span>
          <div><p className="text-xs font-bold uppercase tracking-wide text-brand">Champion</p><p className="text-2xl font-extrabold">{champ.name}</p><p className="text-sm text-dim">≈{champ.rating} · {finishedCount} games played</p></div>
        </div>
      )}

      {t.status === "running" && (
        <div className={`${ui.card} grid gap-2`} aria-live="polite">
          {runningHere && runner.running ? (
            <p className="font-semibold">You are running this tournament. Keep this page open.</p>
          ) : runnerFresh ? (
            <p className="font-semibold">Live: running in {t.runner?.username ?? "someone"}&apos;s browser{iAmRunner ? " (that is you)" : ""}.</p>
          ) : (
            <>
              <p className="font-semibold">Waiting for a browser to run the games.</p>
              <p className="text-sm text-dim">The bots play in a browser tab, because that&apos;s where the chess engine is. {viewerId ? "Press the button and your tab will play the games for everyone." : "Log in to run it."}</p>
              {viewerId && <button onClick={claim} className={`${ui.primary} justify-self-start`}>Run this tournament</button>}
            </>
          )}
          {spec.format === "roundrobin" && total ? <p className="text-xs text-dim">{finishedCount} of {total} games played</p> : <p className="text-xs text-dim">{finishedCount} games played so far</p>}
          {message && <p role="alert" className="text-sm text-warn">{message}</p>}
          {runningHere && !engine && <p className="text-sm text-dim">Loading the engine…</p>}
        </div>
      )}

      {shown ? (
        <BotMatchBoard
          record={{ variant: t.variant, initialFen: shown.initial_fen, moves }}
          white={sideOf(shown.white_bot_id)}
          black={sideOf(shown.black_bot_id)}
          bots={bots}
          status={
            <div className="grid gap-1">
              <p className="font-semibold">
                {shown.status === "playing" ? `Now playing: ${name(shown.white_bot_id)} vs ${name(shown.black_bot_id)}` : `${name(shown.white_bot_id)} vs ${name(shown.black_bot_id)}: ${shown.result === "1-0" ? `${name(shown.white_bot_id)} won` : shown.result === "0-1" ? `${name(shown.black_bot_id)} won` : "draw"}`}
              </p>
              <p className="text-xs text-dim">{spec.format === "knockout" ? `Round ${shown.round}, match ${shown.slot + 1}, game ${shown.game_no}` : `Round ${shown.round}`}{shown.reason ? ` · ${shown.reason}` : ""}{live && shown.id !== live.id ? " · click the Live game to follow the action" : ""}</p>
            </div>
          }
          controls={live && shown.id !== live.id ? <button className={`${ui.btn} justify-self-start`} onClick={() => setSelected(null)}>Back to the live game</button> : undefined}
        />
      ) : (
        <div className={`${ui.card} text-center text-dim`}>The first game hasn&apos;t started yet.</div>
      )}

      <BotBracket spec={spec} games={games} bots={bots} selected={shown?.id ?? null} onSelect={(id) => setSelected(id)} />

      {(isMod || t.creator_id === viewerId) && <button className="justify-self-start text-xs text-dim underline hover:text-danger" onClick={remove}>Delete this tournament</button>}
    </div>
  );
}
