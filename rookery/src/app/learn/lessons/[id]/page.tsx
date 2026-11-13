import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";
import { LessonRunner, type LessonItem } from "@/components/learn/LessonRunner";
import { getCurrentProfile } from "@/lib/auth";
import { LESSONS, getLesson } from "@/lib/lessons/lessons";
import { lessonPuzzles } from "@/lib/puzzles/data";
import { createClient } from "@/lib/supabase/server";

export async function generateMetadata({ params }: PageProps<"/learn/lessons/[id]">): Promise<Metadata> {
  const { id } = await params;
  return { title: getLesson(id)?.title ?? "Lesson" };
}

export default function LessonPage({ params }: PageProps<"/learn/lessons/[id]">) {
  return (
    <Suspense fallback={<div className="h-96 animate-pulse rounded-xl bg-surface" />}>
      <Loader params={params} />
    </Suspense>
  );
}

async function Loader({ params }: { params: PageProps<"/learn/lessons/[id]">["params"] }) {
  const { id } = await params;
  const lesson = getLesson(id);
  if (!lesson) notFound();
  const me = await getCurrentProfile();
  let done: string[] = [];
  if (me) {
    const { data } = await (await createClient()).from("lesson_progress").select("item_id").eq("lesson_id", id);
    done = ((data ?? []) as { item_id: string }[]).map((r) => r.item_id);
  }
  const items: LessonItem[] =
    lesson.kind === "steps"
      ? lesson.steps.map((s) => ({ id: s.id, step: s }))
      : lessonPuzzles({ theme: lesson.theme, alsoTag: lesson.alsoTag, min: lesson.minRating, max: lesson.maxRating, count: lesson.count }).map((p) => ({ id: p.id, puzzle: p }));
  const i = LESSONS.findIndex((l) => l.id === id);
  const nxt = LESSONS[i + 1];
  return (
    <div className="grid gap-4">
      <p className="text-sm"><Link href="/learn" className="text-dim hover:text-ink">← Learn</Link></p>
      <LessonRunner
        key={lesson.id}
        lesson={{ id: lesson.id, title: lesson.title, icon: lesson.icon, intro: lesson.intro, ideas: lesson.ideas, solved: lesson.kind === "puzzles" ? lesson.solved : "" }}
        items={items}
        done={done}
        viewerId={me?.id ?? null}
        next={nxt ? { id: nxt.id, title: nxt.title } : null}
      />
    </div>
  );
}
