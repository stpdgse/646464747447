"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { plural } from "@/lib/ratings";
import { createClient } from "@/lib/supabase/client";

type Item = {
  id: string; bot_name: string; bot_rating: number; variant: string; moves: string[]; status: "playing" | "finished"; result: string | null; updated_at: string;
  player: { username: string } | null;
};

const SELECT = "id, bot_name, bot_rating, variant, moves, status, result, updated_at, player:profiles(username)";

/** Games people are playing against the computer right now, plus a few that just finished. */
export function BotGameList() {
  const [live, setLive] = useState<Item[] | null>(null);
  const [recent, setRecent] = useState<Item[]>([]);

  useEffect(() => {
    const supabase = createClient();
    let dead = false;
    const load = async () => {
      const fresh = new Date(Date.now() - 10 * 60_000).toISOString(); // no move for 10 minutes = abandoned
      const hour = new Date(Date.now() - 60 * 60_000).toISOString();
      const [a, b] = await Promise.all([
        supabase.from("bot_games").select(SELECT).eq("status", "playing").gt("updated_at", fresh).order("updated_at", { ascending: false }).limit(30),
        supabase.from("bot_games").select(SELECT).eq("status", "finished").gt("updated_at", hour).order("updated_at", { ascending: false }).limit(8),
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
      <Link href={`/watch/bot/${g.id}`} className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-line bg-surface px-4 py-3 transition hover:border-brand">
        <span className="font-semibold">{g.player?.username ?? "?"} <span className="font-normal text-dim">vs</span> {g.bot_name} <span className="font-normal text-dim">(bot, ≈{g.bot_rating})</span></span>
        <span className="text-sm text-dim">
          {g.variant === "chess960" ? "Chess960 · " : ""}{plural(Math.ceil(g.moves.length / 2), "move")}{g.result ? ` · ${g.result}` : ""}
        </span>
      </Link>
    </li>
  );

  return (
    <div className="grid gap-8">
      <section aria-labelledby="botlive" className="grid gap-3">
        <h2 id="botlive" className="text-xl font-bold">Playing the computer</h2>
        {live === null ? <div className="h-16 animate-pulse rounded-xl bg-surface" /> : live.length === 0 ? (
          <p className="text-sm text-dim">Nobody is playing a bot right now.</p>
        ) : <ul className="grid gap-2">{live.map(row)}</ul>}
      </section>
      {recent.length > 0 && (
        <section aria-labelledby="botrecent" className="grid gap-3">
          <h2 id="botrecent" className="text-xl font-bold">Just finished against the computer</h2>
          <ul className="grid gap-2">{recent.map(row)}</ul>
        </section>
      )}
    </div>
  );
}
