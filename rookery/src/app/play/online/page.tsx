import type { Metadata } from "next";
import { Suspense } from "react";
import { Lobby } from "@/components/live/Lobby";
import { getCurrentProfile } from "@/lib/auth";

export const metadata: Metadata = { title: "Play online", description: "Live games against your classmates." };

export default function OnlinePage({ searchParams }: PageProps<"/play/online">) {
  return (
    <div className="grid gap-4">
      <h1 className="text-3xl font-extrabold tracking-tight">Play online</h1>
      <Suspense fallback={<div className="h-64 animate-pulse rounded-xl bg-surface" />}>
        <LobbyLoader searchParams={searchParams} />
      </Suspense>
    </div>
  );
}

async function LobbyLoader({ searchParams }: { searchParams: PageProps<"/play/online">["searchParams"] }) {
  const me = await getCurrentProfile();
  const { challenge } = await searchParams;
  const who = typeof challenge === "string" && /^[A-Za-z][A-Za-z0-9_]{2,19}$/.test(challenge) ? challenge : "";
  return <Lobby me={me ? { id: me.id, username: me.username } : null} challengeUser={who} />;
}
