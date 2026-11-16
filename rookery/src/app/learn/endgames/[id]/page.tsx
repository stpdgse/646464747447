import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";
import { EndgameTrainer } from "@/components/learn/EndgameTrainer";
import { getCurrentProfile } from "@/lib/auth";
import { ENDGAMES, getEndgame } from "@/lib/lessons/lessons";
import { createClient } from "@/lib/supabase/server";

export async function generateMetadata({ params }: PageProps<"/learn/endgames/[id]">): Promise<Metadata> {
  const { id } = await params;
  return { title: getEndgame(id)?.title ?? "Endgame" };
}

export default function EndgamePage({ params }: PageProps<"/learn/endgames/[id]">) {
  return (
    <Suspense fallback={<div className="h-96 animate-pulse rounded-xl bg-surface" />}>
      <Loader params={params} />
    </Suspense>
  );
}

async function Loader({ params }: { params: PageProps<"/learn/endgames/[id]">["params"] }) {
  const { id } = await params;
  const endgame = getEndgame(id);
  if (!endgame) notFound();
  const me = await getCurrentProfile();
  let done = false;
  if (me) {
    const { data } = await (await createClient()).from("lesson_progress").select("item_id").eq("lesson_id", "endgames").eq("item_id", id).maybeSingle();
    done = !!data;
  }
  const i = ENDGAMES.findIndex((e) => e.id === id);
  const next = ENDGAMES[i + 1] ? { id: ENDGAMES[i + 1].id, title: ENDGAMES[i + 1].title } : null;
  return (
    <div className="grid gap-4">
      <p className="text-sm"><Link href="/learn/endgames" className="text-dim hover:text-ink">← Endgames</Link></p>
      <h1 className="text-2xl font-extrabold tracking-tight">{endgame.title}</h1>
      <EndgameTrainer key={endgame.id} endgame={endgame} viewerId={me?.id ?? null} done={done} next={next} />
    </div>
  );
}
