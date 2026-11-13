import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Suspense } from "react";
import { TournamentLive, type T } from "@/components/social/TournamentLive";
import { getCurrentProfile } from "@/lib/auth";
import { createClient } from "@/lib/supabase/server";

export const metadata: Metadata = { title: "Tournament" };
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export default function TournamentPage({ params }: PageProps<"/tournaments/[id]">) {
  return (
    <Suspense fallback={<div className="h-96 animate-pulse rounded-xl bg-surface" />}>
      <Loader params={params} />
    </Suspense>
  );
}

async function Loader({ params }: { params: PageProps<"/tournaments/[id]">["params"] }) {
  const { id } = await params;
  if (!UUID.test(id)) notFound();
  const supabase = await createClient();
  const { data } = await supabase.from("tournaments").select("id, name, description, variant, time_initial_ms, time_inc_ms, rated, starts_at, ends_at, created_by").eq("id", id).maybeSingle();
  if (!data) notFound();
  const me = await getCurrentProfile();
  return <TournamentLive tournament={data as T} viewerId={me?.id ?? null} isMod={!!me && (me.is_admin || me.is_moderator)} />;
}
