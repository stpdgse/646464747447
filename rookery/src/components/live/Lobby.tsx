"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { acceptSeek, cancelSeek, createChallenge, quickPair } from "@/app/actions/games";
import { VARIANTS, VARIANT_IDS, type Variant } from "@/lib/chess/variants";
import { categoryOf, describeTimeControl, getLiveTimeControl, LIVE_TIME_CONTROLS } from "@/lib/live/timecontrols";
import type { SeekRow } from "@/lib/live/types";
import { createClient } from "@/lib/supabase/client";

type SeekWithNames = SeekRow & { user: { username: string } | null; target: { username: string } | null };
type Me = { id: string; username: string } | null;

const btn = "rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-medium hover:bg-surface-2 disabled:opacity-40";
const field = "rounded-md border border-line bg-bg px-3 py-2 text-sm text-ink focus:border-brand focus:outline-none";

export function Lobby({ me }: { me: Me }) {
  const router = useRouter();
  const [variant, setVariant] = useState<Variant>("standard");
  const [seeks, setSeeks] = useState<SeekWithNames[]>([]);
  const [activeGame, setActiveGame] = useState<string | null>(null);
  const [waiting, setWaiting] = useState<{ seekId: string; label: string } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [challenge, setChallenge] = useState({ username: "", timeId: "5+0", color: "random" as "white" | "black" | "random" });

  const refresh = useCallback(async () => {
    const supabase = createClient();
    const { data } = await supabase
      .from("seeks")
      .select("*, user:profiles!seeks_user_id_fkey(username), target:profiles!seeks_target_id_fkey(username)")
      .order("created_at", { ascending: true });
    setSeeks((data as SeekWithNames[] | null) ?? []);
    if (me) {
      const { data: games } = await supabase
        .from("games")
        .select("id")
        .eq("status", "started")
        .or(`white_id.eq.${me.id},black_id.eq.${me.id}`)
        .limit(1);
      setActiveGame(games?.[0]?.id ?? null);
    }
  }, [me]);

  useEffect(() => {
    queueMicrotask(() => refresh());
    const supabase = createClient();
    const channel = supabase
      .channel("lobby")
      .on("postgres_changes", { event: "*", schema: "public", table: "seeks" }, () => refresh())
      .on("postgres_changes", { event: "INSERT", schema: "public", table: "games" }, () => refresh())
      .subscribe();
    const timer = setInterval(() => document.visibilityState === "visible" && refresh(), 3000);
    return () => {
      clearInterval(timer);
      supabase.removeChannel(channel);
    };
  }, [refresh]);

  // Matched while waiting in the queue: jump into the game.
  useEffect(() => {
    if (waiting && activeGame) router.push(`/game/${activeGame}`);
  }, [waiting, activeGame, router]);

  const act = async <T extends { ok: boolean }>(fn: () => Promise<T | { ok: false; error: string }>): Promise<T | null> => {
    setError(null);
    setInfo(null);
    setBusy(true);
    try {
      const res = await fn();
      if (!res.ok) {
        setError((res as { error: string }).error);
        return null;
      }
      return res as T;
    } finally {
      setBusy(false);
    }
  };

  const play = async (timeId: string) => {
    const tc = getLiveTimeControl(timeId);
    if (!tc) return;
    const res = await act(() => quickPair(variant, timeId));
    if (!res) return;
    if ("gameId" in res) router.push(`/game/${res.gameId}`);
    else if ("waiting" in res) {
      setWaiting({ seekId: res.seekId, label: `${tc.label} ${VARIANTS[variant].label}` });
      refresh();
    }
  };

  const open = seeks.filter((s) => s.target_id === null);
  const incoming = me ? seeks.filter((s) => s.target_id === me.id) : [];
  const outgoing = me ? seeks.filter((s) => s.user_id === me.id && s.target_id) : [];

  return (
    <div className="grid gap-8">
      {activeGame && (
        <div className="flex flex-wrap items-center gap-3 rounded-xl border border-brand bg-brand/10 px-4 py-3">
          <span className="font-semibold">You have a game in progress.</span>
          <Link href={`/game/${activeGame}`} className="rounded-md bg-brand px-3 py-1.5 text-sm font-semibold text-brand-ink hover:bg-brand-hi">Resume</Link>
        </div>
      )}
      {error && <p role="alert" className="rounded-md border border-danger/50 bg-danger/10 px-3 py-2 text-sm text-danger">{error}</p>}
      {info && <p role="status" className="rounded-md border border-line bg-surface px-3 py-2 text-sm text-dim">{info}</p>}

      <section aria-labelledby="quick" className="grid gap-4">
        <h2 id="quick" className="text-xl font-bold">Quick play</h2>
        {!me && (
          <p className="rounded-md border border-line bg-surface px-3 py-2 text-sm text-dim">
            <Link href="/login" className="font-semibold text-brand hover:underline">Log in</Link> or <Link href="/signup" className="font-semibold text-brand hover:underline">create an account</Link> to play. You can already watch games.
          </p>
        )}
        <div role="radiogroup" aria-label="Game type" className="flex flex-wrap gap-2">
          {VARIANT_IDS.map((v) => (
            <button key={v} role="radio" aria-checked={variant === v} onClick={() => setVariant(v)} title={VARIANTS[v].blurb}
              className={`rounded-full border px-3 py-1 text-sm font-medium ${variant === v ? "border-brand bg-brand/15" : "border-line bg-surface hover:bg-surface-2"}`}>
              {VARIANTS[v].label}
            </button>
          ))}
        </div>
        {waiting ? (
          <div role="status" className="flex flex-wrap items-center gap-3 rounded-xl border border-brand bg-brand/10 px-4 py-4">
            <span className="size-3 animate-pulse rounded-full bg-brand" aria-hidden />
            <span className="font-semibold">Looking for an opponent: {waiting.label}…</span>
            <button className={btn} onClick={async () => { await cancelSeek(waiting.seekId); setWaiting(null); refresh(); }}>Cancel</button>
          </div>
        ) : (
          (["Bullet", "Blitz", "Rapid"] as const).map((group) => (
            <div key={group} className="grid gap-2">
              <h3 className="text-sm font-semibold text-dim">{group}</h3>
              <div className="grid grid-cols-3 gap-2 sm:grid-cols-4 md:grid-cols-6">
                {LIVE_TIME_CONTROLS.filter((t) => t.group === group).map((t) => (
                  <button key={t.id} disabled={!me || busy || !!activeGame} onClick={() => play(t.id)}
                    className="rounded-xl border border-line bg-surface py-4 text-lg font-bold transition hover:border-brand hover:bg-surface-2 disabled:opacity-40 disabled:hover:border-line">
                    {t.label}
                  </button>
                ))}
              </div>
            </div>
          ))
        )}
      </section>

      {incoming.length > 0 && (
        <section aria-labelledby="inc" className="grid gap-2">
          <h2 id="inc" className="text-xl font-bold">Challenges for you</h2>
          {incoming.map((s) => (
            <div key={s.id} className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-brand bg-surface px-4 py-3">
              <span><strong>{s.user?.username}</strong> challenges you · {describeTimeControl(s.time_initial_ms, s.time_inc_ms)} {categoryOf(s.time_initial_ms, s.time_inc_ms)} · {VARIANTS[s.variant].label}</span>
              <span className="flex gap-2">
                <button className="rounded-md bg-brand px-3 py-1.5 text-sm font-semibold text-brand-ink hover:bg-brand-hi disabled:opacity-50" disabled={busy || !!activeGame}
                  onClick={async () => { const r = await act(() => acceptSeek(s.id)); if (r && "gameId" in r) router.push(`/game/${r.gameId}`); }}>Accept</button>
                <button className={btn} onClick={async () => { await cancelSeek(s.id); refresh(); }}>Decline</button>
              </span>
            </div>
          ))}
        </section>
      )}

      <section aria-labelledby="open" className="grid gap-2">
        <h2 id="open" className="text-xl font-bold">Open games</h2>
        {open.length === 0 ? (
          <p className="text-sm text-dim">Nobody is waiting right now. Start a quick game and others can join you.</p>
        ) : (
          <ul className="grid gap-2">
            {open.map((s) => (
              <li key={s.id} className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-line bg-surface px-4 py-3">
                <span><strong>{s.user?.username ?? "Someone"}</strong> · {describeTimeControl(s.time_initial_ms, s.time_inc_ms)} {categoryOf(s.time_initial_ms, s.time_inc_ms)} · {VARIANTS[s.variant].label}</span>
                {me && s.user_id === me.id ? (
                  <button className={btn} onClick={async () => { await cancelSeek(s.id); setWaiting(null); refresh(); }}>Cancel</button>
                ) : (
                  <button className={btn} disabled={!me || busy || !!activeGame}
                    onClick={async () => { const r = await act(() => acceptSeek(s.id)); if (r && "gameId" in r) router.push(`/game/${r.gameId}`); else refresh(); }}>Play</button>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>

      {me && (
        <section aria-labelledby="chal" className="grid max-w-xl gap-3 rounded-xl border border-line bg-surface p-5">
          <h2 id="chal" className="text-xl font-bold">Challenge a classmate</h2>
          <form className="grid gap-3 sm:grid-cols-2"
            onSubmit={async (e) => {
              e.preventDefault();
              const r = await act(() => createChallenge({ username: challenge.username, variant, timeId: challenge.timeId, color: challenge.color }));
              if (r) { setInfo(`Challenge sent to ${challenge.username}. They'll see it in their lobby.`); setChallenge({ ...challenge, username: "" }); refresh(); }
            }}>
            <label className="grid gap-1 text-sm font-medium sm:col-span-2">Username
              <input className={field} value={challenge.username} onChange={(e) => setChallenge({ ...challenge, username: e.target.value })} autoCapitalize="none" spellCheck={false} required />
            </label>
            <label className="grid gap-1 text-sm font-medium">Time
              <select className={field} value={challenge.timeId} onChange={(e) => setChallenge({ ...challenge, timeId: e.target.value })}>
                {LIVE_TIME_CONTROLS.map((t) => <option key={t.id} value={t.id}>{t.label} ({t.group})</option>)}
              </select>
            </label>
            <label className="grid gap-1 text-sm font-medium">You play
              <select className={field} value={challenge.color} onChange={(e) => setChallenge({ ...challenge, color: e.target.value as typeof challenge.color })}>
                <option value="random">Random</option><option value="white">White</option><option value="black">Black</option>
              </select>
            </label>
            <p className="text-xs text-dim sm:col-span-2">Game type: {VARIANTS[variant].label} (change it above).</p>
            <button disabled={busy || !challenge.username.trim()} className="justify-self-start rounded-md bg-brand px-4 py-2 text-sm font-semibold text-brand-ink hover:bg-brand-hi disabled:opacity-50">Send challenge</button>
          </form>
          {outgoing.length > 0 && (
            <ul className="grid gap-1 text-sm">
              {outgoing.map((s) => (
                <li key={s.id} className="flex items-center justify-between gap-2">
                  <span className="text-dim">Waiting for <strong className="text-ink">{s.target?.username}</strong> · {describeTimeControl(s.time_initial_ms, s.time_inc_ms)} · {VARIANTS[s.variant].label}</span>
                  <button className={btn} onClick={async () => { await cancelSeek(s.id); refresh(); }}>Withdraw</button>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
    </div>
  );
}
