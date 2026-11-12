"use client";

import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import { createClient } from "@/lib/supabase/client";
import { USERNAME_RE, friendlyError, isOnline, ui } from "@/lib/social";

type P = { id: string; username: string };
type Row = { id: number; requester_id: string; addressee_id: string; status: "pending" | "accepted"; requester: P; addressee: P };
type Live = { id: string; white_id: string; black_id: string };

/** Friends list with online dots, requests, blocking, and "watch" links for friends who are mid-game. */
export function FriendsPanel({ viewerId }: { viewerId: string }) {
  const [rows, setRows] = useState<Row[] | null>(null);
  const [seen, setSeen] = useState<Record<string, string>>({});
  const [playing, setPlaying] = useState<Live[]>([]);
  const [blocks, setBlocks] = useState<(P & { blocked_id: string })[]>([]);
  const [now, setNow] = useState(0);
  const [username, setUsername] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    const supabase = createClient();
    const [fr, bl] = await Promise.all([
      supabase.from("friendships").select("id, requester_id, addressee_id, status, requester:profiles!friendships_requester_id_fkey(id, username), addressee:profiles!friendships_addressee_id_fkey(id, username)").order("created_at", { ascending: false }),
      supabase.from("blocks").select("blocked_id, user:profiles!blocks_blocked_id_fkey(id, username)"),
    ]);
    const list = (fr.data as unknown as Row[] | null) ?? [];
    setRows(list);
    setBlocks(((bl.data as unknown as { blocked_id: string; user: P }[] | null) ?? []).map((b) => ({ ...b.user, blocked_id: b.blocked_id })));
    const friendIds = list.filter((r) => r.status === "accepted").map((r) => (r.requester_id === viewerId ? r.addressee_id : r.requester_id));
    if (friendIds.length) {
      const [pr, gm] = await Promise.all([
        supabase.from("presence").select("user_id, last_seen_at").in("user_id", friendIds),
        supabase.from("games").select("id, white_id, black_id").eq("status", "started")
          .or(`white_id.in.(${friendIds.join(",")}),black_id.in.(${friendIds.join(",")})`).limit(20),
      ]);
      setSeen(Object.fromEntries(((pr.data as { user_id: string; last_seen_at: string }[] | null) ?? []).map((p) => [p.user_id, p.last_seen_at])));
      setPlaying((gm.data as Live[] | null) ?? []);
    }
    setNow(Date.now());
  }, [viewerId]);

  useEffect(() => {
    queueMicrotask(load);
    const supabase = createClient();
    const channel = supabase.channel(`friends-${viewerId}`).on("postgres_changes", { event: "*", schema: "public", table: "friendships" }, () => load()).subscribe();
    const timer = setInterval(() => document.visibilityState === "visible" && load(), 20_000);
    return () => {
      clearInterval(timer);
      supabase.removeChannel(channel);
    };
  }, [viewerId, load]);

  const supabase = createClient();
  const run = async (fn: () => PromiseLike<{ error: { message?: string; code?: string } | null }>, ok?: string) => {
    setBusy(true); setError(null); setInfo(null);
    const { error: err } = await fn();
    setBusy(false);
    if (err) setError(friendlyError(err)); else if (ok) setInfo(ok);
    await load();
  };

  const addByName = async (e: React.FormEvent) => {
    e.preventDefault();
    const name = username.trim();
    if (!USERNAME_RE.test(name)) return setError("That isn't a valid username.");
    setBusy(true); setError(null); setInfo(null);
    const { data: target } = await supabase.from("profiles").select("id, username").eq("username", name).maybeSingle();
    if (!target) { setBusy(false); return setError(`Nobody is called ${name}.`); }
    if (target.id === viewerId) { setBusy(false); return setError("That's you."); }
    const { error: err } = await supabase.from("friendships").insert({ requester_id: viewerId, addressee_id: target.id });
    setBusy(false);
    if (err) return setError(err.code === "23505" ? "You already have a request or friendship with them." : friendlyError(err));
    setInfo(`Friend request sent to ${target.username}.`);
    setUsername("");
    load();
  };

  const other = (r: Row) => (r.requester_id === viewerId ? r.addressee : r.requester);
  const friends = (rows ?? []).filter((r) => r.status === "accepted").sort((a, b) => Number(isOnline(seen[other(b).id], now)) - Number(isOnline(seen[other(a).id], now)) || other(a).username.localeCompare(other(b).username));
  const incoming = (rows ?? []).filter((r) => r.status === "pending" && r.addressee_id === viewerId);
  const outgoing = (rows ?? []).filter((r) => r.status === "pending" && r.requester_id === viewerId);
  const gameOf = (id: string) => playing.find((g) => g.white_id === id || g.black_id === id);

  return (
    <div className="grid min-w-0 gap-6">
      {error && <p role="alert" className={ui.err}>{error}</p>}
      {info && <p role="status" className={ui.info}>{info}</p>}

      <form onSubmit={addByName} className={`${ui.card} flex flex-wrap items-end gap-3`}>
        <label className="grid min-w-0 flex-1 gap-1.5 text-sm font-medium">Add a friend by username
          <input className={ui.field} value={username} onChange={(e) => setUsername(e.target.value)} autoCapitalize="none" spellCheck={false} maxLength={20} />
        </label>
        <button disabled={busy || !username.trim()} className={ui.primary}>Send request</button>
      </form>

      {incoming.length > 0 && (
        <section aria-labelledby="req" className="grid gap-2">
          <h2 id="req" className="text-xl font-bold">Friend requests</h2>
          <ul className="grid gap-2">
            {incoming.map((r) => (
              <li key={r.id} className={`${ui.card} flex flex-wrap items-center gap-3 py-3`}>
                <Link href={`/u/${r.requester.username}`} className="font-semibold hover:underline">{r.requester.username}</Link>
                <span className="ml-auto flex gap-2">
                  <button disabled={busy} className={ui.primary} onClick={() => run(() => supabase.from("friendships").update({ status: "accepted" }).eq("id", r.id))}>Accept</button>
                  <button disabled={busy} className={ui.btn} onClick={() => run(() => supabase.from("friendships").delete().eq("id", r.id))}>Decline</button>
                </span>
              </li>
            ))}
          </ul>
        </section>
      )}

      <section aria-labelledby="fr" className="grid gap-2">
        <h2 id="fr" className="text-xl font-bold">Friends {rows && <span className="text-base font-normal text-dim">({friends.length})</span>}</h2>
        {rows === null ? (
          <div className="h-24 animate-pulse rounded-xl bg-surface" />
        ) : friends.length === 0 ? (
          <p className="text-sm text-dim">No friends yet. Add a classmate above, or open their profile and press Add friend.</p>
        ) : (
          <ul className="grid gap-2">
            {friends.map((r) => {
              const o = other(r);
              const on = isOnline(seen[o.id], now);
              const g = gameOf(o.id);
              return (
                <li key={r.id} className={`${ui.card} flex flex-wrap items-center gap-3 py-3`}>
                  <span aria-label={on ? "online" : "offline"} className={`size-2.5 shrink-0 rounded-full ${on ? "bg-brand" : "bg-line"}`} />
                  <Link href={`/u/${o.username}`} className="font-semibold hover:underline">{o.username}</Link>
                  <span className="text-xs text-dim">{g ? "playing now" : on ? "online" : "offline"}</span>
                  <span className="ml-auto flex flex-wrap gap-2">
                    {g && <Link href={`/game/${g.id}`} className={ui.btn}>Watch</Link>}
                    <Link href={`/play/online?challenge=${o.username}`} className={ui.btn}>Challenge</Link>
                    <button disabled={busy} className="text-xs text-dim hover:text-ink" onClick={() => run(() => supabase.from("friendships").delete().eq("id", r.id))}>Remove</button>
                  </span>
                </li>
              );
            })}
          </ul>
        )}
      </section>

      {outgoing.length > 0 && (
        <section aria-labelledby="out" className="grid gap-2">
          <h2 id="out" className="text-lg font-bold">Waiting for an answer</h2>
          <ul className="grid gap-2">
            {outgoing.map((r) => (
              <li key={r.id} className="flex items-center gap-3 text-sm">
                <Link href={`/u/${r.addressee.username}`} className="font-semibold hover:underline">{r.addressee.username}</Link>
                <button disabled={busy} className="text-xs text-dim hover:text-ink" onClick={() => run(() => supabase.from("friendships").delete().eq("id", r.id))}>Cancel</button>
              </li>
            ))}
          </ul>
        </section>
      )}

      {blocks.length > 0 && (
        <section aria-labelledby="bl" className="grid gap-2">
          <h2 id="bl" className="text-lg font-bold">Blocked</h2>
          <ul className="grid gap-2">
            {blocks.map((b) => (
              <li key={b.blocked_id} className="flex items-center gap-3 text-sm">
                <span className="font-semibold">{b.username}</span>
                <button disabled={busy} className={ui.btn} onClick={() => run(() => supabase.from("blocks").delete().eq("blocked_id", b.blocked_id))}>Unblock</button>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
