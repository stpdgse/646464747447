import type { Metadata } from "next";
import Link from "next/link";
import { Suspense } from "react";
import { getCurrentProfile } from "@/lib/auth";
import { formatRating, plural, signed } from "@/lib/ratings";
import { createClient } from "@/lib/supabase/server";

export const metadata: Metadata = { title: "Puzzles", description: "Rated tactics, Puzzle Rush and a daily puzzle." };

const MODES = [
  { href: "/puzzles/train", title: "Train", body: "Rated puzzles matched to your level. Pick a theme or take whatever comes." },
  { href: "/puzzles/daily", title: "Daily puzzle", body: "One puzzle a day, the same for the whole class." },
  { href: "/puzzles/rush", title: "Puzzle Rush", body: "Three minutes, three strikes. How many can you solve?" },
];

export default function PuzzlesPage() {
  return (
    <div className="grid gap-8">
      <h1 className="text-3xl font-extrabold tracking-tight">Puzzles</h1>
      <div className="grid gap-4 sm:grid-cols-3">
        {MODES.map((m) => (
          <Link key={m.href} href={m.href} className="rounded-xl border border-brand bg-surface p-5 transition hover:bg-surface-2">
            <h2 className="text-lg font-bold">{m.title}</h2>
            <p className="mt-1 text-sm text-dim">{m.body}</p>
          </Link>
        ))}
      </div>
      <Suspense fallback={<div className="h-40 animate-pulse rounded-xl bg-surface" />}>
        <Progress />
      </Suspense>
    </div>
  );
}

async function Progress() {
  const me = await getCurrentProfile();
  const supabase = await createClient();
  const [rating, attempts, rush] = await Promise.all([
    me ? supabase.from("ratings").select("rating, rd, games").eq("user_id", me.id).eq("perf", "puzzle").maybeSingle() : Promise.resolve({ data: null }),
    me ? supabase.from("puzzle_attempts").select("puzzle_id, puzzle_rating, solved, rating_before, rating_after, created_at").eq("user_id", me.id).order("created_at", { ascending: false }).limit(10) : Promise.resolve({ data: null }),
    supabase.from("rush_scores").select("score, created_at, player:profiles!rush_scores_user_id_fkey(username)").order("score", { ascending: false }).limit(60),
  ]);

  const seen = new Set<string>();
  const top: { username: string; score: number }[] = [];
  for (const r of (rush.data ?? []) as unknown as { score: number; player: { username: string } | null }[]) {
    const name = r.player?.username ?? "?";
    if (seen.has(name)) continue;
    seen.add(name);
    top.push({ username: name, score: r.score });
    if (top.length === 5) break;
  }
  const r = rating.data as { rating: number; rd: number; games: number } | null;

  return (
    <div className="grid gap-8 lg:grid-cols-2">
      <section aria-labelledby="mine" className="grid content-start gap-3">
        <h2 id="mine" className="text-xl font-bold">Your puzzles</h2>
        {!me ? (
          <p className="text-sm text-dim"><Link href="/login" className="font-semibold text-brand hover:underline">Log in</Link> to get a puzzle rating and track your progress.</p>
        ) : !r ? (
          <p className="text-sm text-dim">Solve your first rated puzzle to get a rating.</p>
        ) : (
          <>
            <p><span className="text-3xl font-extrabold tabular-nums">{formatRating(r.rating, r.rd)}</span> <span className="text-dim">puzzle rating · {plural(r.games, "puzzle")} solved or missed</span></p>
            <ul className="grid gap-1.5">
              {((attempts.data ?? []) as { puzzle_id: string; puzzle_rating: number; solved: boolean; rating_before: number; rating_after: number }[]).map((a) => (
                <li key={a.puzzle_id} className="flex items-center justify-between rounded-lg border border-line bg-surface px-3 py-2 text-sm">
                  <span className={a.solved ? "font-semibold text-brand-hi" : "font-semibold text-danger"}>{a.solved ? "Solved" : "Missed"}</span>
                  <span className="text-dim">puzzle {a.puzzle_rating}</span>
                  <span className="tabular-nums">{signed(a.rating_after - a.rating_before)}</span>
                </li>
              ))}
            </ul>
          </>
        )}
      </section>
      <section aria-labelledby="rush" className="grid content-start gap-3">
        <h2 id="rush" className="text-xl font-bold">Puzzle Rush: best scores</h2>
        {top.length === 0 ? (
          <p className="text-sm text-dim">No scores yet. <Link href="/puzzles/rush" className="font-semibold text-brand hover:underline">Be the first.</Link></p>
        ) : (
          <ol className="grid gap-1.5">
            {top.map((t, i) => (
              <li key={t.username} className="flex items-center justify-between rounded-lg border border-line bg-surface px-3 py-2 text-sm">
                <span><span className="mr-3 text-dim">{i + 1}</span><Link href={`/u/${t.username}`} className="font-semibold hover:text-brand hover:underline">{t.username}</Link></span>
                <span className="font-bold tabular-nums">{t.score}</span>
              </li>
            ))}
          </ol>
        )}
      </section>
    </div>
  );
}
