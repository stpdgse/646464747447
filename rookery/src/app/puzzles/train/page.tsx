import type { Metadata } from "next";
import { PuzzleTrainer } from "@/components/puzzles/PuzzleTrainer";

export const metadata: Metadata = { title: "Train puzzles", description: "Rated tactics matched to your level." };

export default function TrainPage() {
  return (
    <div className="grid gap-4">
      <h1 className="text-3xl font-extrabold tracking-tight">Puzzles</h1>
      <PuzzleTrainer mode="train" />
    </div>
  );
}
