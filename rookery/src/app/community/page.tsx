import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = { title: "Community" };

const ITEMS = [
  { href: "/community/friends", title: "Friends", body: "Add friends, see who is online and watch their games." },
  { href: "/tournaments", title: "Tournaments", body: "Arena events: play as many games as you can and climb the standings." },
  { href: "/clubs", title: "Clubs", body: "Join a class club or start your own, with its own chat." },
  { href: "/forum", title: "Forum", body: "Talk chess, ask for help and share games." },
  { href: "/leaderboard", title: "Leaderboard", body: "The strongest players in the class, for every time control and variant." },
  { href: "/watch", title: "Watch", body: "Spectate live games between classmates." },
];

export default function CommunityPage() {
  return (
    <div className="grid gap-6">
      <h1 className="text-3xl font-extrabold tracking-tight">Community</h1>
      <div className="grid gap-4 sm:grid-cols-2">
        {ITEMS.map((m) => (
          <Link key={m.title} href={m.href} className="rounded-xl border border-brand bg-surface p-5 transition hover:bg-surface-2">
            <h2 className="text-lg font-bold">{m.title}</h2>
            <p className="mt-1 text-sm text-dim">{m.body}</p>
          </Link>
        ))}
      </div>
    </div>
  );
}
