"use client";

import { useCallback, useEffect, useState } from "react";
import { createClient } from "@/lib/supabase/client";
import { friendlyError, ui } from "@/lib/social";
import { ReportButton } from "./ReportButton";

type F = { id: number; requester_id: string; addressee_id: string; status: "pending" | "accepted" };

/** Friend / block / report buttons shown on another player's profile. */
export function FriendActions({ viewerId, targetId, targetName }: { viewerId: string | null; targetId: string; targetName: string }) {
  const [f, setF] = useState<F | null | undefined>(undefined);
  const [blocked, setBlocked] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    const supabase = createClient();
    const [fr, bl] = await Promise.all([
      supabase.from("friendships").select("id, requester_id, addressee_id, status")
        .or(`and(requester_id.eq.${viewerId},addressee_id.eq.${targetId}),and(requester_id.eq.${targetId},addressee_id.eq.${viewerId})`).maybeSingle(),
      supabase.from("blocks").select("blocked_id").eq("blocked_id", targetId).maybeSingle(),
    ]);
    setF((fr.data as F | null) ?? null);
    setBlocked(!!bl.data);
  }, [viewerId, targetId]);

  useEffect(() => {
    if (viewerId) queueMicrotask(load);
  }, [viewerId, load]);

  if (!viewerId || viewerId === targetId) return null;
  const supabase = createClient();

  const run = async (fn: () => PromiseLike<{ error: { message?: string; code?: string } | null }>) => {
    setBusy(true);
    setError(null);
    const { error: err } = await fn();
    setBusy(false);
    if (err) setError(friendlyError(err));
    await load();
  };

  const add = () => run(() => supabase.from("friendships").insert({ requester_id: viewerId, addressee_id: targetId }));
  const accept = () => run(() => supabase.from("friendships").update({ status: "accepted" }).eq("id", f!.id));
  const remove = () => run(() => supabase.from("friendships").delete().eq("id", f!.id));
  const block = () => run(async () => {
    const r = await supabase.from("blocks").insert({ blocker_id: viewerId, blocked_id: targetId });
    if (!r.error && f) await supabase.from("friendships").delete().eq("id", f.id);
    return r;
  });
  const unblock = () => run(() => supabase.from("blocks").delete().eq("blocked_id", targetId));

  return (
    <div className="flex flex-wrap items-center gap-2" aria-label={`Actions for ${targetName}`}>
      {f === undefined ? (
        <span className="h-8 w-28 animate-pulse rounded-md bg-surface-2" />
      ) : f === null ? (
        !blocked && <button disabled={busy} onClick={add} className={ui.primary}>Add friend</button>
      ) : f.status === "accepted" ? (
        <button disabled={busy} onClick={remove} className={ui.btn}>Friends ✓ · Remove</button>
      ) : f.requester_id === viewerId ? (
        <button disabled={busy} onClick={remove} className={ui.btn}>Request sent · Cancel</button>
      ) : (
        <>
          <button disabled={busy} onClick={accept} className={ui.primary}>Accept friend request</button>
          <button disabled={busy} onClick={remove} className={ui.btn}>Decline</button>
        </>
      )}
      {blocked ? (
        <button disabled={busy} onClick={unblock} className={ui.btn}>Unblock</button>
      ) : (
        <button disabled={busy} onClick={block} className="text-xs text-dim hover:text-ink">Block</button>
      )}
      <ReportButton viewerId={viewerId} kind="user" refId={targetId} targetUser={targetId} label="Report player" />
      {error && <p role="alert" className="w-full text-xs text-danger">{error}</p>}
    </div>
  );
}
