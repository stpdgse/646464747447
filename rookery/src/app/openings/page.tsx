import type { Metadata } from "next";
import { OpeningExplorer } from "@/components/openings/OpeningExplorer";

export const metadata: Metadata = { title: "Opening explorer", description: "Explore opening theory and see what your classmates play." };

export default function OpeningsPage() {
  return (
    <div className="grid gap-4">
      <div>
        <h1 className="text-3xl font-extrabold tracking-tight">Opening explorer</h1>
        <p className="mt-1 max-w-2xl text-dim">Play moves on the board to walk through theory. The book lists named openings from the Lichess collection, and “Your class” shows what people on this site actually played next and how those games ended.</p>
      </div>
      <OpeningExplorer />
    </div>
  );
}
