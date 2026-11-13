"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { loadSaved, SAVE_KEY } from "@/components/computer/saved";
import { getBot } from "@/lib/engine/bots";
import { createClient } from "@/lib/supabase/client";

type Pending = { href: string; text: string } | null;

/**
 * "You have a game in progress": shown on every page except the game itself, so closing the app, losing signal or wandering
 * off never loses a game. Covers games against the computer (saved in this browser) and live online games.
 */
export function ResumeBanner() {
  const pathname = usePathname();
  const [bot, setBot] = useState<Pending>(null);
  const [live, setLive] = useState<Pending>(null);

  const check = useCallback(async () => {
    const saved = loadSaved(Date.now());
    const name = saved && !saved.end ? getBot(saved.botId)?.name : null;
    setBot(saved && !saved.end && name ? { href: "/play/computer", text: `You have an unfinished game against ${name}.` } : null);
    try {
      const supabase = createClient();
      const { data: sess } = await supabase.auth.getSession();
      const uid = sess.session?.user.id;
      if (!uid) return setLive(null);
      const { data } = await supabase
        .from("games")
        .select("id, white_id, white:profiles!games_white_id_fkey(username), black:profiles!games_black_id_fkey(username)")
        .eq("status", "started")
        .eq("mode", "live")
        .or(`white_id.eq.${uid},black_id.eq.${uid}`)
        .limit(1);
      const g = data?.[0] as unknown as { id: string; white_id: string; white: { username: string } | null; black: { username: string } | null } | undefined;
      if (!g) return setLive(null);
      const opp = g.white_id === uid ? g.black?.username : g.white?.username;
      setLive({ href: `/game/${g.id}`, text: `Your game against ${opp ?? "your opponent"} is still going.` });
    } catch {
      /* offline: keep whatever we showed */
    }
  }, []);

  useEffect(() => {
    queueMicrotask(check);
    const onVisible = () => document.visibilityState === "visible" && check();
    const onStorage = (e: StorageEvent) => e.key === SAVE_KEY && check();
    document.addEventListener("visibilitychange", onVisible);
    window.addEventListener("online", check);
    window.addEventListener("focus", check);
    window.addEventListener("storage", onStorage);
    const timer = setInterval(onVisible, 30_000);
    return () => {
      clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisible);
      window.removeEventListener("online", check);
      window.removeEventListener("focus", check);
      window.removeEventListener("storage", onStorage);
    };
  }, [check, pathname]);

  const items = [live, bot].filter((x): x is NonNullable<Pending> => !!x).filter((x) => !pathname.startsWith(x.href))
    .filter((x) => !(x.href.startsWith("/game/") && pathname.startsWith("/play/online"))); // the lobby shows its own Resume
  if (items.length === 0) return null;
  return (
    <div role="status" className="border-b border-brand bg-brand/15 px-4 py-2">
      <div className="mx-auto flex max-w-6xl flex-col gap-1">
        {items.map((i) => (
          <p key={i.href} className="flex flex-wrap items-center gap-3 text-sm">
            <span className="font-semibold">{i.text}</span>
            <Link href={i.href} className="rounded-md bg-brand px-3 py-1 font-semibold text-brand-ink hover:bg-brand-hi">Resume</Link>
          </p>
        ))}
      </div>
    </div>
  );
}
