"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { createClient } from "@/lib/supabase/client";
import { friendlyError, ui } from "@/lib/social";
import { ReportButton } from "./ReportButton";

type Msg = { id: string; user_id: string; body: string; hidden: boolean; created_at: string; author: { username: string } | null };

type Props = {
  table: "game_messages" | "club_messages";
  scopeColumn: "game_id" | "club_id";
  scopeId: string;
  /** game chat only: which room (players talk to each other, spectators talk among themselves) */
  room?: "players" | "spectators";
  viewerId: string | null;
  isMod: boolean;
  title: string;
  maxLength: number;
  height?: string;
};

/** Live chat. Realtime pushes new messages; a short poll covers networks that block WebSockets. */
export function ChatBox({ table, scopeColumn, scopeId, room, viewerId, isMod, title, maxLength, height = "h-64" }: Props) {
  const [msgs, setMsgs] = useState<Msg[]>([]);
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const endRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLOListElement>(null);

  const load = useCallback(async () => {
    let q = createClient()
      .from(table)
      .select("id, user_id, body, hidden, created_at, author:profiles(username)")
      .eq(scopeColumn, scopeId)
      .order("created_at", { ascending: false })
      .limit(60);
    if (room) q = q.eq("room", room);
    const { data } = await q;
    setMsgs(((data as unknown as Msg[] | null) ?? []).reverse());
  }, [table, scopeColumn, scopeId, room]);

  useEffect(() => {
    if (!viewerId) return;
    queueMicrotask(load);
    const supabase = createClient();
    const channel = supabase
      .channel(`chat-${table}-${scopeId}-${room ?? "all"}`)
      .on("postgres_changes", { event: "*", schema: "public", table, filter: `${scopeColumn}=eq.${scopeId}` }, () => load())
      .subscribe();
    const timer = setInterval(() => document.visibilityState === "visible" && load(), 6000);
    return () => {
      clearInterval(timer);
      supabase.removeChannel(channel);
    };
  }, [viewerId, table, scopeColumn, scopeId, room, load]);

  // Keep the newest message in view, but only scroll the chat itself, never the page.
  useEffect(() => {
    const el = listRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [msgs.length]);

  if (!viewerId) {
    return (
      <section aria-label={title} className={ui.card}>
        <h2 className="text-sm font-bold">{title}</h2>
        <p className="mt-2 text-sm text-dim"><a className="text-brand underline" href="/login">Log in</a> to chat.</p>
      </section>
    );
  }

  const send = async (e: React.FormEvent) => {
    e.preventDefault();
    const body = text.trim();
    if (!body) return;
    setBusy(true);
    setError(null);
    const row: Record<string, string> = { user_id: viewerId, body, [scopeColumn]: scopeId };
    if (room) row.room = room;
    const { error: err } = await createClient().from(table).insert(row);
    setBusy(false);
    if (err) return setError(friendlyError(err));
    setText("");
    load();
  };

  const hide = async (m: Msg) => {
    const { error: err } = await createClient().rpc("mod_hide_message", { p_kind: table === "game_messages" ? "game" : "club", p_id: m.id, p_hidden: !m.hidden });
    if (err) setError(friendlyError(err));
    else load();
  };

  return (
    <section aria-label={title} className={`${ui.card} grid min-w-0 gap-3`}>
      <h2 className="text-sm font-bold">{title}</h2>
      <ol ref={listRef} aria-live="polite" className={`${height} min-w-0 overflow-y-auto rounded-md border border-line bg-bg p-2 text-sm`}>
        {msgs.length === 0 && <li className="p-2 text-dim">No messages yet. Say hi.</li>}
        {msgs.map((m) => (
          <li key={m.id} className={`group flex items-start gap-2 py-0.5 ${m.hidden ? "opacity-50" : ""}`}>
            <span className="min-w-0 flex-1 break-words">
              <strong className={m.user_id === viewerId ? "text-brand" : ""}>{m.author?.username ?? "?"}</strong>{" "}
              <span className={m.hidden ? "line-through" : ""}>{m.body}</span>
              {m.hidden && <em className="ml-1 text-xs text-dim">(hidden)</em>}
            </span>
            <span className="flex shrink-0 gap-2 opacity-0 transition group-focus-within:opacity-100 group-hover:opacity-100">
              {m.user_id !== viewerId && !m.hidden && (
                <ReportButton viewerId={viewerId} kind={table === "game_messages" ? "game_message" : "club_message"} refId={m.id} targetUser={m.user_id} />
              )}
              {isMod && <button type="button" onClick={() => hide(m)} className="text-xs text-dim hover:text-ink">{m.hidden ? "Show" : "Hide"}</button>}
            </span>
          </li>
        ))}
        <div ref={endRef} />
      </ol>
      {error && <p role="alert" className="text-xs text-danger">{error}</p>}
      <form onSubmit={send} className="flex gap-2">
        <input className={ui.field} value={text} onChange={(e) => setText(e.target.value)} maxLength={maxLength} placeholder="Write a message" aria-label="Message" autoComplete="off" />
        <button disabled={busy || !text.trim()} className={ui.primary}>Send</button>
      </form>
    </section>
  );
}
