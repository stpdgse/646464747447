import type { Metadata } from "next";
import Link from "next/link";
import { Suspense } from "react";
import { BotTournamentCreate } from "@/components/computer/BotTournamentCreate";
import { getCurrentProfile } from "@/lib/auth";
import { botElo } from "@/lib/botTournament";
import { ui } from "@/lib/social";
import { createClient } from "@/lib/supabase/server";

export const metadata: Metadata = { title: "Bot tournaments", description: "Knockout and league tournaments between bots. Anyone can start one, everyone can watch." };

export default function BotTournamentsPage() {
  return (
    <div className="grid gap-8">
      <div>
        <h1 className="text-3xl font-extrabold tracking-tight">Bot tournaments</h1>
        <p className="mt-1 text-dim">Pick the bots, start a knockout or a league, and watch them fight their way to the title. Anyone can start one and everyone can watch.</p>
      </div>
      <Suspense fallback={<div className="h-48 animate-pulse rounded-xl bg-surface" />}>
        <Content />
      </Suspense>
    </div>
  );
}

type Row = { id: string; name: string; format: string; best_of: number; status: string; champion_id: string | null; bots: { id: string; name: string; rating: number }[]; updated_at: string; creator: { username: string } | null };

async function Content() {
  const supabase = await createClient();
  const me = await getCurrentProfile();
  const [{ data: ts }, { data: gs }] = await Promise.all([
    supabase.from("bot_tournaments").select("id, name, format, best_of, status, champion_id, bots, updated_at, creator:profiles!bot_tournaments_creator_id_fkey(username)").order("created_at", { ascending: false }).limit(60),
    supabase.from("bot_tournament_games").select("white_bot_id, black_bot_id, result, created_at").eq("status", "finished").order("created_at", { ascending: true }).limit(4000),
  ]);
  const rows = (ts ?? []) as unknown as Row[];
  const running = rows.filter((t) => t.status === "running");
  const finished = rows.filter((t) => t.status === "finished").slice(0, 12);

  // Bot league: Elo from every finished tournament game, starting from each bot's rating label.
  const start: Record<string, number> = {};
  const names: Record<string, string> = {};
  for (const t of [...rows].reverse()) for (const b of t.bots) { start[b.id] = b.rating; names[b.id] = b.name; }
  const elo = botElo(start, (gs ?? []) as { white_bot_id: string; black_bot_id: string; result: string | null }[]);
  const league = Object.entries(elo).filter(([, v]) => v.games > 0).sort((a, b) => b[1].elo - a[1].elo).slice(0, 15);

  const card = (t: Row) => (
    <li key={t.id}>
      <Link href={`/bot-tournaments/${t.id}`} className={`grid h-full content-start gap-1 rounded-xl border bg-surface p-5 transition hover:bg-surface-2 ${t.status === "running" ? "border-brand" : "border-line"}`}>
        <h3 className="text-lg font-bold">{t.name}</h3>
        <p className="text-sm text-dim">{t.format === "knockout" ? `Knockout · best of ${t.best_of}` : "Round-robin"} · {t.bots.length} bots · by {t.creator?.username ?? "?"}</p>
        {t.status === "finished" && <p className="text-sm font-semibold">🏆 {t.bots.find((b) => b.id === t.champion_id)?.name ?? "—"}</p>}
        {t.status === "running" && <p className="text-xs font-semibold text-brand">Running now · click to watch</p>}
      </Link>
    </li>
  );

  return (
    <>
      {running.length > 0 && (
        <section aria-labelledby="run" className="grid gap-3">
          <h2 id="run" className="text-xl font-bold">Running now</h2>
          <ul className="grid gap-3 sm:grid-cols-2">{running.map(card)}</ul>
        </section>
      )}
      {me ? <BotTournamentCreate userId={me.id} /> : <p className={ui.info}><Link className="text-brand underline" href="/login">Log in</Link> to start a bot tournament. Everyone can watch without logging in.</p>}
      {finished.length > 0 && (
        <section aria-labelledby="done" className="grid gap-3">
          <h2 id="done" className="text-xl font-bold">Finished</h2>
          <ul className="grid gap-3 sm:grid-cols-2">{finished.map(card)}</ul>
        </section>
      )}
      <section aria-labelledby="league" className="grid gap-3">
        <h2 id="league" className="text-xl font-bold">Bot league</h2>
        <p className="text-sm text-dim">Every bot starts at its rating, and every tournament game moves it up or down, like Elo for bots.</p>
        {league.length === 0 ? <p className="text-sm text-dim">No tournament games yet.</p> : (
          <div className="overflow-x-auto rounded-xl border border-line">
            <table className="w-full min-w-[24rem] text-sm">
              <thead className="bg-surface-2 text-left text-xs uppercase text-dim"><tr><th className="px-3 py-2">#</th><th className="px-3 py-2">Bot</th><th className="px-3 py-2 text-right">Elo</th><th className="px-3 py-2 text-right">Games</th></tr></thead>
              <tbody>
                {league.map(([id, v], i) => (
                  <tr key={id} className="border-t border-line">
                    <td className="px-3 py-2 tabular-nums">{i + 1}</td>
                    <td className="px-3 py-2 font-semibold">{names[id] ?? id}</td>
                    <td className="px-3 py-2 text-right font-bold tabular-nums">{Math.round(v.elo)}</td>
                    <td className="px-3 py-2 text-right tabular-nums">{v.games}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </>
  );
}
