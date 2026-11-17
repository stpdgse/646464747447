"use client";

import { useRouter } from "next/navigation";
import { useMemo, useState } from "react";
import { useBots } from "@/components/computer/useBots";
import { KNOCKOUT_SIZES, MAX_ROUND_ROBIN, MIN_BOTS, validateSpec, type Format } from "@/lib/botTournament";
import { SPEEDS } from "@/lib/engine/botMove";
import { ui } from "@/lib/social";
import { createClient } from "@/lib/supabase/client";

const NAMES = ["Bot Cup", "Metal Madness", "Robot Rumble", "Silicon Showdown", "Pixel Open", "Engine Invitational", "Gambit Games", "Circuit Classic"];
const ERRORS: Record<string, string> = {
  too_many_tournaments: "You already have a tournament running. Wait for it to finish (or delete it) before starting another.",
  site_busy: "The site is running the most tournaments it allows right now. Try again in a little while.",
  slow_down: "You've started 5 tournaments today. That's the daily limit.",
  bad_size: "Wrong number of bots for that format.",
  bad_bots: "Something is wrong with the bot list.",
  "row-level security": "You can't create tournaments right now (are you muted or suspended?).",
};

/** Anyone logged in can start one. The database enforces the limits; this form explains them. */
export function BotTournamentCreate({ userId }: { userId: string }) {
  const router = useRouter();
  const bots = useBots();
  const [f, setF] = useState(() => ({ name: NAMES[Math.floor(Math.random() * NAMES.length)] + " " + Math.floor(10 + Math.random() * 90), format: "knockout" as Format, bestOf: 1 as 1 | 3, size: 8, variant: "standard" as "standard" | "chess960", speed: "fast" as "fast" | "normal" }));
  const [picked, setPicked] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const target = f.format === "knockout" ? f.size : null;
  const max = f.format === "knockout" ? f.size : MAX_ROUND_ROBIN;
  const chosen = useMemo(() => picked.map((id) => bots.find((b) => b.id === id)).filter((b): b is NonNullable<typeof b> => !!b), [picked, bots]);
  const problem = validateSpec({ format: f.format, bestOf: f.bestOf, bots: chosen.map((b) => ({ id: b.id, name: b.name, rating: b.rating })) });

  const toggle = (id: string) => setPicked((cur) => (cur.includes(id) ? cur.filter((x) => x !== id) : cur.length >= max ? cur : [...cur, id]));
  const random = () => {
    const pool = [...bots].sort(() => Math.random() - 0.5);
    setPicked(pool.slice(0, target ?? 6).map((b) => b.id));
  };
  const strongest = () => setPicked([...bots].sort((a, b) => b.rating - a.rating).slice(0, target ?? 6).map((b) => b.id));
  const spread = () => {
    const sorted = [...bots].sort((a, b) => a.rating - b.rating), n = target ?? 6;
    setPicked(Array.from({ length: n }, (_, i) => sorted[Math.round((i * (sorted.length - 1)) / (n - 1))].id));
  };

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (problem) return setError(problem);
    setBusy(true);
    setError(null);
    const { data, error: err } = await createClient().from("bot_tournaments").insert({
      creator_id: userId, name: f.name.trim(), format: f.format, best_of: f.format === "knockout" ? f.bestOf : 1, variant: f.variant, speed: f.speed,
      bots: chosen.map((b) => ({ id: b.id, name: b.name, rating: b.rating })),
    }).select("id").single();
    setBusy(false);
    if (err || !data) {
      const key = Object.keys(ERRORS).find((k) => err?.message.includes(k));
      return setError(key ? ERRORS[key] : err?.message.includes("check constraint") ? "The name needs 3 to 50 characters." : "Could not create it. Please try again.");
    }
    router.push(`/bot-tournaments/${data.id}`);
  };

  return (
    <form onSubmit={submit} className={`${ui.card} grid gap-4`}>
      <div>
        <h2 className="text-xl font-bold">Start a bot tournament</h2>
        <p className="text-xs text-dim">Limits so nobody can flood the site: one running tournament per person, 5 a day, and 8 on the site at once. Everyone can watch.</p>
      </div>
      {error && <p role="alert" className={ui.err}>{error}</p>}
      <label className="grid gap-1.5 text-sm font-medium">Name
        <input className={ui.field} value={f.name} onChange={(e) => setF({ ...f, name: e.target.value })} minLength={3} maxLength={50} required />
      </label>
      <div className="grid gap-3 sm:grid-cols-4">
        <label className="grid gap-1.5 text-sm font-medium">Format
          <select className={ui.field} value={f.format} onChange={(e) => { setF({ ...f, format: e.target.value as Format }); setPicked([]); }}>
            <option value="knockout">Knockout (bracket)</option><option value="roundrobin">Round-robin (league)</option>
          </select>
        </label>
        {f.format === "knockout" ? (
          <>
            <label className="grid gap-1.5 text-sm font-medium">Bots
              <select className={ui.field} value={f.size} onChange={(e) => { setF({ ...f, size: Number(e.target.value) }); setPicked([]); }}>
                {KNOCKOUT_SIZES.map((n) => <option key={n} value={n}>{n} bots</option>)}
              </select>
            </label>
            <label className="grid gap-1.5 text-sm font-medium">Each match
              <select className={ui.field} value={f.bestOf} onChange={(e) => setF({ ...f, bestOf: Number(e.target.value) as 1 | 3 })}>
                <option value={1}>One game</option><option value={3}>Best of three</option>
              </select>
            </label>
          </>
        ) : <p className="self-end pb-2 text-xs text-dim sm:col-span-2">Pick {MIN_BOTS} to {MAX_ROUND_ROBIN} bots. Every pair plays twice (once with each colour).</p>}
        <label className="grid gap-1.5 text-sm font-medium">Speed
          <select className={ui.field} value={f.speed} onChange={(e) => setF({ ...f, speed: e.target.value as "fast" | "normal" })}>
            {(["fast", "normal"] as const).map((s) => <option key={s} value={s}>{SPEEDS[s].label}</option>)}
          </select>
        </label>
      </div>

      <fieldset className="grid gap-2">
        <legend className="mb-1 text-sm font-medium">Choose the bots ({chosen.length}{target ? ` of ${target}` : ""} picked)</legend>
        <div className="flex flex-wrap gap-2">
          <button type="button" className={ui.btn} onClick={random}>Random bots</button>
          <button type="button" className={ui.btn} onClick={spread}>A mix of strengths</button>
          <button type="button" className={ui.btn} onClick={strongest}>Strongest bots</button>
          <button type="button" className={ui.btn} onClick={() => setPicked([])}>Clear</button>
        </div>
        <div className="grid max-h-80 gap-2 overflow-y-auto rounded-lg border border-line bg-bg p-2 sm:grid-cols-2 lg:grid-cols-3">
          {bots.map((b) => {
            const on = picked.includes(b.id);
            return (
              <label key={b.id} className={`flex cursor-pointer items-center gap-2 rounded-md border px-2 py-1.5 text-sm ${on ? "border-brand bg-brand/15" : "border-line bg-surface hover:bg-surface-2"} ${!on && picked.length >= max ? "opacity-50" : ""}`}>
                <input type="checkbox" className="size-4 accent-[var(--brand)]" checked={on} disabled={!on && picked.length >= max} onChange={() => toggle(b.id)} />
                {b.avatarImage ? <img src={b.avatarImage} alt="" width={20} height={20} className="size-5 rounded" /> : <span aria-hidden>{b.avatar}</span>}
                <span className="min-w-0 flex-1 truncate">{b.name}</span>
                <span className="text-xs text-dim">≈{b.rating}</span>
              </label>
            );
          })}
        </div>
      </fieldset>

      <div className="flex flex-wrap items-center gap-3">
        <button disabled={busy || !!problem} className={ui.primary}>Create tournament</button>
        {problem && picked.length > 0 && <span className="text-xs text-dim">{problem}</span>}
      </div>
      <p className="text-xs text-dim">The games are played in your browser while you watch, so keep the tournament page open. If you leave, any viewer can take over and carry on.</p>
    </form>
  );
}
