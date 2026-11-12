"use client";

import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";
import { play } from "@/lib/sounds";
import { createClient } from "@/lib/supabase/client";

type Item = {
  id: number; kind: "your_turn" | "game_over" | "friend_request" | "friend_accepted" | "club_invite"; game_id: string | null; created_at: string; read_at: string | null;
  actor: { username: string } | null; club: { slug: string; name: string } | null;
  game: { white_id: string; black_id: string; white: { username: string } | null; black: { username: string } | null } | null;
};

/** In-app notifications: "your move" in daily games, and results. Updates instantly via Realtime, with a slow poll as backup. */
export function NotificationBell({ userId }: { userId: string }) {
  const [items, setItems] = useState<Item[]>([]);
  const [open, setOpen] = useState(false);
  const boxRef = useRef<HTMLDivElement>(null);

  const load = useCallback(async () => {
    const { data } = await createClient()
      .from("notifications")
      .select("id, kind, game_id, created_at, read_at, actor:profiles!notifications_actor_id_fkey(username), club:clubs(slug, name), game:games(white_id, black_id, white:profiles!games_white_id_fkey(username), black:profiles!games_black_id_fkey(username))")
      .order("created_at", { ascending: false })
      .limit(15);
    setItems((data as unknown as Item[] | null) ?? []);
  }, []);

  useEffect(() => {
    queueMicrotask(() => load());
    const supabase = createClient();
    const channel = supabase
      .channel(`notifications-${userId}`)
      .on("postgres_changes", { event: "INSERT", schema: "public", table: "notifications", filter: `user_id=eq.${userId}` }, () => load())
      .subscribe();
    const timer = setInterval(() => document.visibilityState === "visible" && load(), 30_000);
    return () => {
      clearInterval(timer);
      supabase.removeChannel(channel);
    };
  }, [userId, load]);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => !boxRef.current?.contains(e.target as Node) && setOpen(false);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const hrefOf = (i: Item) =>
    i.kind === "friend_request" || i.kind === "friend_accepted" ? "/community/friends"
      : i.kind === "club_invite" ? (i.club ? `/clubs/${i.club.slug}` : "/clubs")
      : i.game_id ? `/game/${i.game_id}` : "/play/daily";

  const unread = items.filter((i) => !i.read_at).length;
  const prevUnread = useRef<number | null>(null);
  useEffect(() => {
    if (prevUnread.current !== null && unread > prevUnread.current) play("notify");
    prevUnread.current = unread;
  }, [unread]);

  const toggle = async () => {
    const next = !open;
    setOpen(next);
    if (next && unread > 0) {
      await createClient().from("notifications").update({ read_at: new Date().toISOString() }).is("read_at", null);
      // keep the highlight for this view; reload quietly afterwards
      setTimeout(load, 1500);
    }
  };

  const text = (i: Item) => {
    const who = i.actor?.username ?? "Someone";
    if (i.kind === "friend_request") return `${who} sent you a friend request`;
    if (i.kind === "friend_accepted") return `${who} accepted your friend request`;
    if (i.kind === "club_invite") return `${who} invited you to ${i.club?.name ?? "a club"}`;
    const opp = i.game ? (i.game.white_id === userId ? i.game.black?.username : i.game.white?.username) : null;
    return i.kind === "your_turn" ? `Your move against ${opp ?? "your opponent"}` : `Game over against ${opp ?? "your opponent"}`;
  };

  return (
    <div ref={boxRef} className="relative">
      <button onClick={toggle} aria-haspopup="true" aria-expanded={open} aria-label={unread ? `${unread} unread notifications` : "Notifications"}
        className="relative rounded-md px-2.5 py-1.5 text-dim hover:text-ink">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
          <path d="M18 8a6 6 0 0 0-12 0c0 7-3 9-3 9h18s-3-2-3-9" /><path d="M13.7 21a2 2 0 0 1-3.4 0" />
        </svg>
        {unread > 0 && <span className="absolute -right-0.5 -top-0.5 grid min-w-4 place-items-center rounded-full bg-brand px-1 text-[10px] font-bold leading-4 text-brand-ink">{unread > 9 ? "9+" : unread}</span>}
      </button>
      {open && (
        <div role="menu" className="absolute right-0 z-50 mt-2 w-72 overflow-hidden rounded-xl border border-line bg-surface shadow-xl">
          {items.length === 0 ? (
            <p className="px-4 py-6 text-center text-sm text-dim">Nothing yet.</p>
          ) : (
            <ul className="max-h-80 overflow-auto">
              {items.map((i) => (
                <li key={i.id} className="border-b border-line last:border-0">
                  <Link href={hrefOf(i)} onClick={() => setOpen(false)} className={`block px-4 py-3 text-sm hover:bg-surface-2 ${i.read_at ? "text-dim" : "font-semibold"}`}>
                    {text(i)}
                    <span className="block text-xs font-normal text-dim">{new Date(i.created_at).toLocaleString("en", { dateStyle: "medium", timeStyle: "short" })}</span>
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
