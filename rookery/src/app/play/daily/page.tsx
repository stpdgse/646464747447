import type { Metadata } from "next";
import { Suspense } from "react";
import { DailyLobby } from "@/components/live/DailyLobby";
import { getCurrentProfile } from "@/lib/auth";

export const metadata: Metadata = { title: "Daily games", description: "Slow chess: a day or more per move. Play between classes." };

export default function DailyPage() {
  return (
    <div className="grid gap-4">
      <div>
        <h1 className="text-3xl font-extrabold tracking-tight">Daily games</h1>
        <p className="mt-1 max-w-xl text-dim">Slow chess: you get days, not minutes, for each move. Make a move between classes and come back tomorrow. You&apos;ll be notified when it&apos;s your turn.</p>
      </div>
      <Suspense fallback={<div className="h-64 animate-pulse rounded-xl bg-surface" />}>
        <Loader />
      </Suspense>
    </div>
  );
}

async function Loader() {
  const me = await getCurrentProfile();
  return <DailyLobby me={me ? { id: me.id, username: me.username } : null} />;
}
