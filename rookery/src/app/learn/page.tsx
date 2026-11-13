import type { Metadata } from "next";
import Link from "next/link";
import { Suspense } from "react";
import { getCurrentProfile } from "@/lib/auth";
import { ENDGAMES, LESSONS } from "@/lib/lessons/lessons";
import { createClient } from "@/lib/supabase/server";

export const metadata: Metadata = { title: "Learn", description: "Lessons, endgames, puzzles and the analysis board." };

const TOOLS = [
  { href: "/openings", title: "Opening explorer", body: "Walk through opening theory, see names for thousands of lines, and what your classmates play." },
  { href: "/play/board", title: "Analysis board", body: "Switch on Stockfish for live evaluation and best-move arrows, or review a whole game." },
  { href: "/puzzles", title: "Puzzles", body: "Rated tactics, a daily puzzle and Puzzle Rush." },
];

export default function LearnPage() {
  return (
    <div className="grid gap-10">
      <h1 className="text-3xl font-extrabold tracking-tight">Learn</h1>
      <Suspense fallback={<div className="h-64 animate-pulse rounded-xl bg-surface" />}>
        <Courses />
      </Suspense>
      <section aria-labelledby="tools" className="grid gap-3">
        <h2 id="tools" className="text-xl font-bold">Tools</h2>
        <div className="grid gap-4 sm:grid-cols-3">
          {TOOLS.map((m) => (
            <Link key={m.href} href={m.href} className="rounded-xl border border-line bg-surface p-5 transition hover:border-brand hover:bg-surface-2">
              <h3 className="text-lg font-bold">{m.title}</h3>
              <p className="mt-1 text-sm text-dim">{m.body}</p>
            </Link>
          ))}
        </div>
      </section>
    </div>
  );
}

async function Courses() {
  const me = await getCurrentProfile();
  const progress = new Map<string, Set<string>>();
  if (me) {
    const supabase = await createClient();
    const { data } = await supabase.from("lesson_progress").select("lesson_id, item_id");
    for (const r of (data ?? []) as { lesson_id: string; item_id: string }[]) {
      if (!progress.has(r.lesson_id)) progress.set(r.lesson_id, new Set());
      progress.get(r.lesson_id)!.add(r.item_id);
    }
  }
  const doneOf = (id: string) => progress.get(id)?.size ?? 0;
  const endgamesDone = doneOf("endgames");
  return (
    <>
      <section aria-labelledby="lessons" className="grid gap-3">
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <h2 id="lessons" className="text-xl font-bold">Lessons</h2>
          <p className="text-sm text-dim">{me ? "Your progress is saved." : <><Link href="/login" className="font-semibold text-brand underline">Log in</Link> to save your progress.</>}</p>
        </div>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {LESSONS.map((l) => {
            const total = l.kind === "steps" ? l.steps.length : l.count;
            const done = Math.min(total, doneOf(l.id));
            return (
              <Link key={l.id} href={`/learn/lessons/${l.id}`} className="grid content-start gap-1 rounded-xl border border-line bg-surface p-5 transition hover:border-brand hover:bg-surface-2">
                <span aria-hidden className="text-3xl">{l.icon}</span>
                <h3 className="text-lg font-bold">{l.title}</h3>
                <p className="text-xs font-semibold text-brand">{l.level}</p>
                <p className="text-sm text-dim">{l.blurb}</p>
                <p className="mt-1 text-xs font-semibold text-dim">{done >= total ? "✓ Complete" : done > 0 ? `${done} of ${total} done` : `${total} exercises`}</p>
              </Link>
            );
          })}
        </div>
      </section>
      <section aria-labelledby="endgames" className="grid gap-3">
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <h2 id="endgames" className="text-xl font-bold">Endgames</h2>
          <p className="text-sm text-dim">{endgamesDone} of {ENDGAMES.length} mastered</p>
        </div>
        <p className="text-sm text-dim">Play classic endings against Stockfish until you can win (or hold) them every time.</p>
        <Link href="/learn/endgames" className="justify-self-start rounded-md bg-brand px-4 py-2 text-sm font-semibold text-brand-ink hover:bg-brand-hi">Practise endgames</Link>
      </section>
    </>
  );
}
