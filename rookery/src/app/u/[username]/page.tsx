import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";
import { AchievementsGrid } from "@/components/ratings/AchievementsGrid";
import { BotHistory, type BotHistoryRow } from "@/components/ratings/BotHistory";
import { ageMs } from "@/lib/social";
import { RatingPanel } from "@/components/ratings/RatingPanel";
import type { ChartPoint } from "@/components/ratings/RatingChart";
import { FriendActions } from "@/components/social/FriendActions";
import { getCurrentProfile, type Profile } from "@/lib/auth";
import { VARIANTS, type Variant } from "@/lib/chess/variants";
import { describeGameTime } from "@/lib/live/timecontrols";
import { PERFS, formatRating, perfLabel, plural, tally, type HistoryPoint, type RatingRow } from "@/lib/ratings";
import { createClient } from "@/lib/supabase/server";

export async function generateMetadata({ params }: PageProps<"/u/[username]">): Promise<Metadata> {
  const { username } = await params;
  return { title: username };
}

export default function ProfilePage({ params }: PageProps<"/u/[username]">) {
  return (
    <Suspense fallback={<div className="h-40 animate-pulse rounded-xl bg-surface" />}>
      <ProfileView params={params} />
    </Suspense>
  );
}

type RecentGame = {
  id: string; variant: Variant; result: string | null; reason: string | null; rated: boolean; moves: string[];
  white_id: string; black_id: string; time_initial_ms: number; time_inc_ms: number; mode: "live" | "daily"; days_per_move: number | null; ended_at: string | null;
  white: { username: string } | null; black: { username: string } | null;
};

async function ProfileView({ params }: { params: PageProps<"/u/[username]">["params"] }) {
  const { username } = await params;
  const supabase = await createClient();
  const { data } = await supabase.from("profiles").select("*").eq("username", username).maybeSingle();
  if (!data) notFound();
  const p = data as Profile;
  const viewer = await getCurrentProfile();

  const [ratingsRes, historyRes, gamesRes, puzzleRes, botRes, badgeRes] = await Promise.all([
    supabase.from("ratings").select("user_id, perf, rating, rd, games").eq("user_id", p.id),
    supabase.from("rating_history").select("game_id, perf, rating_before, rating_after, created_at").eq("user_id", p.id).order("created_at", { ascending: true }).limit(2000),
    supabase
      .from("games")
      .select("id, variant, result, reason, rated, moves, white_id, black_id, time_initial_ms, time_inc_ms, mode, days_per_move, ended_at, white:profiles!games_white_id_fkey(username), black:profiles!games_black_id_fkey(username)")
      .eq("status", "finished")
      .or(`white_id.eq.${p.id},black_id.eq.${p.id}`)
      .order("ended_at", { ascending: false })
      .limit(500),
    supabase.from("puzzle_attempts").select("rating_before, rating_after, created_at").eq("user_id", p.id).order("created_at", { ascending: true }).limit(2000),
    supabase.from("bot_games").select("id, bot_id, bot_name, bot_rating, player_color, variant, initial_fen, moves, status, result, reason, time_control, watchable, updated_at")
      .eq("player_id", p.id).order("updated_at", { ascending: false }).limit(60),
    supabase.from("achievements").select("key, earned_at").eq("user_id", p.id),
  ]);
  const earned = Object.fromEntries(((badgeRes.data ?? []) as { key: string; earned_at: string }[]).map((b) => [b.key, b.earned_at]));
  // A game nobody has moved in for an hour was left unfinished.
  const botGames = ((botRes.data ?? []) as Omit<BotHistoryRow, "abandoned">[])
    .map((g) => ({ ...g, abandoned: g.status === "playing" && ageMs(g.updated_at) > 3600_000 }))
    .filter((g) => g.moves.length > 0 || g.status === "playing");

  const ratings = ((ratingsRes.data ?? []) as RatingRow[]).sort((a, b) => PERFS.findIndex((x) => x.id === a.perf) - PERFS.findIndex((x) => x.id === b.perf));
  const history = (historyRes.data ?? []) as HistoryPoint[];
  const games = (gamesRes.data ?? []) as unknown as RecentGame[];
  const record = tally(games, p.id);

  // One series per rating type, oldest first, starting from the first game's "before" rating.
  const series: Record<string, ChartPoint[]> = {};
  for (const perf of PERFS.map((x) => x.id)) {
    const rows = history.filter((h) => h.perf === perf);
    if (rows.length === 0) continue;
    series[perf] = [
      { rating: rows[0].rating_before, delta: 0, created_at: rows[0].created_at },
      ...rows.map((h) => ({ rating: h.rating_after, delta: h.rating_after - h.rating_before, created_at: h.created_at })),
    ];
  }

  const attempts = (puzzleRes.data ?? []) as { rating_before: number; rating_after: number; created_at: string }[];
  if (attempts.length > 0) {
    series.puzzle = [
      { rating: attempts[0].rating_before, delta: 0, created_at: attempts[0].created_at },
      ...attempts.map((a) => ({ rating: a.rating_after, delta: a.rating_after - a.rating_before, created_at: a.created_at })),
    ];
  }

  const pct = (n: number) => (record.total ? (n / record.total) * 100 : 0);

  return (
    <div className="grid gap-8">
      <article className="grid gap-3 rounded-xl border border-line bg-surface p-6">
        <header>
          <h1 className="text-3xl font-extrabold tracking-tight">{p.display_name || p.username}</h1>
          <p className="text-dim">
            @{p.username}
            {p.country ? ` · ${p.country}` : ""} · joined {new Date(p.created_at).toLocaleDateString("en", { month: "long", year: "numeric" })}
          </p>
        </header>
        {p.bio && <p className="whitespace-pre-line">{p.bio}</p>}
        {(p.is_admin || p.is_moderator) && <p className="text-xs font-semibold text-brand">{p.is_admin ? "Admin" : "Moderator"}</p>}
        <FriendActions viewerId={viewer?.id ?? null} targetId={p.id} targetName={p.username} />
        {(viewer?.is_admin || viewer?.is_moderator) && viewer.id !== p.id && (
          <a href={`/admin?tab=users&user=${p.username}`} className="justify-self-start text-xs text-dim underline hover:text-ink">Moderate this account</a>
        )}
      </article>

      <section aria-labelledby="ratings" className="grid gap-3">
        <h2 id="ratings" className="text-xl font-bold">Ratings</h2>
        {ratings.length === 0 ? (
          <p className="text-sm text-dim">No rated games yet.</p>
        ) : (
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
            {ratings.map((r) => (
              <div key={r.perf} className="rounded-xl border border-line bg-surface p-4">
                <p className="text-xs font-semibold text-dim">{perfLabel(r.perf)}</p>
                <p className="text-2xl font-extrabold tabular-nums">{formatRating(r.rating, r.rd)}</p>
                <p className="text-xs text-dim">{plural(r.games, "game")}</p>
              </div>
            ))}
          </div>
        )}
      </section>

      <section aria-labelledby="graph" className="grid gap-3">
        <h2 id="graph" className="text-xl font-bold">Rating history</h2>
        <RatingPanel series={series} />
      </section>

      <section aria-labelledby="record" className="grid gap-3">
        <h2 id="record" className="text-xl font-bold">Record</h2>
        {record.total === 0 ? (
          <p className="text-sm text-dim">No finished games yet.</p>
        ) : (
          <div className="grid gap-2">
            <div className="flex h-3 overflow-hidden rounded-full bg-surface-2" role="img" aria-label={`${plural(record.wins, "win")}, ${plural(record.draws, "draw")}, ${plural(record.losses, "loss", "losses")}`}>
              <div style={{ width: `${pct(record.wins)}%`, background: "var(--brand-hi)" }} />
              <div style={{ width: `${pct(record.draws)}%`, background: "var(--ink-dim)", marginLeft: 2 }} />
              <div style={{ width: `${pct(record.losses)}%`, background: "var(--danger)", marginLeft: 2 }} />
            </div>
            <p className="text-sm text-dim"><strong className="text-ink">{plural(record.wins, "win")}</strong> · <strong className="text-ink">{plural(record.draws, "draw")}</strong> · <strong className="text-ink">{plural(record.losses, "loss", "losses")}</strong> ({plural(record.total, "game")})</p>
          </div>
        )}
      </section>

      <section aria-labelledby="badges" className="grid gap-3">
        <h2 id="badges" className="text-xl font-bold">Achievements</h2>
        <AchievementsGrid earned={earned} />
      </section>

      <section aria-labelledby="botgames" className="grid gap-3">
        <h2 id="botgames" className="text-xl font-bold">Against the computer</h2>
        <BotHistory rows={botGames} username={p.username} isOwner={viewer?.id === p.id} />
      </section>

      <section aria-labelledby="games" className="grid gap-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 id="games" className="text-xl font-bold">Recent games</h2>
          {games.length > 0 && (
            <a href={`/u/${p.username}/pgn`} className="rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-medium hover:bg-surface-2">Download all as PGN</a>
          )}
        </div>
        {games.length === 0 ? (
          <p className="text-sm text-dim">Finished games will show up here.</p>
        ) : (
          <ul className="grid gap-2">
            {games.slice(0, 15).map((g) => {
              const mine = g.white_id === p.id ? "white" : "black";
              const opp = mine === "white" ? g.black : g.white;
              const outcome = g.result === "1/2-1/2" ? "Draw" : (g.result === "1-0") === (mine === "white") ? "Win" : "Loss";
              return (
                <li key={g.id}>
                  <Link href={`/game/${g.id}`} className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-line bg-surface px-4 py-3 transition hover:border-brand">
                    <span className="flex items-center gap-3">
                      <span className={`w-10 text-sm font-bold ${outcome === "Win" ? "text-brand-hi" : outcome === "Loss" ? "text-danger" : "text-dim"}`}>{outcome}</span>
                      <span>vs <strong>{opp?.username ?? "?"}</strong></span>
                    </span>
                    <span className="text-sm text-dim">
                      {describeGameTime(g)} · {VARIANTS[g.variant].label} · {g.rated ? "Rated" : "Casual"} · {plural(Math.ceil(g.moves.length / 2), "move")}
                    </span>
                  </Link>
                </li>
              );
            })}
          </ul>
        )}
      </section>
    </div>
  );
}
