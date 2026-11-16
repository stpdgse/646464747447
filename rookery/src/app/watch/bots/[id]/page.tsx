import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Suspense } from "react";
import { BotMatchSpectator } from "@/components/computer/BotMatchSpectator";
import { MATCH_SELECT, type BotMatchRow } from "@/lib/botMatches";
import { createClient } from "@/lib/supabase/server";

export const metadata: Metadata = { title: "Bot battle" };
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export default function BotBattlePage({ params }: PageProps<"/watch/bots/[id]">) {
  return (
    <Suspense fallback={<div className="h-96 animate-pulse rounded-xl bg-surface" />}>
      <Loader params={params} />
    </Suspense>
  );
}

async function Loader({ params }: { params: PageProps<"/watch/bots/[id]">["params"] }) {
  const { id } = await params;
  if (!UUID.test(id)) notFound();
  const { data } = await (await createClient()).from("bot_matches").select(MATCH_SELECT).eq("id", id).maybeSingle();
  if (!data) notFound();
  return <BotMatchSpectator initial={data as unknown as BotMatchRow} />;
}
