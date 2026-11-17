"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { createClient } from "@/lib/supabase/client";

type Item = { id: string; name: string; format: string; best_of: number; bots: { id: string; name: string }[]; status: "running" | "finished"; champion_id: string | null; creator: { username: string } | null };

/** Bot tournaments running right now (and a couple that just finished) for the Watch tab. */
export function BotTournamentWatch() {
  const [items, setItems] = useState<Item[] | null>(null);
  useEffect(() => {
    let dead = false;
    const load = async () => {
      const day = new Date(Date.now() - 24 * 3600_000).toISOString();
      const { data } = await createClient().from("bot_tournaments").select("id, name, format, best_of, bots, status, champion_id, creator:profiles!bot_tournaments_creator_id_fkey(username)")
        .gt("updated_at", day).order("updated_at", { ascending: false }).limit(8);
      if (!dead) setItems((data as unknown as Item[]) ?? []);
    };
    queueMicrotask(load);
    const timer = setInterval(() => document.visibilityState === "visible" && load(), 8000);
    return () => {
      dead = true;
      clearInterval(timer);
    };
  }, []);
  return (
    <section aria-labelledby="bottour" className="grid gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 id="bottour" className="text-xl font-bold">Bot tournaments</h2>
        <Link href="/bot-tournaments" className="rounded-md bg-brand px-3 py-1.5 text-sm font-semibold text-brand-ink hover:bg-brand-hi">All tournaments / start one</Link>
      </div>
      {items === null ? <div className="h-16 animate-pulse rounded-xl bg-surface" /> : items.length === 0 ? (
        <p className="text-sm text-dim">No bot tournaments in the last day.</p>
      ) : (
        <ul className="grid gap-2">
          {items.map((t) => (
            <li key={t.id}>
              <Link href={`/bot-tournaments/${t.id}`} className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-line bg-surface px-4 py-3 transition hover:border-brand">
                <span className="font-semibold">{t.name}</span>
                <span className="text-sm text-dim">{t.format === "knockout" ? "Knockout" : "League"} · {t.bots.length} bots · {t.status === "running" ? <strong className="text-brand">running</strong> : `finished${t.champion_id ? ` · 🏆 ${t.bots.find((b) => b.id === t.champion_id)?.name ?? ""}` : ""}`} · by {t.creator?.username ?? "?"}</span>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
