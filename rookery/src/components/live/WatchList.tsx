"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { VARIANTS, type Variant } from "@/lib/chess/variants";
import { describeGameTime } from "@/lib/live/timecontrols";
import { plural } from "@/lib/ratings";
import { createClient } from "@/lib/supabase/client";

type Item = {
  id: string; variant: Variant; status: string; result: string | null; moves: string[]; time_initial_ms: number; time_inc_ms: number; mode: "live" | "daily"; days_per_move: number | null;
  white: { username: string } | null; black: { username: string } | null;
};

const SELECT = "id, variant, status, result, moves, time_initial_ms, time_inc_ms, mode, days_per_move, white:profiles!games_white_id_fkey(username), black:profiles!games_black_id_fkey(username)";

export function WatchList() {
  const [live, setLive] = useState<Item[] | null>(null);
  const [recent, setRecent] = useState<Item[]>([]);

  useEffect(() => {
    const supabase = createClient();
    let dead = false;
    const load = async () => {
      const [a, b] = await Promise.all([
        supabase.from("games").select(SELECT).eq("status", "started").eq("mode", "live").order("created_at", { ascending: false }).limit(30),
        supabase.from("games").select(SELECT).eq("status", "finished").order("ended_at", { ascending: false }).limit(10),
      ]);
      if (dead) return;
      setLive((a.data as unknown as Item[]) ?? []);
      setRecent((b.data as unknown as Item[]) ?? []);
    };
    load();
    const timer = setInterval(() => document.visibilityState === "visible" && load(), 5000);
    return () => {
      dead = true;
      clearInterval(timer);
    };
  }, []);

  const row = (g: Item) => (
    <li key={g.id}>
      <Link href={`/game/${g.id}`} className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-line bg-surface px-4 py-3 transition hover:border-brand">
        <span className="font-semibold">{g.white?.username ?? "?"} <span className="font-normal text-dim">vs</span> {g.black?.username ?? "?"}</span>
        <span className="text-sm text-dim">
          {describeGameTime(g)} · {VARIANTS[g.variant].label} · {plural(Math.ceil(g.moves.length / 2), "move")}{g.result ? ` · ${g.result}` : ""}
        </span>
      </Link>
    </li>
  );

  return (
    <div className="grid gap-8">
      <section aria-labelledby="live" className="grid gap-3">
        <h2 id="live" className="text-xl font-bold">Live now</h2>
        {live === null ? <div className="h-20 animate-pulse rounded-xl bg-surface" /> : live.length === 0 ? (
          <p className="text-sm text-dim">No games are being played right now. <Link href="/play/online" className="font-semibold text-brand underline">Start one</Link>.</p>
        ) : <ul className="grid gap-2">{live.map(row)}</ul>}
      </section>
      {recent.length > 0 && (
        <section aria-labelledby="recent" className="grid gap-3">
          <h2 id="recent" className="text-xl font-bold">Just finished</h2>
          <ul className="grid gap-2">{recent.map(row)}</ul>
        </section>
      )}
    </div>
  );
}
