"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";
import { achievementOf, type Achievement } from "@/lib/achievements";
import { play } from "@/lib/sounds";
import { createClient } from "@/lib/supabase/client";

const THROTTLE_MS = 45_000;

/** Quietly asks the database "did this player earn any badges?" after page loads, and announces new ones. */
export function AchievementToaster() {
  const pathname = usePathname();
  const [toasts, setToasts] = useState<(Achievement & { id: number })[]>([]);
  const [username, setUsername] = useState<string | null>(null);
  const last = useRef(0);
  const seq = useRef(0);

  const check = useCallback(async (force = false) => {
    if (!force && Date.now() - last.current < THROTTLE_MS) return;
    last.current = Date.now();
    try {
      const supabase = createClient();
      const { data: sess } = await supabase.auth.getSession();
      const uid = sess.session?.user.id;
      if (!uid) return;
      const { data, error } = await supabase.rpc("check_achievements");
      if (error || !Array.isArray(data) || data.length === 0) return;
      const { data: me } = await supabase.from("profiles").select("username").eq("id", uid).maybeSingle();
      setUsername((me as { username: string } | null)?.username ?? null);
      const fresh = (data as string[]).map(achievementOf).filter((a): a is Achievement => !!a);
      if (fresh.length === 0) return;
      play("notify");
      const items = fresh.map((a) => ({ ...a, id: ++seq.current }));
      setToasts((cur) => [...cur, ...items]);
      setTimeout(() => setToasts((cur) => cur.filter((t) => !items.some((i) => i.id === t.id))), 9000);
    } catch {
      /* offline: try again next time */
    }
  }, []);

  useEffect(() => {
    const t = setTimeout(check, 2500);
    return () => clearTimeout(t);
  }, [pathname, check]);
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const onOnline = () => check();
    // Something just happened (a game ended, a lesson was solved): look again in a few seconds, once the database has caught up.
    const onProgress = () => {
      clearTimeout(timer);
      timer = setTimeout(() => check(true), 3000);
    };
    window.addEventListener("online", onOnline);
    window.addEventListener("rookery:progress", onProgress);
    return () => {
      clearTimeout(timer);
      window.removeEventListener("online", onOnline);
      window.removeEventListener("rookery:progress", onProgress);
    };
  }, [check]);

  if (toasts.length === 0) return null;
  return (
    <div role="status" aria-live="polite" className="pointer-events-none fixed bottom-4 right-4 z-50 grid w-[min(22rem,calc(100vw-2rem))] gap-2">
      {toasts.map((t) => (
        <div key={t.id} className="pointer-events-auto flex items-start gap-3 rounded-xl border border-brand bg-surface p-3 shadow-xl">
          <span aria-hidden className="text-3xl">{t.icon}</span>
          <span className="min-w-0 flex-1">
            <span className="block text-xs font-semibold uppercase tracking-wide text-brand">Achievement unlocked</span>
            <span className="block font-bold">{t.name}</span>
            <span className="block text-sm text-dim">{t.desc}</span>
            {username && <Link href={`/u/${username}`} className="text-xs font-semibold text-brand underline">See all your badges</Link>}
          </span>
          <button aria-label="Dismiss" onClick={() => setToasts((cur) => cur.filter((x) => x.id !== t.id))} className="text-dim hover:text-ink">✕</button>
        </div>
      ))}
    </div>
  );
}
