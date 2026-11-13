import type { Metadata } from "next";
import { Suspense } from "react";
import { ComputerGame } from "@/components/computer/ComputerGame";

export const metadata: Metadata = {
  title: "Play the computer",
  description: "Play Stockfish at any strength, or take on a bot with a personality.",
};

export default function ComputerPage({ searchParams }: PageProps<"/play/computer">) {
  return (
    <div className="grid gap-4">
      <h1 className="text-3xl font-extrabold tracking-tight">Play the computer</h1>
      <Suspense fallback={<div className="h-64 animate-pulse rounded-xl bg-surface" />}>
        <Game searchParams={searchParams} />
      </Suspense>
    </div>
  );
}

async function Game({ searchParams }: { searchParams: PageProps<"/play/computer">["searchParams"] }) {
  const { bot } = await searchParams;
  // /play/computer?bot=<id> preselects an opponent (used by the admin "Try it" link)
  const id = typeof bot === "string" && /^[a-z0-9][a-z0-9-]{1,29}$/.test(bot) ? bot : undefined;
  return <ComputerGame initialBotId={id} />;
}
