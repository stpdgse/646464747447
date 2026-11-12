import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";
import { NewThread } from "@/components/social/ForumActions";
import { getCurrentProfile } from "@/lib/auth";
import { fmtDate, ui } from "@/lib/social";
import { createClient } from "@/lib/supabase/server";

export async function generateMetadata({ params }: PageProps<"/forum/[slug]">): Promise<Metadata> {
  const { slug } = await params;
  return { title: `Forum: ${slug}` };
}

export default function CategoryPage({ params }: PageProps<"/forum/[slug]">) {
  return (
    <Suspense fallback={<div className="h-64 animate-pulse rounded-xl bg-surface" />}>
      <Loader params={params} />
    </Suspense>
  );
}

type Thread = {
  id: string; title: string; pinned: boolean; locked: boolean; deleted_at: string | null; post_count: number; last_post_at: string;
  author: { username: string } | null;
};

async function Loader({ params }: { params: PageProps<"/forum/[slug]">["params"] }) {
  const { slug } = await params;
  const supabase = await createClient();
  const { data: cat } = await supabase.from("forum_categories").select("id, slug, name, description, mod_only").eq("slug", slug).maybeSingle();
  if (!cat) notFound();
  const me = await getCurrentProfile();
  const isMod = !!me && (me.is_admin || me.is_moderator);
  const { data } = await supabase
    .from("forum_threads")
    .select("id, title, pinned, locked, deleted_at, post_count, last_post_at, author:profiles(username)")
    .eq("category_id", cat.id)
    .order("pinned", { ascending: false })
    .order("last_post_at", { ascending: false })
    .limit(100);
  const threads = (data ?? []) as unknown as Thread[];
  const canStart = !!me && (!cat.mod_only || isMod);
  return (
    <div className="grid gap-6">
      <header className="grid gap-1">
        <p className="text-sm"><Link href="/forum" className="text-dim hover:text-ink">← Forum</Link></p>
        <h1 className="text-3xl font-extrabold tracking-tight">{cat.name}</h1>
        <p className="text-dim">{cat.description}</p>
      </header>
      {threads.length === 0 ? (
        <p className="text-dim">No threads yet.</p>
      ) : (
        <ul className="grid gap-2">
          {threads.map((t) => (
            <li key={t.id}>
              <Link href={`/forum/thread/${t.id}`} className={`flex flex-wrap items-center gap-x-4 gap-y-1 rounded-xl border border-line bg-surface px-5 py-3 transition hover:bg-surface-2 ${t.deleted_at ? "opacity-50" : ""}`}>
                <span className="min-w-0 flex-1">
                  <span className="font-semibold">{t.pinned && <span title="Pinned">📌 </span>}{t.locked && <span title="Locked">🔒 </span>}{t.title}</span>
                  {t.deleted_at && <span className="ml-2 text-xs text-danger">(removed)</span>}
                  <span className="block text-xs text-dim">by {t.author?.username ?? "?"}</span>
                </span>
                <span className="text-right text-xs text-dim">{Math.max(0, t.post_count - 1)} replies<br />{fmtDate(t.last_post_at)}</span>
              </Link>
            </li>
          ))}
        </ul>
      )}
      {canStart ? <NewThread categoryId={cat.id} slug={cat.slug} /> : <p className={ui.info}>{me ? "Only moderators can start threads here." : <><Link className="text-brand underline" href="/login">Log in</Link> to start a thread.</>}</p>}
    </div>
  );
}
