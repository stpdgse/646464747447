import type { Metadata } from "next";
import Link from "next/link";
import { Suspense } from "react";
import { CreateClub } from "@/components/social/CreateClub";
import { getCurrentProfile } from "@/lib/auth";
import { createClient } from "@/lib/supabase/server";
import { ui } from "@/lib/social";

export const metadata: Metadata = { title: "Clubs" };

export default function ClubsPage() {
  return (
    <div className="grid gap-6">
      <h1 className="text-3xl font-extrabold tracking-tight">Clubs</h1>
      <Suspense fallback={<div className="h-48 animate-pulse rounded-xl bg-surface" />}>
        <ClubList />
      </Suspense>
    </div>
  );
}

type ClubRow = { id: string; slug: string; name: string; description: string; open: boolean; members: { count: number }[] };

async function ClubList() {
  const supabase = await createClient();
  const me = await getCurrentProfile();
  const { data } = await supabase
    .from("clubs")
    .select("id, slug, name, description, open, members:club_members(count)")
    .eq("archived", false)
    .eq("members.status", "active")
    .order("created_at", { ascending: false })
    .limit(100);
  const clubs = ((data ?? []) as unknown as ClubRow[]).sort((a, b) => (b.members[0]?.count ?? 0) - (a.members[0]?.count ?? 0));
  return (
    <>
      {clubs.length === 0 ? (
        <p className="text-dim">No clubs yet. Start the first one.</p>
      ) : (
        <ul className="grid gap-4 sm:grid-cols-2">
          {clubs.map((c) => (
            <li key={c.id}>
              <Link href={`/clubs/${c.slug}`} className="block h-full rounded-xl border border-line bg-surface p-5 transition hover:bg-surface-2">
                <h2 className="text-lg font-bold">{c.name}</h2>
                <p className="mt-1 line-clamp-2 text-sm text-dim">{c.description || "No description."}</p>
                <p className="mt-2 text-xs font-semibold text-dim">{c.members[0]?.count ?? 0} members · {c.open ? "open" : "by request"}</p>
              </Link>
            </li>
          ))}
        </ul>
      )}
      {me ? <CreateClub viewerId={me.id} /> : <p className={ui.info}><Link className="text-brand underline" href="/login">Log in</Link> to join or start a club.</p>}
    </>
  );
}
