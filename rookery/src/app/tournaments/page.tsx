import type { Metadata } from "next";
import Link from "next/link";
import { Suspense } from "react";
import { TournamentCreate } from "@/components/social/TournamentCreate";
import { getCurrentProfile } from "@/lib/auth";
import { describeTimeControl } from "@/lib/live/timecontrols";
import { fmtDate, ui } from "@/lib/social";
import { createClient } from "@/lib/supabase/server";

export const metadata: Metadata = { title: "Tournaments", description: "Arena tournaments for the class." };

export default function TournamentsPage() {
  return (
    <div className="grid gap-6">
      <div>
        <h1 className="text-3xl font-extrabold tracking-tight">Tournaments</h1>
        <p className="mt-1 text-dim">Join an arena, press Play to be paired with another player, and play as many games as you can before the time is up. A win is 2 points, a draw 1.</p>
      </div>
      <Suspense fallback={<div className="h-48 animate-pulse rounded-xl bg-surface" />}>
        <List />
      </Suspense>
    </div>
  );
}

type Row = { id: string; name: string; variant: string; time_initial_ms: number; time_inc_ms: number; rated: boolean; starts_at: string; ends_at: string; players: { count: number }[] };

async function List() {
  const supabase = await createClient();
  const me = await getCurrentProfile();
  const { data } = await supabase
    .from("tournaments")
    .select("id, name, variant, time_initial_ms, time_inc_ms, rated, starts_at, ends_at, players:tournament_players(count)")
    .order("starts_at", { ascending: false })
    .limit(40);
  const rows = (data ?? []) as unknown as Row[];
  // Compare against one clock reading taken here, on the server, for this request.
  const now = await supabase.rpc("server_time").then((r) => (typeof r.data === "string" ? new Date(r.data).getTime() : 0));
  const state = (t: Row) => (now >= new Date(t.ends_at).getTime() ? "finished" : now >= new Date(t.starts_at).getTime() ? "running" : "upcoming");
  const groups = [
    { id: "running", title: "Running now", items: rows.filter((t) => state(t) === "running") },
    { id: "upcoming", title: "Starting soon", items: rows.filter((t) => state(t) === "upcoming").reverse() },
    { id: "finished", title: "Finished", items: rows.filter((t) => state(t) === "finished").slice(0, 10) },
  ];
  return (
    <>
      {groups.map((g) => g.items.length > 0 && (
        <section key={g.id} aria-labelledby={`g-${g.id}`} className="grid gap-3">
          <h2 id={`g-${g.id}`} className="text-xl font-bold">{g.title}</h2>
          <ul className="grid gap-3 sm:grid-cols-2">
            {g.items.map((t) => (
              <li key={t.id}>
                <Link href={`/tournaments/${t.id}`} className={`grid h-full content-start gap-1 rounded-xl border bg-surface p-5 transition hover:bg-surface-2 ${g.id === "running" ? "border-brand" : "border-line"}`}>
                  <h3 className="text-lg font-bold">{t.name}</h3>
                  <p className="text-sm text-dim">{describeTimeControl(t.time_initial_ms, t.time_inc_ms)} · {t.variant === "chess960" ? "Chess960" : "Standard"} · {t.rated ? "Rated" : "Casual"}</p>
                  <p className="text-xs text-dim">{fmtDate(t.starts_at)} · {Math.round((new Date(t.ends_at).getTime() - new Date(t.starts_at).getTime()) / 60000)} min · {t.players[0]?.count ?? 0} players</p>
                </Link>
              </li>
            ))}
          </ul>
        </section>
      ))}
      {rows.length === 0 && <p className="text-dim">No tournaments yet.</p>}
      {me && (me.is_admin || me.is_moderator) ? <TournamentCreate userId={me.id} /> : <p className={ui.info}>Moderators and admins create tournaments. Ask one to set one up for your class.</p>}
    </>
  );
}
