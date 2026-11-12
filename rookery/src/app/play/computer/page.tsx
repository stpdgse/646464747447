import type { Metadata } from "next";
import { ComputerGame } from "@/components/computer/ComputerGame";

export const metadata: Metadata = {
  title: "Play the computer",
  description: "Play Stockfish at any strength, or take on a bot with a personality.",
};

export default function ComputerPage() {
  return (
    <div className="grid gap-4">
      <h1 className="text-3xl font-extrabold tracking-tight">Play the computer</h1>
      <ComputerGame />
    </div>
  );
}
