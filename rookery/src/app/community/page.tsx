import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = { title: "Community" };

const ITEMS = [
  { href: "/leaderboard", title: "Leaderboard", body: "The strongest players in the class, for every time control and variant.", live: true },
  { href: "/watch", title: "Watch", body: "Spectate live games between classmates.", live: true },
  { href: "/community", title: "Friends", body: "Add friends and see when they are online.", live: false, when: "M10" },
  { href: "/community", title: "Clubs and forum", body: "Class clubs, team matches and discussion boards.", live: false, when: "M11" },
];

export default function CommunityPage() {
  return (
    <div className="grid gap-6">
      <h1 className="text-3xl font-extrabold tracking-tight">Community</h1>
      <div className="grid gap-4 sm:grid-cols-2">
        {ITEMS.map((m) =>
          m.live ? (
            <Link key={m.title} href={m.href} className="rounded-xl border border-brand bg-surface p-5 transition hover:bg-surface-2">
              <h2 className="text-lg font-bold">{m.title}</h2>
              <p className="mt-1 text-sm text-dim">{m.body}</p>
            </Link>
          ) : (
            <div key={m.title} className="rounded-xl border border-line bg-surface p-5 opacity-70">
              <h2 className="text-lg font-bold">{m.title}</h2>
              <p className="mt-1 text-sm text-dim">{m.body}</p>
              <p className="mt-2 text-xs font-semibold text-dim">Coming in {m.when}</p>
            </div>
          ),
        )}
      </div>
    </div>
  );
}
