"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { acceptSeek, cancelSeek, createChallenge, createDailySeek } from "@/app/actions/games";
import { VARIANTS, VARIANT_IDS, type Variant } from "@/lib/chess/variants";
import { DAILY_DAYS, formatDuration } from "@/lib/live/timecontrols";
import type { SeekRow } from "@/lib/live/types";
import { plural } from "@/lib/ratings";
import { createClient } from "@/lib/supabase/client";

type Me = { id: string; username: string } | null;
type SeekWithNames = SeekRow & { user: { username: string } | null; target: { username: string } | null };
type MyGame = {
  id: string; variant: Variant; moves: string[]; white_id: string; black_id: string; move_deadline: string | null; days_per_move: number | null; rated: boolean;
  white: { username: string } | null; black: { username: string } | null;
};

const btn = "rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-medium hover:bg-surface-2 disabled:opacity-40";
const field = "rounded-md border border-line bg-bg px-3 py-2 text-sm text-ink focus:border-brand focus:outline-none";

export function DailyLobby({ me }: { me: Me }) {
  const router = useRouter();
  const [seeks, setSeeks] = useState<SeekWithNames[]>([]);
  const [games, setGames] = useState<MyGame[]>([]);
  const [variant, setVariant] = useState<Variant>("standard");
  const [days, setDays] = useState<number>(3);
  const [rated, setRated] = useState(true);
  const [username, setUsername] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [now, setNow] = useState(0);

  const refresh = useCallback(async () => {
    const supabase = createClient();
    const { data } = await supabase
      .from("seeks")
      .select("*, user:profiles!seeks_user_id_fkey(username), target:profiles!seeks_target_id_fkey(username)")
      .eq("mode", "daily")
      .order("created_at", { ascending: true });
    setSeeks((data as SeekWithNames[] | null) ?? []);
    if (me) {
      const { data: g } = await supabase
        .from("games")
        .select("id, variant, moves, white_id, black_id, move_deadline, days_per_move, rated, white:profiles!games_white_id_fkey(username), black:profiles!games_black_id_fkey(username)")
        .eq("status", "started")
        .eq("mode", "daily")
        .or(`white_id.eq.${me.id},black_id.eq.${me.id}`)
        .order("move_deadline", { ascending: true });
      setGames((g as unknown as MyGame[] | null) ?? []);
    }
    setNow(Date.now());
  }, [me]);

  useEffect(() => {
    queueMicrotask(() => refresh());
    const timer = setInterval(() => document.visibilityState === "visible" && refresh(), 10_000);
    return () => clearInterval(timer);
  }, [refresh]);

  const act = async <T extends { ok: boolean }>(fn: () => Promise<T | { ok: false; error: string }>): Promise<T | null> => {
    setError(null);
    setInfo(null);
    setBusy(true);
    try {
      const res = await fn();
      if (!res.ok) { setError((res as { error: string }).error); return null; }
      return res as T;
    } finally {
      setBusy(false);
    }
  };

  const open = seeks.filter((s) => s.target_id === null);
  const incoming = me ? seeks.filter((s) => s.target_id === me.id) : [];
  const outgoing = me ? seeks.filter((s) => s.user_id === me.id && s.target_id) : [];
  const myOffer = me ? open.find((s) => s.user_id === me.id) : undefined;
  const myTurn = (g: MyGame) => (g.moves.length % 2 === 0) === (g.white_id === me?.id);
  const ordered = [...games].sort((a, b) => Number(myTurn(b)) - Number(myTurn(a)));
  const turnsWaiting = games.filter(myTurn).length;

  return (
    <div className="grid gap-8">
      {error && <p role="alert" className="rounded-md border border-danger/50 bg-danger/10 px-3 py-2 text-sm text-danger">{error}</p>}
      {info && <p role="status" className="rounded-md border border-line bg-surface px-3 py-2 text-sm text-dim">{info}</p>}
      {!me && (
        <p className="rounded-md border border-line bg-surface px-3 py-2 text-sm text-dim">
          <Link href="/login" className="font-semibold text-brand hover:underline">Log in</Link> or <Link href="/signup" className="font-semibold text-brand hover:underline">create an account</Link> to play daily games.
        </p>
      )}

      {me && (
        <section aria-labelledby="mine" className="grid gap-3">
          <h2 id="mine" className="text-xl font-bold">Your games {turnsWaiting > 0 && <span className="ml-2 rounded-full bg-brand px-2 py-0.5 text-xs font-bold text-brand-ink">{turnsWaiting} waiting for you</span>}</h2>
          {ordered.length === 0 ? (
            <p className="text-sm text-dim">No daily games yet. Post an offer below or challenge a classmate.</p>
          ) : (
            <ul className="grid gap-2">
              {ordered.map((g) => {
                const opp = g.white_id === me.id ? g.black : g.white;
                const left = g.move_deadline ? Date.parse(g.move_deadline) - (now || Date.parse(g.move_deadline)) : null;
                return (
                  <li key={g.id}>
                    <Link href={`/game/${g.id}`} className={`flex flex-wrap items-center justify-between gap-2 rounded-xl border px-4 py-3 transition hover:border-brand ${myTurn(g) ? "border-brand bg-brand/10" : "border-line bg-surface"}`}>
                      <span className="flex items-center gap-3">
                        <span className={`rounded-full px-2 py-0.5 text-xs font-bold ${myTurn(g) ? "bg-brand text-brand-ink" : "bg-surface-2 text-dim"}`}>{myTurn(g) ? "Your move" : "Waiting"}</span>
                        <span>vs <strong>{opp?.username ?? "?"}</strong></span>
                      </span>
                      <span className="text-sm text-dim">
                        {VARIANTS[g.variant].label} · {g.rated ? "Rated" : "Casual"} · {plural(Math.ceil(g.moves.length / 2), "move")}{left !== null ? ` · ${left <= 0 ? "overdue" : `${formatDuration(left)} left`}` : ""}
                      </span>
                    </Link>
                  </li>
                );
              })}
            </ul>
          )}
        </section>
      )}

      {incoming.length > 0 && (
        <section aria-labelledby="inc" className="grid gap-2">
          <h2 id="inc" className="text-xl font-bold">Daily challenges for you</h2>
          {incoming.map((s) => (
            <div key={s.id} className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-brand bg-surface px-4 py-3">
              <span><strong>{s.user?.username}</strong> · {plural(s.days_per_move ?? 0, "day")} per move · {VARIANTS[s.variant].label} · {s.rated ? "Rated" : "Casual"}</span>
              <span className="flex gap-2">
                <button className="rounded-md bg-brand px-3 py-1.5 text-sm font-semibold text-brand-ink hover:bg-brand-hi disabled:opacity-50" disabled={busy}
                  onClick={async () => { const r = await act(() => acceptSeek(s.id)); if (r && "gameId" in r) router.push(`/game/${r.gameId}`); }}>Accept</button>
                <button className={btn} onClick={async () => { await cancelSeek(s.id); refresh(); }}>Decline</button>
              </span>
            </div>
          ))}
        </section>
      )}

      <section aria-labelledby="open" className="grid gap-2">
        <h2 id="open" className="text-xl font-bold">Open daily offers</h2>
        {open.length === 0 ? (
          <p className="text-sm text-dim">No open offers right now. Post one below.</p>
        ) : (
          <ul className="grid gap-2">
            {open.map((s) => (
              <li key={s.id} className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-line bg-surface px-4 py-3">
                <span><strong>{s.user?.username ?? "Someone"}</strong> · {plural(s.days_per_move ?? 0, "day")} per move · {VARIANTS[s.variant].label} · {s.rated ? "Rated" : "Casual"}</span>
                {me && s.user_id === me.id ? (
                  <button className={btn} onClick={async () => { await cancelSeek(s.id); refresh(); }}>Withdraw</button>
                ) : (
                  <button className={btn} disabled={!me || busy} onClick={async () => { const r = await act(() => acceptSeek(s.id)); if (r && "gameId" in r) router.push(`/game/${r.gameId}`); else refresh(); }}>Play</button>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>

      {me && (
        <section aria-labelledby="new" className="grid max-w-xl gap-4 rounded-xl border border-line bg-surface p-5">
          <h2 id="new" className="text-xl font-bold">Start a daily game</h2>
          <div className="grid gap-3 sm:grid-cols-3">
            <label className="grid gap-1 text-sm font-medium">Game
              <select className={field} value={variant} onChange={(e) => setVariant(e.target.value as Variant)}>
                {VARIANT_IDS.map((v) => <option key={v} value={v}>{VARIANTS[v].label}</option>)}
              </select>
            </label>
            <label className="grid gap-1 text-sm font-medium">Time per move
              <select className={field} value={days} onChange={(e) => setDays(Number(e.target.value))}>
                {DAILY_DAYS.map((d) => <option key={d} value={d}>{plural(d, "day")}</option>)}
              </select>
            </label>
            <label className="flex items-center gap-2 self-end pb-2 text-sm">
              <input type="checkbox" checked={rated} onChange={(e) => setRated(e.target.checked)} className="size-4 accent-[var(--brand)]" /> Rated
            </label>
          </div>
          <div className="flex flex-wrap gap-2">
            <button className="rounded-md bg-brand px-4 py-2 text-sm font-semibold text-brand-ink hover:bg-brand-hi disabled:opacity-50" disabled={busy || !!myOffer}
              onClick={async () => { const r = await act(() => createDailySeek({ variant, days, rated })); if (r) { setInfo("Offer posted. You'll get a notification when someone accepts."); refresh(); } }}>
              {myOffer ? "You already have an open offer" : "Post an open offer"}
            </button>
          </div>
          <form className="grid gap-3 border-t border-line pt-4" onSubmit={async (e) => {
            e.preventDefault();
            const r = await act(() => createChallenge({ username, variant, color: "random", rated, mode: "daily", days }));
            if (r) { setInfo(`Challenge sent to ${username}.`); setUsername(""); refresh(); }
          }}>
            <label className="grid gap-1 text-sm font-medium">Or challenge a classmate
              <input className={field} placeholder="Username" value={username} onChange={(e) => setUsername(e.target.value)} autoCapitalize="none" spellCheck={false} />
            </label>
            <button disabled={busy || !username.trim()} className={`${btn} justify-self-start`}>Send challenge</button>
          </form>
          {outgoing.length > 0 && (
            <ul className="grid gap-1 text-sm">
              {outgoing.map((s) => (
                <li key={s.id} className="flex items-center justify-between gap-2">
                  <span className="text-dim">Waiting for <strong className="text-ink">{s.target?.username}</strong> · {plural(s.days_per_move ?? 0, "day")} per move</span>
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
