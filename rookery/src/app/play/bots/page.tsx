import type { Metadata } from "next";
import { Suspense } from "react";
import { BotArena } from "@/components/computer/BotArena";

export const metadata: Metadata = { title: "Bot battles", description: "Pick two bots and watch them play each other." };

export default function BotsPage() {
  return (
    <div className="grid gap-4">
      <div>
        <h1 className="text-3xl font-extrabold tracking-tight">Bot battles</h1>
        <p className="mt-1 text-dim">Pick any two bots and let them fight. Everyone can watch on the Watch tab.</p>
      </div>
      <Suspense fallback={<div className="h-64 animate-pulse rounded-xl bg-surface" />}>
        <BotArena />
      </Suspense>
    </div>
  );
}
