"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { createClient } from "@/lib/supabase/client";
import { fmtDate, friendlyError, ui } from "@/lib/social";
import { ReportButton } from "./ReportButton";

export type ThreadInfo = { id: string; title: string; pinned: boolean; locked: boolean; deleted_at: string | null; author_id: string; category: { slug: string; name: string } | null };
export type Post = { id: string; author_id: string; body: string; created_at: string; edited_at: string | null; deleted_at: string | null; deleted_by_mod: boolean; author: { username: string } | null };

export function NewThread({ categoryId, slug }: { categoryId: string; slug: string }) {
  const router = useRouter();
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    const { data, error: err } = await createClient().rpc("create_forum_thread", { p_category: categoryId, p_title: title, p_body: body });
    setBusy(false);
    if (err) return setError(friendlyError(err));
    router.push(`/forum/thread/${data as string}`);
    void slug;
  };

  return (
    <form onSubmit={submit} className={`${ui.card} grid max-w-2xl gap-3`}>
      <h2 className="text-xl font-bold">Start a thread</h2>
      {error && <p role="alert" className={ui.err}>{error}</p>}
      <label className="grid gap-1.5 text-sm font-medium">Title
        <input className={ui.field} value={title} onChange={(e) => setTitle(e.target.value)} minLength={3} maxLength={120} required />
      </label>
      <label className="grid gap-1.5 text-sm font-medium">Message
        <textarea className={ui.field} rows={5} value={body} onChange={(e) => setBody(e.target.value)} maxLength={4000} required />
      </label>
      <button disabled={busy || title.trim().length < 3 || !body.trim()} className={`${ui.primary} justify-self-start`}>Post thread</button>
    </form>
  );
}

export function ThreadView({ thread, posts, viewerId, isMod }: { thread: ThreadInfo; posts: Post[]; viewerId: string | null; isMod: boolean }) {
  const router = useRouter();
  const supabase = createClient();
  const [reply, setReply] = useState("");
  const [editing, setEditing] = useState<{ id: string; body: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async (fn: () => PromiseLike<{ error: { message?: string; code?: string } | null }>) => {
    setBusy(true);
    setError(null);
    const { error: err } = await fn();
    setBusy(false);
    if (err) setError(friendlyError(err));
    else router.refresh();
    return !err;
  };

  // Deleted posts are hidden from everyone except moderators.
  const visible = posts.filter((p) => !p.deleted_at || isMod);
  const removedThread = !!thread.deleted_at;

  const mod = (kind: "post" | "thread", id: string, action: string) => run(() => supabase.rpc("mod_forum", { p_kind: kind, p_id: id, p_action: action }));

  return (
    <div className="grid min-w-0 gap-4">
      <header className="grid gap-2">
        <h1 className="text-2xl font-extrabold tracking-tight">{thread.pinned && "📌 "}{thread.locked && "🔒 "}{thread.title}</h1>
        {removedThread && <p className={ui.err}>This thread was removed by a moderator.</p>}
        {isMod && (
          <div className="flex flex-wrap gap-2" aria-label="Moderator tools">
            <button disabled={busy} className={ui.btn} onClick={() => mod("thread", thread.id, thread.locked ? "unlock" : "lock")}>{thread.locked ? "Unlock" : "Lock"}</button>
            <button disabled={busy} className={ui.btn} onClick={() => mod("thread", thread.id, thread.pinned ? "unpin" : "pin")}>{thread.pinned ? "Unpin" : "Pin"}</button>
            <button disabled={busy} className={ui.danger} onClick={() => mod("thread", thread.id, removedThread ? "restore" : "remove")}>{removedThread ? "Restore thread" : "Remove thread"}</button>
          </div>
        )}
      </header>
      {error && <p role="alert" className={ui.err}>{error}</p>}

      <ol className="grid gap-3">
        {visible.map((p) => {
          const removed = !!p.deleted_at;
          const mine = p.author_id === viewerId;
          return (
            <li key={p.id} id={`p-${p.id}`} className={`${ui.card} min-w-0 py-4 ${removed ? "opacity-60" : ""}`}>
              <div className="flex flex-wrap items-baseline gap-x-3 text-xs text-dim">
                <Link href={`/u/${p.author?.username}`} className="text-sm font-bold text-ink hover:underline">{p.author?.username ?? "?"}</Link>
                <span>{fmtDate(p.created_at)}{p.edited_at ? " (edited)" : ""}</span>
                {removed && <span className="text-danger">{p.deleted_by_mod ? "removed by a moderator" : "deleted by author"}</span>}
              </div>
              {editing?.id === p.id ? (
                <form className="mt-2 grid gap-2" onSubmit={async (e) => { e.preventDefault(); if (await run(() => supabase.from("forum_posts").update({ body: editing.body.trim() }).eq("id", p.id))) setEditing(null); }}>
                  <textarea className={ui.field} rows={4} value={editing.body} maxLength={4000} onChange={(e) => setEditing({ id: p.id, body: e.target.value })} />
                  <span className="flex gap-2"><button disabled={busy || !editing.body.trim()} className={ui.primary}>Save</button><button type="button" className={ui.btn} onClick={() => setEditing(null)}>Cancel</button></span>
                </form>
              ) : (
                <p className="mt-2 whitespace-pre-wrap break-words">{p.body}</p>
              )}
              {viewerId && !editing && (
                <div className="mt-2 flex flex-wrap gap-3 text-xs">
                  {mine && !removed && !thread.locked && <button className="text-dim hover:text-ink" onClick={() => setEditing({ id: p.id, body: p.body })}>Edit</button>}
                  {mine && !removed && <button className="text-dim hover:text-danger" onClick={() => confirm("Delete your post?") && run(() => supabase.from("forum_posts").update({ deleted_at: new Date().toISOString() }).eq("id", p.id))}>Delete</button>}
                  {!mine && !removed && <ReportButton viewerId={viewerId} kind="forum_post" refId={p.id} targetUser={p.author_id} />}
                  {isMod && <button className="text-dim hover:text-ink" onClick={() => mod("post", p.id, removed ? "restore" : "remove")}>{removed ? "Restore" : "Remove"}</button>}
                </div>
              )}
            </li>
          );
        })}
      </ol>

      {!viewerId ? (
        <p className={ui.info}><Link className="text-brand underline" href="/login">Log in</Link> to reply.</p>
      ) : thread.locked || removedThread ? (
        <p className={ui.info}>This thread is closed.</p>
      ) : (
        <form className={`${ui.card} grid gap-3`} onSubmit={async (e) => { e.preventDefault(); if (await run(() => supabase.from("forum_posts").insert({ thread_id: thread.id, author_id: viewerId, body: reply.trim() }))) setReply(""); }}>
          <label className="grid gap-1.5 text-sm font-medium">Reply
            <textarea className={ui.field} rows={4} value={reply} maxLength={4000} onChange={(e) => setReply(e.target.value)} />
          </label>
          <button disabled={busy || !reply.trim()} className={`${ui.primary} justify-self-start`}>Post reply</button>
        </form>
      )}
    </div>
  );
}
