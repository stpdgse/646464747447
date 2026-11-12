import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = { title: "Learn" };

const ITEMS = [
  { href: "/openings", title: "Opening explorer", body: "Walk through opening theory, see names for thousands of lines, and what your classmates play.", live: true },
  { href: "/play/board", title: "Analysis board with engine", body: "Switch on Stockfish for live evaluation and best-move arrows, or review a whole game for accuracy and mistakes.", live: true },
  { href: "/puzzles", title: "Puzzles", body: "Rated tactics, a daily puzzle and Puzzle Rush.", live: true },
  { href: "/learn", title: "Lessons and endgames", body: "Guided lessons and endgame practice against the computer.", live: false },
];

export default function LearnPage() {
  return (
    <div className="grid gap-6">
      <h1 className="text-3xl font-extrabold tracking-tight">Learn</h1>
      <div className="grid gap-4 sm:grid-cols-2">
        {ITEMS.map((m) =>
          m.live ? (
            <Link key={m.title} href={m.href} className="rounded-xl border border-brand bg-surface p-5 transition hover:bg-surface-2">
              <h2 className="text-lg font-bold">{m.title}</h2>
              <p className="mt-1 text-sm text-dim">{m.body}</p>
            </Link>
          ) : (
            <div key={m.title} className="rounded-xl border border-dashed border-line bg-surface p-5">
              <h2 className="text-lg font-bold">{m.title}</h2>
              <p className="mt-1 text-sm text-dim">{m.body}</p>
              <p className="mt-2 text-xs font-semibold text-dim">Coming later</p>
            </div>
          ),
        )}
      </div>
    </div>
  );
}
