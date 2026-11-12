import type { Metadata } from "next";
import { Suspense } from "react";
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
  return <LeaderboardTabs rows={rows} me={me?.id ?? null} />;
}
