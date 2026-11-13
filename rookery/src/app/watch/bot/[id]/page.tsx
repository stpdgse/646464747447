import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Suspense } from "react";
import { BotSpectator, type BotGameRow } from "@/components/live/BotSpectator";
import { createClient } from "@/lib/supabase/server";

export const metadata: Metadata = { title: "Watching a bot game" };
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export default function BotGamePage({ params }: PageProps<"/watch/bot/[id]">) {
  return (
    <Suspense fallback={<div className="h-96 animate-pulse rounded-xl bg-surface" />}>
      <Loader params={params} />
    </Suspense>
  );
}

async function Loader({ params }: { params: PageProps<"/watch/bot/[id]">["params"] }) {
  const { id } = await params;
  if (!UUID.test(id)) notFound();
  const supabase = await createClient();
  const { data } = await supabase
    .from("bot_games")
    .select("id, bot_id, bot_name, bot_rating, player_color, variant, initial_fen, moves, status, result, reason, updated_at, player:profiles(username)")
    .eq("id", id)
    .maybeSingle();
  if (!data) notFound();
  return <BotSpectator initial={data as unknown as BotGameRow} />;
}
