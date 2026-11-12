import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Suspense } from "react";
import { ClubView, type Club, type Member } from "@/components/social/ClubView";
import { getCurrentProfile } from "@/lib/auth";
import { createClient } from "@/lib/supabase/server";

export async function generateMetadata({ params }: PageProps<"/clubs/[slug]">): Promise<Metadata> {
  const { slug } = await params;
  return { title: `Club: ${slug}` };
}

export default function ClubPage({ params }: PageProps<"/clubs/[slug]">) {
  return (
    <Suspense fallback={<div className="h-64 animate-pulse rounded-xl bg-surface" />}>
      <Loader params={params} />
    </Suspense>
  );
}

async function Loader({ params }: { params: PageProps<"/clubs/[slug]">["params"] }) {
  const { slug } = await params;
  const supabase = await createClient();
  const { data: club } = await supabase.from("clubs").select("id, slug, name, description, owner_id, open, archived").eq("slug", slug).maybeSingle();
  if (!club) notFound();
  const me = await getCurrentProfile();
  const { data: members } = await supabase
    .from("club_members")
    .select("user_id, role, status, joined_at, user:profiles(username)")
    .eq("club_id", club.id)
    .order("joined_at", { ascending: true })
    .limit(500);
  return <ClubView club={club as Club} initialMembers={(members ?? []) as unknown as Member[]} viewerId={me?.id ?? null} isMod={!!me && (me.is_admin || me.is_moderator)} />;
}
