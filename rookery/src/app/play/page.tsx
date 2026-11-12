import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = { title: "Play" };

const MODES = [
  { href: "/play/online", title: "Play online", body: "Live games against classmates with real clocks. Bullet, Blitz and Rapid, plus challenges." },
  { href: "/play/daily", title: "Daily games", body: "Slow chess: days per move. Play between classes and get notified on your turn." },
  { href: "/play/computer", title: "Play the computer", body: "Stockfish at any strength, plus eight bots with personalities." },
  { href: "/play/board", title: "Analysis board", body: "Play both sides, set up any position, load a PGN. Chess960, King of the Hill and Three-check included." },
];

export default function PlayPage() {
  return (
    <div className="grid gap-6">
      <h1 className="text-3xl font-extrabold tracking-tight">Play</h1>
      <div className="grid gap-4 sm:grid-cols-2">
        {MODES.map((m) => (
          <Link key={m.href} href={m.href} className="rounded-xl border border-brand bg-surface p-5 transition hover:bg-surface-2">
            <h2 className="text-lg font-bold">{m.title}</h2>
            <p className="mt-1 text-sm text-dim">{m.body}</p>
          </Link>
        ))}
      </div>
    </div>
  );
}
