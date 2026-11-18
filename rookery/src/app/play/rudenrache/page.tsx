import type { Metadata } from "next";
import Link from "next/link";
import { Suspense } from "react";
import { RudeChallenge, type Attempt } from "@/components/rude/RudeChallenge";
import { getCurrentProfile } from "@/lib/auth";
import { EVENT_KEY, LEVELS, MAX_ATTEMPTS, leaderboard } from "@/lib/rude";
import { fmtDate, ui } from "@/lib/social";
import { createClient } from "@/lib/supabase/server";

export const metadata: Metadata = { title: "Rüdenrache Holiday Challenge", description: "A limited-time holiday event: beat 20 Rüde bots in a row, rated 100 to 2000." };

export default function RudePage() {
  return (
    <div className="grid gap-6">
      <Suspense fallback={<div className="h-64 animate-pulse rounded-xl bg-surface" />}>
        <Content />
      </Suspense>
    </div>
  );
}

type AttemptRow = { id: string; user_id: string; attempt_no: number; level: number; best_level: number; cleared: boolean; status: "playing" | "finished"; updated_at: string; user: { username: string } | null };

async function Content() {
  const supabase = await createClient();
  const me = await getCurrentProfile();
  const [{ data: ev }, { data: rows }, { data: st }] = await Promise.all([
    supabase.from("events").select("key, name, starts_at, ends_at").eq("key", EVENT_KEY).maybeSingle(),
    supabase.from("rude_attempts").select("id, user_id, attempt_no, level, best_level, cleared, status, updated_at, user:profiles(username)").eq("event_key", EVENT_KEY).order("updated_at", { ascending: false }).limit(2000),
    supabase.rpc("server_time"),
  ]);
  if (!ev) return <p className="text-dim">This event isn&apos;t set up.</p>;
  const now = typeof st === "string" ? new Date(st).getTime() : 0;
  const phase = now < new Date(ev.starts_at).getTime() ? "upcoming" : now >= new Date(ev.ends_at).getTime() ? "ended" : "live";
  const all = (rows ?? []) as unknown as AttemptRow[];
  const mine: Attempt[] = all.filter((r) => r.user_id === me?.id).sort((a, b) => a.attempt_no - b.attempt_no).map((r) => ({ id: r.id, attempt_no: r.attempt_no, level: r.level, best_level: r.best_level, cleared: r.cleared, status: r.status }));
  const board = leaderboard(all.map((r) => ({ user_id: r.user_id, username: r.user?.username ?? "?", best_level: r.best_level, cleared: r.cleared, updated_at: r.updated_at })));
  const daysLeft = Math.max(0, Math.ceil((new Date(ev.ends_at).getTime() - now) / 86_400_000));
  // Attempts that were touched in the last 30 minutes are being played right now.
  const live = all.filter((r) => r.status === "playing" && now - new Date(r.updated_at).getTime() < 30 * 60_000);

  return (
    <>
      <header className="grid gap-3 overflow-hidden rounded-2xl border border-warn bg-gradient-to-br from-warn/20 via-surface to-brand/15 p-6">
        <div className="flex flex-wrap items-center gap-2">
          <span className="rounded-full bg-warn px-3 py-1 text-xs font-extrabold uppercase tracking-wide text-[#151515]">🎄 Limited time · Holiday event</span>
          {phase === "live" && <span className="text-sm font-semibold">{daysLeft <= 1 ? "Ends today" : `${daysLeft} days left`} · until {fmtDate(ev.ends_at)}</span>}
          {phase === "upcoming" && <span className="text-sm font-semibold">Starts {fmtDate(ev.starts_at)}</span>}
          {phase === "ended" && <span className="text-sm font-semibold">Ended {fmtDate(ev.ends_at)}</span>}
        </div>
        <h1 className="text-3xl font-extrabold tracking-tight sm:text-4xl">🐺 Rüdenrache: the Holiday Challenge</h1>
        <p className="max-w-3xl text-base">
          Twenty Rüden stand between you and glory, rated <strong>100, 200, 300 … up to 2000</strong>. Beat the first one, then the next, then the next.
          A draw or a loss ends your attempt, and the highest Rüde you beat is your score. You get <strong>{MAX_ATTEMPTS} attempts</strong> in total. How far can you get?
        </p>
        <ul className="grid gap-1 text-sm text-dim sm:grid-cols-2">
          <li>♟️ You play White against the odd Rüden and Black against the even ones.</li>
          <li>👀 Everyone can watch your games live on the <Link href="/watch" className="font-semibold text-brand underline">Watch tab</Link>.</li>
          <li>🏅 Special limited-edition badges for every milestone.</li>
          <li>🏆 The leaderboard shows who got furthest.</li>
        </ul>
      </header>

      <RudeChallenge phase={phase} viewerId={me?.id ?? null} initialAttempts={mine} />

      {live.length > 0 && (
        <section aria-labelledby="live" className="grid gap-2">
          <h2 id="live" className="text-xl font-bold">Fighting the Rüden right now</h2>
          <ul className="grid gap-2 sm:grid-cols-2">
            {live.map((r) => (
              <li key={r.id}><Link href="/watch" className="flex items-center justify-between gap-2 rounded-xl border border-brand bg-surface px-4 py-3 transition hover:bg-surface-2"><span className="font-semibold">{r.user?.username ?? "?"}</span><span className="text-sm text-dim">attempt {r.attempt_no} · facing Rüde {r.level} · watch</span></Link></li>
            ))}
          </ul>
        </section>
      )}

      <section aria-labelledby="board" className="grid gap-3">
        <h2 id="board" className="text-xl font-bold">Leaderboard</h2>
        <p className="text-sm text-dim">The highest Rüde each player has beaten, out of {LEVELS.length}. Ties go to fewer attempts, then to whoever got there first.</p>
        {board.length === 0 ? <p className="text-sm text-dim">Nobody has tried yet. Be the first!</p> : (
          <div className="overflow-x-auto rounded-xl border border-line">
            <table className="w-full min-w-[26rem] text-sm">
              <thead className="bg-surface-2 text-left text-xs uppercase text-dim"><tr><th className="px-3 py-2">#</th><th className="px-3 py-2">Player</th><th className="px-3 py-2 text-right">Beat up to</th><th className="px-3 py-2 text-right">Attempts</th></tr></thead>
              <tbody>
                {board.slice(0, 50).map((e, i) => (
                  <tr key={e.user_id} className={`border-t border-line ${e.user_id === me?.id ? "bg-brand/10" : ""}`}>
                    <td className="px-3 py-2 tabular-nums">{i + 1}</td>
                    <td className="px-3 py-2 font-semibold"><Link href={`/u/${e.username}`} className="hover:underline">{e.username}</Link>{e.cleared && <span className="ml-2" title="Cleared all 20">🏔️</span>}</td>
                    <td className="px-3 py-2 text-right font-bold tabular-nums">{e.best > 0 ? `Rüde ${e.best}` : "none yet"}</td>
                    <td className="px-3 py-2 text-right tabular-nums">{e.attempts} / {MAX_ATTEMPTS}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {!me && <p className={ui.info}><Link className="text-brand underline" href="/login">Log in</Link> to join the leaderboard.</p>}
      </section>
    </>
  );
}
