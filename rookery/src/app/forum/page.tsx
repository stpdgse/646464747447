import type { Metadata } from "next";
import Link from "next/link";
import { Suspense } from "react";
import { createClient } from "@/lib/supabase/server";
import { fmtDate } from "@/lib/social";

export const metadata: Metadata = { title: "Forum" };

export default function ForumPage() {
  return (
    <div className="grid gap-6">
      <h1 className="text-3xl font-extrabold tracking-tight">Forum</h1>
      <Suspense fallback={<div className="h-48 animate-pulse rounded-xl bg-surface" />}>
        <Categories />
      </Suspense>
    </div>
  );
}

type Cat = { id: string; slug: string; name: string; description: string; mod_only: boolean };

async function Categories() {
  const supabase = await createClient();
  const { data: cats } = await supabase.from("forum_categories").select("id, slug, name, description, mod_only").order("sort");
  const { data: threads } = await supabase.from("forum_threads").select("category_id, post_count, last_post_at").is("deleted_at", null).limit(2000);
  const stats = new Map<string, { threads: number; last: string | null }>();
  for (const t of (threads ?? []) as { category_id: string; last_post_at: string }[]) {
    const s = stats.get(t.category_id) ?? { threads: 0, last: null };
    s.threads += 1;
    if (!s.last || t.last_post_at > s.last) s.last = t.last_post_at;
    stats.set(t.category_id, s);
  }
  return (
    <ul className="grid gap-3">
      {((cats ?? []) as Cat[]).map((c) => {
        const s = stats.get(c.id);
        return (
          <li key={c.id}>
            <Link href={`/forum/${c.slug}`} className="flex flex-wrap items-center gap-x-6 gap-y-1 rounded-xl border border-line bg-surface p-5 transition hover:bg-surface-2">
              <span className="min-w-0 flex-1">
                <span className="block text-lg font-bold">{c.name}</span>
                <span className="block text-sm text-dim">{c.description}</span>
              </span>
              <span className="text-right text-xs text-dim">
                {s?.threads ?? 0} threads{s?.last ? <><br />last post {fmtDate(s.last)}</> : null}
              </span>
            </Link>
          </li>
        );
      })}
    </ul>
  );
}
