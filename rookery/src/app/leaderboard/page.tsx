import type { Metadata } from "next";
import { Suspense } from "react";
import Link from "next/link";
import { LeaderboardTabs, type LeaderRow } from "@/components/ratings/LeaderboardTabs";
import { getCurrentProfile } from "@/lib/auth";
import { PERFS } from "@/lib/ratings";
import { createClient } from "@/lib/supabase/server";

export const metadata: Metadata = { title: "Leaderboard", description: "The strongest players in the class, by rating." };

export default function LeaderboardPage() {
  return (
    <div className="grid gap-4">
      <h1 className="text-3xl font-extrabold tracking-tight">Leaderboard</h1>
      <Suspense fallback={<div className="h-64 animate-pulse rounded-xl bg-surface" />}>
        <Board />
      </Suspense>
    </div>
  );
}

async function Board() {
  const supabase = await createClient();
  const me = await getCurrentProfile();
  const results = await Promise.all(
    PERFS.map((p) =>
      supabase
        .from("ratings")
        .select("user_id, perf, rating, rd, games, player:profiles!ratings_user_id_fkey(username)")
        .eq("perf", p.id)
        .order("rating", { ascending: false })
        .limit(25),
    ),
  );
  const rows: LeaderRow[] = results.flatMap((r) =>
    ((r.data ?? []) as unknown as (Omit<LeaderRow, "username"> & { player: { username: string } | null })[]).map(({ player, ...rest }) => ({
      ...rest,
      username: player?.username ?? "?",
    })),
  );
  const { data: rush } = await supabase.from("rush_scores").select("score, player:profiles!rush_scores_user_id_fkey(username)").order("score", { ascending: false }).limit(80);
  const seen = new Set<string>();
  const top: { username: string; score: number }[] = [];
  for (const r of (rush ?? []) as unknown as { score: number; player: { username: string } | null }[]) {
    const name = r.player?.username ?? "?";
    if (seen.has(name)) continue;
    seen.add(name);
    top.push({ username: name, score: r.score });
    if (top.length === 10) break;
  }
  return (
    <div className="grid gap-10">
      <LeaderboardTabs rows={rows} me={me?.id ?? null} />
      <section aria-labelledby="rush" className="grid gap-3">
        <h2 id="rush" className="text-xl font-bold">Puzzle Rush: best scores</h2>
        {top.length === 0 ? (
          <p className="text-sm text-dim">No scores yet. <Link href="/puzzles/rush" className="font-semibold text-brand underline">Try Puzzle Rush.</Link></p>
        ) : (
          <ol className="grid max-w-md gap-1.5">
            {top.map((t, i) => (
              <li key={t.username} className="flex items-center justify-between rounded-lg border border-line bg-surface px-4 py-2 text-sm">
                <span><span className="mr-3 text-dim">{i + 1}</span><Link href={`/u/${t.username}`} className="font-semibold hover:text-brand underline">{t.username}</Link></span>
                <span className="font-bold tabular-nums">{t.score}</span>
              </li>
            ))}
          </ol>
        )}
      </section>
    </div>
  );
}
