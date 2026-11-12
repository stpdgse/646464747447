import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";
import { ThreadView, type Post, type ThreadInfo } from "@/components/social/ForumActions";
import { getCurrentProfile } from "@/lib/auth";
import { createClient } from "@/lib/supabase/server";

export const metadata: Metadata = { title: "Forum thread" };
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export default function ThreadPage({ params }: PageProps<"/forum/thread/[id]">) {
  return (
    <Suspense fallback={<div className="h-64 animate-pulse rounded-xl bg-surface" />}>
      <Loader params={params} />
    </Suspense>
  );
}

async function Loader({ params }: { params: PageProps<"/forum/thread/[id]">["params"] }) {
  const { id } = await params;
  if (!UUID.test(id)) notFound();
  const supabase = await createClient();
  const { data: thread } = await supabase
    .from("forum_threads")
    .select("id, title, pinned, locked, deleted_at, author_id, category:forum_categories(slug, name)")
    .eq("id", id)
    .maybeSingle();
  if (!thread) notFound();
  const me = await getCurrentProfile();
  const { data: posts } = await supabase
    .from("forum_posts")
    .select("id, author_id, body, created_at, edited_at, deleted_at, deleted_by_mod, author:profiles(username)")
    .eq("thread_id", id)
    .order("created_at", { ascending: true })
    .limit(300);
  const info = thread as unknown as ThreadInfo;
  return (
    <div className="grid gap-4">
      <p className="text-sm"><Link href={`/forum/${info.category?.slug ?? ""}`} className="text-dim hover:text-ink">← {info.category?.name ?? "Forum"}</Link></p>
      <ThreadView thread={info} posts={(posts ?? []) as unknown as Post[]} viewerId={me?.id ?? null} isMod={!!me && (me.is_admin || me.is_moderator)} />
    </div>
  );
}
