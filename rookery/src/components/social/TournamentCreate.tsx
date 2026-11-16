"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { LIVE_TIME_CONTROLS, getLiveTimeControl } from "@/lib/live/timecontrols";
import { friendlyError, ui } from "@/lib/social";
import { createClient } from "@/lib/supabase/client";

const DURATIONS = [15, 20, 30, 45, 60, 90, 120];

/** Moderators and admins create tournaments here. Players just join them. */
export function TournamentCreate({ userId }: { userId: string }) {
  const router = useRouter();
  const [f, setF] = useState({ name: "", description: "", timeId: "3+2", variant: "standard" as "standard" | "chess960", rated: false, startNow: true, start: "", minutes: 30 });
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    const tc = getLiveTimeControl(f.timeId);
    if (!tc) return setError("Pick a time control.");
    const startsAt = f.startNow ? new Date() : new Date(f.start);
    if (Number.isNaN(startsAt.getTime())) return setError("Pick a start time.");
    const endsAt = new Date(startsAt.getTime() + f.minutes * 60_000);
    setBusy(true);
    setError(null);
    const { error: err } = await createClient().from("tournaments").insert({
      name: f.name.trim(), description: f.description.trim(), created_by: userId, variant: f.variant, time_initial_ms: tc.ms, time_inc_ms: tc.inc, rated: f.rated,
      starts_at: startsAt.toISOString(), ends_at: endsAt.toISOString(),
    });
    setBusy(false);
    if (err) return setError(err.message.includes("check constraint") ? "The name needs 3 to 60 letters." : friendlyError(err));
    setF({ ...f, name: "", description: "" });
    router.refresh();
  };

  return (
    <form onSubmit={submit} className={`${ui.card} grid max-w-2xl gap-3`}>
      <h2 className="text-xl font-bold">Create a tournament</h2>
      {error && <p role="alert" className={ui.err}>{error}</p>}
      <label className="grid gap-1.5 text-sm font-medium">Name
        <input className={ui.field} value={f.name} onChange={(e) => setF({ ...f, name: e.target.value })} minLength={3} maxLength={60} required placeholder="Friday lunch arena" />
      </label>
      <label className="grid gap-1.5 text-sm font-medium">Description (optional)
        <input className={ui.field} value={f.description} onChange={(e) => setF({ ...f, description: e.target.value })} maxLength={400} />
      </label>
      <div className="grid gap-3 sm:grid-cols-3">
        <label className="grid gap-1.5 text-sm font-medium">Time control
          <select className={ui.field} value={f.timeId} onChange={(e) => setF({ ...f, timeId: e.target.value })}>
            {LIVE_TIME_CONTROLS.map((t) => <option key={t.id} value={t.id}>{t.label} ({t.group})</option>)}
          </select>
        </label>
        <label className="grid gap-1.5 text-sm font-medium">Variant
          <select className={ui.field} value={f.variant} onChange={(e) => setF({ ...f, variant: e.target.value as typeof f.variant })}>
            <option value="standard">Standard</option><option value="chess960">Chess960</option>
          </select>
        </label>
        <label className="grid gap-1.5 text-sm font-medium">Length
          <select className={ui.field} value={f.minutes} onChange={(e) => setF({ ...f, minutes: Number(e.target.value) })}>
            {DURATIONS.map((m) => <option key={m} value={m}>{m} minutes</option>)}
          </select>
        </label>
      </div>
      <div className="flex flex-wrap items-center gap-4">
        <label className="flex items-center gap-2 text-sm"><input type="checkbox" className="size-4 accent-[var(--brand)]" checked={f.startNow} onChange={(e) => setF({ ...f, startNow: e.target.checked })} /> Start right now</label>
        {!f.startNow && (
          <label className="flex items-center gap-2 text-sm font-medium">Starts at
            <input type="datetime-local" className={ui.field} value={f.start} onChange={(e) => setF({ ...f, start: e.target.value })} required />
          </label>
        )}
        <label className="flex items-center gap-2 text-sm"><input type="checkbox" className="size-4 accent-[var(--brand)]" checked={f.rated} onChange={(e) => setF({ ...f, rated: e.target.checked })} /> Rated games</label>
      </div>
      <button disabled={busy || f.name.trim().length < 3} className={`${ui.primary} justify-self-start`}>Create tournament</button>
    </form>
  );
}
