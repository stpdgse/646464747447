"use client";

import Link from "next/link";
import { useState } from "react";
import { PERFS, formatRating, type Perf } from "@/lib/ratings";

export type LeaderRow = { user_id: string; perf: Perf; rating: number; rd: number; games: number; username: string };

export function LeaderboardTabs({ rows, me }: { rows: LeaderRow[]; me: string | null }) {
  const withData = PERFS.filter((p) => rows.some((r) => r.perf === p.id));
  const [perf, setPerf] = useState<Perf>(withData[0]?.id ?? "blitz");
  const list = rows.filter((r) => r.perf === perf);

  return (
    <div className="grid gap-4">
      <div role="tablist" aria-label="Leaderboard" className="flex flex-wrap gap-2">
        {PERFS.map((p) => (
          <button key={p.id} role="tab" aria-selected={perf === p.id} onClick={() => setPerf(p.id)}
            className={`rounded-full border px-3 py-1 text-sm font-medium ${perf === p.id ? "border-brand bg-brand/15" : "border-line bg-surface hover:bg-surface-2"}`}>
            {p.label}
          </button>
        ))}
      </div>
      {list.length === 0 ? (
        <p className="rounded-xl border border-line bg-surface px-4 py-8 text-center text-sm text-dim">
          Nobody is rated here yet. <Link href="/play/online" className="font-semibold text-brand underline">Play a rated game</Link> to take the top spot.
        </p>
      ) : (
        <div className="overflow-hidden rounded-xl border border-line bg-surface">
          <table className="w-full text-left text-sm">
            <thead className="bg-surface-2 text-dim">
              <tr><th className="w-14 px-4 py-2 font-medium">#</th><th className="px-4 py-2 font-medium">Player</th><th className="px-4 py-2 text-right font-medium">Rating</th><th className="px-4 py-2 text-right font-medium">Games</th></tr>
            </thead>
            <tbody>
              {list.map((r, i) => (
                <tr key={r.user_id} className={`border-t border-line ${r.user_id === me ? "bg-brand/10" : ""}`}>
                  <td className="px-4 py-2 text-dim">{i + 1}</td>
                  <td className="px-4 py-2 font-semibold"><Link href={`/u/${r.username}`} className="hover:text-brand underline">{r.username}</Link>{r.user_id === me && <span className="ml-2 text-xs font-normal text-dim">you</span>}</td>
                  <td className="px-4 py-2 text-right font-bold tabular-nums">{formatRating(r.rating, r.rd)}</td>
                  <td className="px-4 py-2 text-right tabular-nums text-dim">{r.games}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <p className="text-xs text-dim">A “?” means the rating is still provisional (few games). Ratings use Glicko-2 and update after every rated game.</p>
    </div>
  );
}
