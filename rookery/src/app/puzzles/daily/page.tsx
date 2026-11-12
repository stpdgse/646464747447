import type { Metadata } from "next";
import { PuzzleTrainer } from "@/components/puzzles/PuzzleTrainer";

export const metadata: Metadata = { title: "Daily puzzle", description: "One puzzle a day, the same for everyone." };

export default function DailyPuzzlePage() {
  return (
    <div className="grid gap-4">
      <h1 className="text-3xl font-extrabold tracking-tight">Daily puzzle</h1>
      <PuzzleTrainer mode="daily" />
    </div>
  );
}
