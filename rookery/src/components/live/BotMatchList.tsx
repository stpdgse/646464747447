"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { plural } from "@/lib/ratings";
import { createClient } from "@/lib/supabase/client";

type Item = {
  id: string; white_bot_name: string; white_bot_rating: number; black_bot_name: string; black_bot_rating: number; moves: string[]; status: "playing" | "finished";
  result: string | null; variant: string; creator: { username: string } | null;
};
const SELECT = "id, white_bot_name, white_bot_rating, black_bot_name, black_bot_rating, moves, status, result, variant, creator:profiles(username)";

/** Bot battles happening now (and a few that just ended), plus a way to start one. */
export function BotMatchList() {
  const [live, setLive] = useState<Item[] | null>(null);
  const [recent, setRecent] = useState<Item[]>([]);

  useEffect(() => {
    const supabase = createClient();
    let dead = false;
    const load = async () => {
      const fresh = new Date(Date.now() - 3 * 60_000).toISOString(); // a host who left stops showing after three minutes
      const hour = new Date(Date.now() - 60 * 60_000).toISOString();
      const [a, b] = await Promise.all([
        supabase.from("bot_matches").select(SELECT).eq("status", "playing").gt("updated_at", fresh).order("updated_at", { ascending: false }).limit(20),
        supabase.from("bot_matches").select(SELECT).eq("status", "finished").gt("updated_at", hour).order("updated_at", { ascending: false }).limit(6),
      ]);
      if (dead) return;
      setLive((a.data as unknown as Item[]) ?? []);
      setRecent((b.data as unknown as Item[]) ?? []);
    };
    queueMicrotask(load);
    const timer = setInterval(() => document.visibilityState === "visible" && load(), 5000);
    return () => {
      dead = true;
      clearInterval(timer);
    };
  }, []);

  const row = (g: Item) => (
    <li key={g.id}>
      <Link href={`/watch/bots/${g.id}`} className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-line bg-surface px-4 py-3 transition hover:border-brand">
        <span className="font-semibold">{g.white_bot_name} <span className="font-normal text-dim">(≈{g.white_bot_rating}) vs</span> {g.black_bot_name} <span className="font-normal text-dim">(≈{g.black_bot_rating})</span></span>
        <span className="text-sm text-dim">{g.variant === "chess960" ? "Chess960 · " : ""}{plural(Math.ceil(g.moves.length / 2), "move")}{g.result ? ` · ${g.result}` : ""} · hosted by {g.creator?.username ?? "?"}</span>
      </Link>
    </li>
  );

  return (
    <section aria-labelledby="botbattles" className="grid gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 id="botbattles" className="text-xl font-bold">Bot battles</h2>
        <Link href="/play/bots" className="rounded-md bg-brand px-3 py-1.5 text-sm font-semibold text-brand-ink hover:bg-brand-hi">Start a bot battle</Link>
      </div>
      {live === null ? <div className="h-16 animate-pulse rounded-xl bg-surface" /> : live.length === 0 ? (
        <p className="text-sm text-dim">No bots are fighting right now.</p>
      ) : <ul className="grid gap-2">{live.map(row)}</ul>}
      {recent.length > 0 && (
        <>
          <h3 className="text-sm font-bold uppercase tracking-wide text-dim">Just finished</h3>
          <ul className="grid gap-2">{recent.map(row)}</ul>
        </>
      )}
    </section>
  );
}
