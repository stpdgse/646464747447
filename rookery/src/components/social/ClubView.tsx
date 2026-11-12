"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { createClient } from "@/lib/supabase/client";
import { USERNAME_RE, friendlyError, ui } from "@/lib/social";
import { ChatBox } from "./ChatBox";

export type Club = { id: string; slug: string; name: string; description: string; owner_id: string; open: boolean; archived: boolean };
export type Member = { user_id: string; role: "owner" | "moderator" | "member"; status: "active" | "requested" | "invited"; joined_at: string; user: { username: string } | null };

export function ClubView({ club, initialMembers, viewerId, isMod }: { club: Club; initialMembers: Member[]; viewerId: string | null; isMod: boolean }) {
  const router = useRouter();
  const members = initialMembers;
  const mine = members.find((m) => m.user_id === viewerId) ?? null;
  const active = members.filter((m) => m.status === "active");
  const requests = members.filter((m) => m.status === "requested");
  const invited = members.filter((m) => m.status === "invited");
  const myRole = mine?.status === "active" ? mine.role : null;
  const manager = myRole === "owner" || myRole === "moderator";
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [invite, setInvite] = useState("");
  const [edit, setEdit] = useState({ name: club.name, description: club.description, open: club.open });
  const supabase = createClient();

  const run = async (fn: () => PromiseLike<{ error: { message?: string; code?: string } | null }>) => {
    setBusy(true);
    setError(null);
    const { error: err } = await fn();
    setBusy(false);
    if (err) setError(friendlyError(err));
    else router.refresh();
    return !err;
  };

  const join = () => run(() => supabase.from("club_members").insert({ club_id: club.id, user_id: viewerId, role: "member", status: club.open ? "active" : "requested" }));
  const leave = () => run(() => supabase.from("club_members").delete().eq("club_id", club.id).eq("user_id", viewerId!));
  const approve = (uid: string) => run(() => supabase.from("club_members").update({ status: "active" }).eq("club_id", club.id).eq("user_id", uid));
  const kick = (uid: string) => run(() => supabase.from("club_members").delete().eq("club_id", club.id).eq("user_id", uid));
  const setRole = (uid: string, role: "moderator" | "member") => run(() => supabase.rpc("club_set_role", { p_club: club.id, p_user: uid, p_role: role }));

  const sendInvite = async (e: React.FormEvent) => {
    e.preventDefault();
    const name = invite.trim();
    if (!USERNAME_RE.test(name)) return setError("That isn't a valid username.");
    const { data: t } = await supabase.from("profiles").select("id").eq("username", name).maybeSingle();
    if (!t) return setError(`Nobody is called ${name}.`);
    if (await run(() => supabase.from("club_members").insert({ club_id: club.id, user_id: t.id, role: "member", status: "invited" }))) setInvite("");
  };

  const save = async (e: React.FormEvent) => {
    e.preventDefault();
    await run(() => supabase.from("clubs").update({ name: edit.name.trim(), description: edit.description.trim(), open: edit.open }).eq("id", club.id));
  };

  return (
    <div className="grid min-w-0 gap-6">
      <header className="grid gap-2">
        <p className="text-sm"><Link href="/clubs" className="text-dim hover:text-ink">← All clubs</Link></p>
        <h1 className="text-3xl font-extrabold tracking-tight">{club.name}</h1>
        {club.description && <p className="whitespace-pre-line text-dim">{club.description}</p>}
        <p className="text-xs font-semibold text-dim">{active.length} members · {club.open ? "anyone can join" : "join by request"}{club.archived ? " · archived" : ""}</p>
      </header>

      {error && <p role="alert" className={ui.err}>{error}</p>}

      {viewerId && (
        <div className="flex flex-wrap items-center gap-3">
          {!mine && <button disabled={busy} onClick={join} className={ui.primary}>{club.open ? "Join club" : "Request to join"}</button>}
          {mine?.status === "requested" && (<><span className="text-sm text-dim">Request sent.</span><button disabled={busy} onClick={leave} className={ui.btn}>Cancel</button></>)}
          {mine?.status === "invited" && (
            <>
              <span className="text-sm">You were invited.</span>
              <button disabled={busy} onClick={() => approve(viewerId)} className={ui.primary}>Accept</button>
              <button disabled={busy} onClick={leave} className={ui.btn}>Decline</button>
            </>
          )}
          {mine?.status === "active" && myRole !== "owner" && <button disabled={busy} onClick={leave} className={ui.btn}>Leave club</button>}
        </div>
      )}

      <div className="grid min-w-0 gap-6 lg:grid-cols-[1fr_20rem]">
        <div className="grid min-w-0 content-start gap-6">
          {myRole || isMod ? (
            <ChatBox table="club_messages" scopeColumn="club_id" scopeId={club.id} viewerId={viewerId} isMod={isMod} title="Club chat" maxLength={300} height="h-80" />
          ) : (
            <p className={ui.info}>{viewerId ? "Join the club to read and write in its chat." : "Log in and join to chat."}</p>
          )}

          {manager && requests.length > 0 && (
            <section aria-labelledby="rq" className="grid gap-2">
              <h2 id="rq" className="text-lg font-bold">Join requests</h2>
              <ul className="grid gap-2">
                {requests.map((m) => (
                  <li key={m.user_id} className={`${ui.card} flex items-center gap-3 py-3`}>
                    <Link href={`/u/${m.user?.username}`} className="font-semibold hover:underline">{m.user?.username}</Link>
                    <span className="ml-auto flex gap-2">
                      <button disabled={busy} onClick={() => approve(m.user_id)} className={ui.primary}>Approve</button>
                      <button disabled={busy} onClick={() => kick(m.user_id)} className={ui.btn}>Reject</button>
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          )}

          {manager && (
            <section aria-labelledby="inv" className={`${ui.card} grid gap-3`}>
              <h2 id="inv" className="text-lg font-bold">Invite a classmate</h2>
              <form onSubmit={sendInvite} className="flex gap-2">
                <input className={ui.field} value={invite} onChange={(e) => setInvite(e.target.value)} placeholder="Username" aria-label="Username to invite" autoCapitalize="none" spellCheck={false} maxLength={20} />
                <button disabled={busy || !invite.trim()} className={ui.primary}>Invite</button>
              </form>
              {invited.length > 0 && <p className="text-xs text-dim">Waiting: {invited.map((m) => m.user?.username).join(", ")}</p>}
            </section>
          )}

          {myRole === "owner" && (
            <form onSubmit={save} className={`${ui.card} grid gap-3`}>
              <h2 className="text-lg font-bold">Club settings</h2>
              <label className="grid gap-1.5 text-sm font-medium">Name
                <input className={ui.field} value={edit.name} onChange={(e) => setEdit({ ...edit, name: e.target.value })} minLength={3} maxLength={40} required />
              </label>
              <label className="grid gap-1.5 text-sm font-medium">Description
                <textarea className={ui.field} rows={3} maxLength={400} value={edit.description} onChange={(e) => setEdit({ ...edit, description: e.target.value })} />
              </label>
              <label className="flex items-center gap-2 text-sm">
                <input type="checkbox" checked={edit.open} onChange={(e) => setEdit({ ...edit, open: e.target.checked })} /> Anyone can join
              </label>
              <span className="flex gap-2">
                <button disabled={busy} className={ui.primary}>Save</button>
                <button type="button" disabled={busy} className={ui.danger}
                  onClick={() => confirm("Archive this club? It disappears from the list.") && run(() => supabase.from("clubs").update({ archived: true }).eq("id", club.id))}>Archive club</button>
              </span>
            </form>
          )}
        </div>

        <aside aria-labelledby="mem" className="grid min-w-0 content-start gap-2">
          <h2 id="mem" className="text-lg font-bold">Members</h2>
          <ul className="grid gap-1">
            {active.map((m) => (
              <li key={m.user_id} className="flex flex-wrap items-center gap-2 text-sm">
                <Link href={`/u/${m.user?.username}`} className="font-semibold hover:underline">{m.user?.username}</Link>
                {m.role !== "member" && <span className="rounded bg-surface-2 px-1.5 text-xs text-brand">{m.role}</span>}
                {m.role !== "owner" && m.user_id !== viewerId && (myRole === "owner" || (myRole === "moderator" && m.role === "member")) && (
                  <span className="ml-auto flex gap-2 text-xs">
                    {myRole === "owner" && <button disabled={busy} className="text-dim hover:text-ink" onClick={() => setRole(m.user_id, m.role === "moderator" ? "member" : "moderator")}>{m.role === "moderator" ? "Demote" : "Make mod"}</button>}
                    <button disabled={busy} className="text-dim hover:text-danger" onClick={() => kick(m.user_id)}>Remove</button>
                  </span>
                )}
              </li>
            ))}
          </ul>
        </aside>
      </div>
    </div>
  );
}
