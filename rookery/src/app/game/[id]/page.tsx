import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Suspense } from "react";
import { LiveGame } from "@/components/live/LiveGame";
import { getCurrentProfile } from "@/lib/auth";
import type { GameRow } from "@/lib/live/types";
import { createClient } from "@/lib/supabase/server";

export const metadata: Metadata = { title: "Game" };

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export default function GamePage({ params }: PageProps<"/game/[id]">) {
  return (
    <Suspense fallback={<div className="h-96 animate-pulse rounded-xl bg-surface" />}>
      <Game params={params} />
    </Suspense>
  );
}

async function Game({ params }: { params: PageProps<"/game/[id]">["params"] }) {
  const { id } = await params;
  if (!UUID.test(id)) notFound();
  const supabase = await createClient();
  const { data } = await supabase
    .from("games")
    .select("*, white:profiles!games_white_id_fkey(username), black:profiles!games_black_id_fkey(username)")
    .eq("id", id)
    .maybeSingle();
  if (!data) notFound();
  const { white, black, ...row } = data as GameRow & { white: { username: string }; black: { username: string } };
  const me = await getCurrentProfile();
  return <LiveGame initial={row} names={{ white: white.username, black: black.username }} viewerId={me?.id ?? null} />;
}
