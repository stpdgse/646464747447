"use client";

import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import { createClient } from "@/lib/supabase/client";
import { REPORT_REASONS, USERNAME_RE, fmtDate, friendlyError, isFuture, ui } from "@/lib/social";
import { FairplayTab } from "./FairplayTab";

type Tab = "reports" | "fairplay" | "users" | "log";
const TABS: { id: Tab; label: string }[] = [
  { id: "reports", label: "Reports" }, { id: "fairplay", label: "Fair play" }, { id: "users", label: "Users" }, { id: "log", label: "Audit log" },
];

export function AdminPanel({ initialTab, initialGame, initialUser, isAdmin }: { initialTab: Tab; initialGame: string; initialUser: string; isAdmin: boolean }) {
  const [tab, setTab] = useState<Tab>(initialTab);
  return (
    <div className="grid min-w-0 gap-6">
      <div role="tablist" aria-label="Admin sections" className="flex flex-wrap gap-2">
        {TABS.map((t) => (
          <button key={t.id} role="tab" aria-selected={tab === t.id} onClick={() => setTab(t.id)}
            className={`rounded-md px-4 py-2 text-sm font-semibold ${tab === t.id ? "bg-brand text-brand-ink" : "border border-line bg-surface hover:bg-surface-2"}`}>{t.label}</button>
        ))}
      </div>
      {tab === "reports" && <Reports isAdmin={isAdmin} />}
      {tab === "fairplay" && <FairplayTab initialGame={initialGame} />}
      {tab === "users" && <Users initialUser={initialUser} isAdmin={isAdmin} />}
      {tab === "log" && <Log />}
    </div>
  );
}

// ---------------------------------------------------------------------------------------------------------------------

type Report = {
  id: number; kind: string; ref_id: string; reason: string; details: string; status: "open" | "actioned" | "dismissed"; note: string | null; created_at: string;
  target_user: string | null;
  reporter: { username: string } | null; target: { username: string } | null; handler: { username: string } | null;
};

function Reports({ isAdmin }: { isAdmin: boolean }) {
  const [status, setStatus] = useState<"open" | "done">("open");
  const [rows, setRows] = useState<Report[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notes, setNotes] = useState<Record<number, string>>({});
  const [shown, setShown] = useState<Record<number, string>>({});

  const load = useCallback(async () => {
    let q = createClient()
      .from("reports")
      .select("id, kind, ref_id, reason, details, status, note, created_at, target_user, reporter:profiles!reports_reporter_id_fkey(username), target:profiles!reports_target_user_fkey(username), handler:profiles!reports_handled_by_fkey(username)")
      .order("created_at", { ascending: status === "open" })
      .limit(100);
    q = status === "open" ? q.eq("status", "open") : q.neq("status", "open");
    const { data } = await q;
    setRows((data as unknown as Report[] | null) ?? []);
  }, [status]);

  useEffect(() => {
    queueMicrotask(load);
  }, [load]);

  const supabase = createClient();
  const act = async (r: Report, outcome: "actioned" | "dismissed", mute?: number, ban?: number) => {
    setError(null);
    if (r.target_user && mute) {
      const e = (await supabase.rpc("mod_mute", { p_user: r.target_user, p_hours: mute, p_reason: `report #${r.id}: ${r.reason}` })).error;
      if (e) return setError(friendlyError(e));
    }
    if (r.target_user && ban) {
      const e = (await supabase.rpc("mod_ban", { p_user: r.target_user, p_days: ban, p_reason: `report #${r.id}: ${r.reason}` })).error;
      if (e) return setError(friendlyError(e));
    }
    const e = (await supabase.rpc("mod_resolve_report", { p_report: r.id, p_status: outcome, p_note: notes[r.id] ?? "" })).error;
    if (e) return setError(friendlyError(e));
    load();
  };

  const preview = async (r: Report) => {
    let text = "(not found)";
    if (r.kind === "game_message" || r.kind === "club_message") {
      const { data } = await supabase.from(r.kind === "game_message" ? "game_messages" : "club_messages").select("body").eq("id", r.ref_id).maybeSingle();
      text = data?.body ?? text;
    } else if (r.kind === "forum_post") {
      const { data } = await supabase.from("forum_posts").select("body").eq("id", r.ref_id).maybeSingle();
      text = data?.body ?? text;
    } else if (r.kind === "forum_thread") {
      const { data } = await supabase.from("forum_threads").select("title").eq("id", r.ref_id).maybeSingle();
      text = data?.title ?? text;
    }
    setShown((s) => ({ ...s, [r.id]: text }));
  };

  const hideMsg = async (r: Report) => {
    const { error: e } = await supabase.rpc("mod_hide_message", { p_kind: r.kind === "game_message" ? "game" : "club", p_id: r.ref_id, p_hidden: true });
    if (e) setError(friendlyError(e)); else setShown((s) => ({ ...s, [r.id]: `${s[r.id] ?? ""}  [hidden]` }));
  };
  const removePost = async (r: Report) => {
    const { error: e } = await supabase.rpc("mod_forum", { p_kind: r.kind === "forum_post" ? "post" : "thread", p_id: r.ref_id, p_action: "remove" });
    if (e) setError(friendlyError(e)); else setShown((s) => ({ ...s, [r.id]: `${s[r.id] ?? ""}  [removed]` }));
  };

  const link = (r: Report) =>
    r.kind === "user" ? <Link className="text-brand underline" href={`/u/${r.target?.username ?? ""}`}>profile</Link>
      : r.kind === "game" ? <Link className="text-brand underline" href={`/game/${r.ref_id}`}>game</Link>
      : r.kind === "forum_thread" ? <Link className="text-brand underline" href={`/forum/thread/${r.ref_id}`}>thread</Link>
      : null;

  return (
    <div className="grid min-w-0 gap-4">
      <div className="flex gap-2">
        <button className={status === "open" ? ui.primary : ui.btn} onClick={() => { setRows(null); setStatus("open"); }}>Open</button>
        <button className={status === "done" ? ui.primary : ui.btn} onClick={() => { setRows(null); setStatus("done"); }}>Handled</button>
      </div>
      {error && <p role="alert" className={ui.err}>{error}</p>}
      {rows === null ? <div className="h-24 animate-pulse rounded-xl bg-surface" /> : rows.length === 0 ? <p className="text-dim">{status === "open" ? "Nothing to review. 🎉" : "Nothing handled yet."}</p> : (
        <ul className="grid gap-3">
          {rows.map((r) => (
            <li key={r.id} className={`${ui.card} grid gap-2 py-4`}>
              <p className="flex flex-wrap items-baseline gap-x-3 text-sm">
                <strong>{REPORT_REASONS.find((x) => x.id === r.reason)?.label ?? r.reason}</strong>
                <span className="text-dim">{r.kind.replace("_", " ")}</span>
                {r.target && <span>against <Link className="font-semibold hover:underline" href={`/u/${r.target.username}`}>{r.target.username}</Link></span>}
                <span className="text-dim">reported by {r.reporter?.username ?? "?"} · {fmtDate(r.created_at)}</span>
                {link(r)}
              </p>
              {r.details && <p className="text-sm">&ldquo;{r.details}&rdquo;</p>}
              {["game_message", "club_message", "forum_post", "forum_thread"].includes(r.kind) && (
                shown[r.id] !== undefined ? <p className="whitespace-pre-wrap break-words rounded-md border border-line bg-bg p-2 text-sm">{shown[r.id]}</p>
                  : <button className={`${ui.btn} justify-self-start`} onClick={() => preview(r)}>Show content</button>
              )}
              {r.status === "open" ? (
                <>
                  <input className={ui.field} placeholder="Note (optional)" maxLength={500} value={notes[r.id] ?? ""} onChange={(e) => setNotes({ ...notes, [r.id]: e.target.value })} aria-label="Note" />
                  <div className="flex flex-wrap gap-2">
                    <button className={ui.btn} onClick={() => act(r, "dismissed")}>Dismiss</button>
                    <button className={ui.btn} onClick={() => act(r, "actioned")}>Mark handled</button>
                    {r.target_user && (<>
                      <button className={ui.btn} onClick={() => act(r, "actioned", 1)}>Mute 1h</button>
                      <button className={ui.btn} onClick={() => act(r, "actioned", 24)}>Mute 24h</button>
                      <button className={ui.btn} onClick={() => act(r, "actioned", 168)}>Mute 7d</button>
                      {isAdmin && <button className={ui.danger} onClick={() => confirm("Suspend this account for 7 days?") && act(r, "actioned", undefined, 7)}>Ban 7d</button>}
                    </>)}
                    {(r.kind === "game_message" || r.kind === "club_message") && <button className={ui.btn} onClick={() => hideMsg(r)}>Hide message</button>}
                    {(r.kind === "forum_post" || r.kind === "forum_thread") && <button className={ui.btn} onClick={() => removePost(r)}>Remove</button>}
                    {r.kind === "game" && <Link className={ui.btn} href={`/admin?tab=fairplay&game=${r.ref_id}`}>Fair-play check</Link>}
                  </div>
                </>
              ) : (
                <p className="text-xs text-dim">{r.status} by {r.handler?.username ?? "?"}{r.note ? `: ${r.note}` : ""}</p>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------------------------------------------------

type U = { id: string; username: string; is_admin: boolean; is_moderator: boolean; muted_until: string | null; banned_until: string | null; created_at: string };

function Users({ initialUser, isAdmin }: { initialUser: string; isAdmin: boolean }) {
  const supabase = createClient();
  const [q, setQ] = useState(initialUser);
  const [users, setUsers] = useState<U[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reason, setReason] = useState("");

  const search = useCallback(async (text: string) => {
    let query = createClient().from("profiles").select("id, username, is_admin, is_moderator, muted_until, banned_until, created_at").order("created_at", { ascending: false }).limit(40);
    const t = text.trim();
    if (t) query = query.ilike("username", `%${t.replace(/[%_]/g, "")}%`);
    const { data } = await query;
    setUsers((data as U[] | null) ?? []);
  }, []);

  useEffect(() => {
    queueMicrotask(() => search(initialUser));
  }, [search, initialUser]);

  const call = async (fn: () => PromiseLike<{ error: { message?: string; code?: string } | null }>) => {
    setError(null);
    const { error: e } = await fn();
    if (e) setError(friendlyError(e));
    search(q);
  };

  return (
    <div className="grid min-w-0 gap-4">
      <form onSubmit={(e) => { e.preventDefault(); search(q); }} className="flex max-w-lg gap-2">
        <input className={ui.field} value={q} onChange={(e) => setQ(e.target.value)} placeholder="Search username" aria-label="Search username" autoCapitalize="none" spellCheck={false} />
        <button className={ui.primary}>Search</button>
      </form>
      <label className="grid max-w-lg gap-1 text-xs font-medium">Reason (saved in the audit log)
        <input className={ui.field} value={reason} onChange={(e) => setReason(e.target.value)} maxLength={300} />
      </label>
      {error && <p role="alert" className={ui.err}>{error}</p>}
      {users === null ? <div className="h-24 animate-pulse rounded-xl bg-surface" /> : users.length === 0 ? <p className="text-dim">No matching users.</p> : (
        <ul className="grid gap-2">
          {users.map((u) => {
            const muted = isFuture(u.muted_until);
            const banned = isFuture(u.banned_until);
            const protectedAcct = u.is_admin || (u.is_moderator && !isAdmin);
            return (
              <li key={u.id} className={`${ui.card} grid gap-2 py-3`}>
                <p className="flex flex-wrap items-baseline gap-x-3">
                  <Link href={`/u/${u.username}`} className="font-semibold hover:underline">{u.username}</Link>
                  {u.is_admin && <span className="rounded bg-surface-2 px-1.5 text-xs text-brand">admin</span>}
                  {u.is_moderator && <span className="rounded bg-surface-2 px-1.5 text-xs text-brand">moderator</span>}
                  {muted && <span className="text-xs text-danger">muted until {fmtDate(u.muted_until!)}</span>}
                  {banned && <span className="text-xs text-danger">banned until {fmtDate(u.banned_until!)}</span>}
                  <span className="text-xs text-dim">joined {fmtDate(u.created_at)}</span>
                </p>
                {!protectedAcct && (
                  <div className="flex flex-wrap gap-2">
                    {muted ? <button className={ui.btn} onClick={() => call(() => supabase.rpc("mod_mute", { p_user: u.id, p_hours: 0, p_reason: reason }))}>Unmute</button> : (
                      [[1, "Mute 1h"], [24, "Mute 24h"], [168, "Mute 7d"]] as const).map(([h, l]) => (
                      <button key={h} className={ui.btn} onClick={() => call(() => supabase.rpc("mod_mute", { p_user: u.id, p_hours: h, p_reason: reason }))}>{l}</button>
                    ))}
                    {isAdmin && (banned ? <button className={ui.btn} onClick={() => call(() => supabase.rpc("mod_ban", { p_user: u.id, p_days: 0, p_reason: reason }))}>Lift ban</button> : (
                      [[1, "Ban 1d"], [7, "Ban 7d"], [36500, "Ban for good"]] as const).map(([d, l]) => (
                      <button key={d} className={ui.danger} onClick={() => confirm(`${l} for ${u.username}?`) && call(() => supabase.rpc("mod_ban", { p_user: u.id, p_days: d, p_reason: reason }))}>{l}</button>
                    )))}
                    {isAdmin && <button className={ui.btn} onClick={() => call(() => supabase.rpc("admin_set_moderator", { p_user: u.id, p_on: !u.is_moderator }))}>{u.is_moderator ? "Remove moderator" : "Make moderator"}</button>}
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
      <p className="text-xs text-dim">Usernames must match {String(USERNAME_RE)}.</p>
    </div>
  );
}

// ---------------------------------------------------------------------------------------------------------------------

type Entry = { id: number; action: string; ref_kind: string | null; ref_id: string | null; detail: string | null; created_at: string; actor: { username: string } | null; target: { username: string } | null };

function Log() {
  const [rows, setRows] = useState<Entry[] | null>(null);
  useEffect(() => {
    createClient()
      .from("mod_actions")
      .select("id, action, ref_kind, ref_id, detail, created_at, actor:profiles!mod_actions_actor_id_fkey(username), target:profiles!mod_actions_target_user_fkey(username)")
      .order("created_at", { ascending: false })
      .limit(150)
      .then(({ data }) => setRows((data as unknown as Entry[] | null) ?? []));
  }, []);
  if (rows === null) return <div className="h-24 animate-pulse rounded-xl bg-surface" />;
  if (rows.length === 0) return <p className="text-dim">No moderator actions yet.</p>;
  return (
    <ul className="grid gap-1 text-sm">
      {rows.map((r) => (
        <li key={r.id} className="flex flex-wrap gap-x-3 border-b border-line py-1.5">
          <span className="text-xs text-dim">{fmtDate(r.created_at)}</span>
          <strong>{r.actor?.username ?? "?"}</strong>
          <span>{r.action.replaceAll("_", " ")}</span>
          {r.target && <span>→ {r.target.username}</span>}
          {r.detail && <span className="text-dim">{r.detail}</span>}
        </li>
      ))}
    </ul>
  );
}
