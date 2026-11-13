"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";
import { playTournament } from "@/app/actions/tournaments";
import { describeTimeControl } from "@/lib/live/timecontrols";
import { fmtDate, ui } from "@/lib/social";
import { createClient } from "@/lib/supabase/client";

export type T = { id: string; name: string; description: string; variant: string; time_initial_ms: number; time_inc_ms: number; rated: boolean; starts_at: string; ends_at: string; created_by: string };
type P = { user_id: string; points: number; games: number; wins: number; draws: number; losses: number; active: boolean; waiting_since: string | null; user: { username: string } | null };

const fmtLeft = (ms: number) => {
  const s = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), r = s % 60;
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${String(r).padStart(2, "0")}` : `${m}:${String(r).padStart(2, "0")}`;
};

/** Standings, countdown, and the Join / Play buttons. Polls every few seconds, and keeps asking for a game while you wait. */
export function TournamentLive({ tournament: t, viewerId, isMod }: { tournament: T; viewerId: string | null; isMod: boolean }) {
  const router = useRouter();
  const [players, setPlayers] = useState<P[] | null>(null);
  const offset = useRef(0); // server time minus this device's clock
  const [now, setNow] = useState(0);
  const [waiting, setWaiting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const waitingRef = useRef(false);

  const load = useCallback(async () => {
    const supabase = createClient();
    const [{ data }, { data: st }] = await Promise.all([
      supabase.from("tournament_players").select("user_id, points, games, wins, draws, losses, active, waiting_since, user:profiles!tournament_players_user_id_fkey(username)").eq("tournament_id", t.id),
      supabase.rpc("server_time"),
    ]);
    setPlayers((data as unknown as P[] | null) ?? []);
    if (typeof st === "string") offset.current = new Date(st).getTime() - Date.now();
    setNow(Date.now() + offset.current);
  }, [t.id]);

  useEffect(() => {
    queueMicrotask(load);
    const supabase = createClient();
    const channel = supabase.channel(`tournament-${t.id}`).on("postgres_changes", { event: "*", schema: "public", table: "tournament_players", filter: `tournament_id=eq.${t.id}` }, () => load()).subscribe();
    const timer = setInterval(() => document.visibilityState === "visible" && load(), 8000);
    const tick = setInterval(() => setNow(Date.now() + offset.current), 1000);
    return () => {
      clearInterval(timer); clearInterval(tick);
      supabase.removeChannel(channel);
    };
  }, [t.id, load]);

  const start = new Date(t.starts_at).getTime(), end = new Date(t.ends_at).getTime();
  const phase = now === 0 ? "loading" : now < start ? "upcoming" : now < end ? "running" : "finished";
  const me = players?.find((p) => p.user_id === viewerId && p.active) ?? null;
  const rows = [...(players ?? [])].filter((p) => p.active || p.games > 0).sort((a, b) => b.points - a.points || b.wins - a.wins || a.games - b.games || (a.user?.username ?? "").localeCompare(b.user?.username ?? ""));

  const ask = useCallback(async () => {
    const res = await playTournament(t.id);
    if (!res.ok) { setWaiting(false); waitingRef.current = false; setError(res.error); return; }
    if ("gameId" in res) { waitingRef.current = false; setWaiting(false); router.push(`/game/${res.gameId}`); return; }
    setError(null);
  }, [t.id, router]);

  // While waiting, ask again every 4 seconds: that keeps us in the queue and notices when someone pairs with us.
  useEffect(() => {
    if (!waiting) return;
    const timer = setInterval(() => waitingRef.current && ask(), 4000);
    return () => clearInterval(timer);
  }, [waiting, ask]);

  const join = async () => {
    setBusy(true); setError(null);
    const { error: err } = await createClient().rpc("tournament_join", { p_tournament: t.id });
    setBusy(false);
    if (err) return setError(err.message.includes("ended") ? "This tournament is over." : "Could not join.");
    load();
  };
  const leave = async () => {
    setWaiting(false); waitingRef.current = false;
    await createClient().rpc("tournament_leave", { p_tournament: t.id });
    load();
  };
  const play = async () => {
    setBusy(true); setError(null);
    waitingRef.current = true; setWaiting(true);
    await ask();
    setBusy(false);
  };
  const cancelWaiting = async () => {
    waitingRef.current = false; setWaiting(false);
    await createClient().rpc("tournament_stop_waiting", { p_tournament: t.id });
  };

  const podium = phase === "finished" ? rows.filter((p) => p.games > 0).slice(0, 3) : [];

  return (
    <div className="grid min-w-0 gap-6">
      <header className="grid gap-1">
        <p className="text-sm"><Link href="/tournaments" className="text-dim hover:text-ink">← Tournaments</Link></p>
        <h1 className="text-3xl font-extrabold tracking-tight">{t.name}</h1>
        {t.description && <p className="text-dim">{t.description}</p>}
        <p className="text-sm text-dim">{describeTimeControl(t.time_initial_ms, t.time_inc_ms)} · {t.variant === "chess960" ? "Chess960" : "Standard"} · {t.rated ? "Rated" : "Casual"} · {fmtDate(t.starts_at)} to {fmtDate(t.ends_at)}</p>
      </header>

      <div className={`${ui.card} grid gap-3`} aria-live="polite">
        {phase === "loading" && <p className="text-dim">Loading…</p>}
        {phase === "upcoming" && <p className="text-lg font-bold">Starts in <span className="tabular-nums">{fmtLeft(start - now)}</span></p>}
        {phase === "running" && <p className="text-lg font-bold">Ends in <span className="tabular-nums">{fmtLeft(end - now)}</span></p>}
        {phase === "finished" && <p className="text-lg font-bold">Tournament over</p>}
        {error && <p role="alert" className={ui.err}>{error}</p>}
        {viewerId ? (
          phase !== "finished" && (
            <div className="flex flex-wrap items-center gap-3">
              {!me ? <button disabled={busy} onClick={join} className={ui.primary}>Join the tournament</button> : (
                <>
                  {phase === "running" && !waiting && <button disabled={busy} onClick={play} className={ui.primary}>Play</button>}
                  {waiting && (<><span role="status" className="text-sm font-semibold">Looking for an opponent…</span><button onClick={cancelWaiting} className={ui.btn}>Stop looking</button></>)}
                  {phase === "upcoming" && <span className="text-sm text-dim">You&apos;re in. The Play button appears when it starts.</span>}
                  <button onClick={leave} className="text-xs text-dim underline hover:text-ink">Leave</button>
                </>
              )}
            </div>
          )
        ) : <p className="text-sm text-dim"><Link href="/login" className="font-semibold text-brand underline">Log in</Link> to join.</p>}
        {isMod && phase !== "finished" && <CancelButton id={t.id} />}
      </div>

      {podium.length > 0 && (
        <section aria-label="Podium" className="grid gap-3 sm:grid-cols-3">
          {podium.map((p, i) => (
            <div key={p.user_id} className={`${ui.card} text-center ${i === 0 ? "border-brand" : ""}`}>
              <p aria-hidden className="text-4xl">{["🥇", "🥈", "🥉"][i]}</p>
              <Link href={`/u/${p.user?.username}`} className="text-lg font-bold hover:underline">{p.user?.username}</Link>
              <p className="text-sm text-dim">{p.points} points · {p.games} games</p>
            </div>
          ))}
        </section>
      )}

      <section aria-labelledby="standings" className="grid gap-2">
        <h2 id="standings" className="text-xl font-bold">Standings</h2>
        {players === null ? <div className="h-24 animate-pulse rounded-xl bg-surface" /> : rows.length === 0 ? <p className="text-sm text-dim">Nobody has joined yet.</p> : (
          <div className="overflow-x-auto rounded-xl border border-line">
            <table className="w-full min-w-[28rem] text-sm">
              <thead className="bg-surface-2 text-left text-xs uppercase text-dim">
                <tr><th className="px-3 py-2">#</th><th className="px-3 py-2">Player</th><th className="px-3 py-2 text-right">Points</th><th className="px-3 py-2 text-right">Games</th><th className="px-3 py-2 text-right">W / D / L</th></tr>
              </thead>
              <tbody>
                {rows.map((p, i) => (
                  <tr key={p.user_id} className={`border-t border-line ${p.user_id === viewerId ? "bg-brand/10" : ""}`}>
                    <td className="px-3 py-2 tabular-nums">{i + 1}</td>
                    <td className="px-3 py-2 font-semibold"><Link href={`/u/${p.user?.username}`} className="hover:underline">{p.user?.username}</Link>{p.waiting_since && phase === "running" && <span className="ml-2 text-xs font-normal text-dim">looking for a game</span>}{!p.active && <span className="ml-2 text-xs font-normal text-dim">left</span>}</td>
                    <td className="px-3 py-2 text-right font-bold tabular-nums">{p.points}</td>
                    <td className="px-3 py-2 text-right tabular-nums">{p.games}</td>
                    <td className="px-3 py-2 text-right tabular-nums">{p.wins} / {p.draws} / {p.losses}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        <p className="text-xs text-dim">A game scores only if each player makes at least two moves. Win 2, draw 1, loss 0.</p>
      </section>
    </div>
  );
}

function CancelButton({ id }: { id: string }) {
  const router = useRouter();
  return (
    <button className="justify-self-start text-xs text-dim underline hover:text-danger" onClick={async () => {
      if (!confirm("Cancel this tournament for everyone?")) return;
      await createClient().from("tournaments").delete().eq("id", id);
      router.push("/tournaments");
    }}>Cancel tournament (moderators)</button>
  );
}
