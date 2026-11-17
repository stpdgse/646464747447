import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Suspense } from "react";
import { BotTournamentLive } from "@/components/computer/BotTournamentLive";
import { getCurrentProfile } from "@/lib/auth";
import { GAME_SELECT, TOURNAMENT_SELECT, type BotTournamentRow, type StoredGame } from "@/lib/botTournamentRows";
import { createClient } from "@/lib/supabase/server";

export const metadata: Metadata = { title: "Bot tournament" };
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export default function BotTournamentPage({ params }: PageProps<"/bot-tournaments/[id]">) {
  return (
    <Suspense fallback={<div className="h-96 animate-pulse rounded-xl bg-surface" />}>
      <Loader params={params} />
    </Suspense>
  );
}

async function Loader({ params }: { params: PageProps<"/bot-tournaments/[id]">["params"] }) {
  const { id } = await params;
  if (!UUID.test(id)) notFound();
  const supabase = await createClient();
  const [{ data: t }, { data: games }] = await Promise.all([
    supabase.from("bot_tournaments").select(TOURNAMENT_SELECT).eq("id", id).maybeSingle(),
    supabase.from("bot_tournament_games").select(GAME_SELECT).eq("tournament_id", id).order("created_at", { ascending: true }),
  ]);
  if (!t) notFound();
  const me = await getCurrentProfile();
  return <BotTournamentLive initial={t as unknown as BotTournamentRow} initialGames={(games ?? []) as StoredGame[]} viewerId={me?.id ?? null} isMod={!!me && (me.is_admin || me.is_moderator)} />;
}
