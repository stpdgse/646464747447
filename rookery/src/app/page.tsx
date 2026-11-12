import Link from "next/link";
import { SITE } from "@/lib/site";

const MODES = [
  { href: "/play/online", title: "Play online", body: "Challenge a classmate or get paired. Bullet, Blitz and Rapid." },
  { href: "/play/computer", title: "Play the computer", body: "Stockfish at any strength, or take on a bot with a personality." },
  { href: "/puzzles", title: "Puzzles", body: "Rated tactics. Climb the ladder, or sprint in Puzzle Rush." },
  { href: "/play/daily", title: "Daily games", body: "Slow games over days. Make a move between classes." },
];

export default function Home() {
  return (
    <div className="grid gap-10">
      <section className="grid gap-4 pt-6">
        <h1 className="text-4xl font-extrabold tracking-tight sm:text-5xl">{SITE.tagline}</h1>
        <p className="max-w-xl text-lg text-dim">{SITE.description}</p>
        <div className="flex gap-3">
          <Link href="/play" className="rounded-md bg-brand px-5 py-2.5 font-semibold text-brand-ink hover:bg-brand-hi">
            Play now
          </Link>
          <Link href="/signup" className="rounded-md border border-line px-5 py-2.5 font-semibold hover:bg-surface-2">
            Create account
          </Link>
        </div>
      </section>
      <section aria-label="Ways to play" className="grid gap-4 sm:grid-cols-2">
        {MODES.map((m) => (
          <Link key={m.href} href={m.href} className="rounded-xl border border-line bg-surface p-5 transition hover:border-brand">
            <h2 className="text-lg font-bold">{m.title}</h2>
            <p className="mt-1 text-sm text-dim">{m.body}</p>
          </Link>
        ))}
      </section>
    </div>
  );
}
