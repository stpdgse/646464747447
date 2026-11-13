import type { Metadata } from "next";
import Link from "next/link";
import { Suspense } from "react";
import { getCurrentProfile } from "@/lib/auth";
import { ENDGAMES } from "@/lib/lessons/lessons";
import { createClient } from "@/lib/supabase/server";

export const metadata: Metadata = { title: "Endgames" };

export default function EndgamesPage() {
  return (
    <div className="grid gap-6">
      <div>
        <p className="text-sm"><Link href="/learn" className="text-dim hover:text-ink">← Learn</Link></p>
        <h1 className="text-3xl font-extrabold tracking-tight">Endgames</h1>
        <p className="mt-1 text-dim">Each position is played against Stockfish. Win it, or hold the draw, to master it.</p>
      </div>
      <Suspense fallback={<div className="h-64 animate-pulse rounded-xl bg-surface" />}>
        <List />
      </Suspense>
    </div>
  );
}

async function List() {
  const me = await getCurrentProfile();
  let done = new Set<string>();
  if (me) {
    const { data } = await (await createClient()).from("lesson_progress").select("item_id").eq("lesson_id", "endgames");
    done = new Set(((data ?? []) as { item_id: string }[]).map((r) => r.item_id));
  }
  return (
    <ul className="grid gap-3 sm:grid-cols-2">
      {ENDGAMES.map((e) => (
        <li key={e.id}>
          <Link href={`/learn/endgames/${e.id}`} className="grid h-full content-start gap-1 rounded-xl border border-line bg-surface p-5 transition hover:border-brand hover:bg-surface-2">
            <h2 className="text-lg font-bold">{e.title}</h2>
            <p className="text-xs font-semibold text-brand">{e.level} · you {e.goal === "win" ? "win" : "hold the draw"} as {e.player}</p>
            <p className="text-sm text-dim">{e.blurb}</p>
            {done.has(e.id) && <p className="text-xs font-semibold text-brand">✓ Mastered</p>}
          </Link>
        </li>
      ))}
    </ul>
  );
}
