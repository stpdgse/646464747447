import type { Metadata } from "next";
import { Suspense } from "react";
import { PuzzleRush } from "@/components/puzzles/PuzzleRush";
import { getCurrentProfile } from "@/lib/auth";
import { createClient } from "@/lib/supabase/server";

export const metadata: Metadata = { title: "Puzzle Rush", description: "Solve as many puzzles as you can in three minutes." };

export default function RushPage() {
  return (
    <div className="grid gap-4">
      <h1 className="text-3xl font-extrabold tracking-tight">Puzzle Rush</h1>
      <Suspense fallback={<div className="aspect-square max-w-xl animate-pulse rounded-xl bg-surface" />}>
        <Loader />
      </Suspense>
    </div>
  );
}

async function Loader() {
  const me = await getCurrentProfile();
  let best: number | null = null;
  if (me) {
    const supabase = await createClient();
    const { data } = await supabase.from("rush_scores").select("score").eq("user_id", me.id).order("score", { ascending: false }).limit(1);
    best = data?.[0]?.score ?? null;
  }
  return <PuzzleRush signedIn={!!me} best={best} />;
}
