"use client";

import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import { BotMatchBoard } from "@/components/computer/BotMatchBoard";
import { useBots } from "@/components/computer/useBots";
import { MATCH_SELECT, type BotMatchRow } from "@/lib/botMatches";
import { createClient } from "@/lib/supabase/client";

/** Read-only live view of a bot battle. */
export function BotMatchSpectator({ initial }: { initial: BotMatchRow }) {
  const bots = useBots();
  const [row, setRow] = useState(initial);
  const [now, setNow] = useState(0);

  const load = useCallback(async () => {
    setNow(Date.now());
    const { data } = await createClient().from("bot_matches").select(MATCH_SELECT).eq("id", initial.id).maybeSingle();
    const next = data as unknown as BotMatchRow | null;
    if (next) setRow((cur) => (next.moves.length >= cur.moves.length || next.status !== cur.status ? next : cur));
  }, [initial.id]);

  useEffect(() => {
    queueMicrotask(load);
    const supabase = createClient();
    const channel = supabase.channel(`botmatch-${initial.id}`).on("postgres_changes", { event: "UPDATE", schema: "public", table: "bot_matches", filter: `id=eq.${initial.id}` }, () => load()).subscribe();
    const timer = setInterval(() => document.visibilityState === "visible" && load(), 2500);
    return () => {
      clearInterval(timer);
      supabase.removeChannel(channel);
    };
  }, [initial.id, load]);

  const stale = row.status === "playing" && now > 0 && now - new Date(row.updated_at).getTime() > 25_000;
  const outcome = row.status === "finished" ? (row.result === "1-0" ? `${row.white_bot_name} wins` : row.result === "0-1" ? `${row.black_bot_name} wins` : row.result === "1/2-1/2" ? "Draw" : "Stopped by the host") : null;
  return (
    <div className="grid gap-3">
      <p className="text-sm"><Link href="/watch" className="text-dim hover:text-ink">← Watch</Link></p>
      <BotMatchBoard
        record={{ variant: row.variant, initialFen: row.initial_fen, moves: row.moves }}
        white={{ id: row.white_bot_id, name: row.white_bot_name, rating: row.white_bot_rating }}
        black={{ id: row.black_bot_id, name: row.black_bot_name, rating: row.black_bot_rating }}
        bots={bots}
        status={
          outcome ? (
            <div className="grid gap-1"><p className="text-lg font-bold">{outcome}</p><p className="text-sm text-dim">{row.reason ? `(${row.reason})` : ""} {row.white_bot_name} vs {row.black_bot_name}</p></div>
          ) : stale ? (
            <div className="grid gap-1"><p className="font-semibold">The host paused or left</p><p className="text-sm text-dim">The battle continues when they come back to the Bot battles page.</p></div>
          ) : (
            <div className="grid gap-1"><p className="font-semibold">Live: {row.white_bot_name} vs {row.black_bot_name}</p><p className="text-xs text-dim">hosted by {row.creator?.username ?? "someone"} · {row.speed} speed · you are watching</p></div>
          )
        }
        controls={<Link href="/play/bots" className="justify-self-start rounded-md bg-brand px-4 py-2 text-sm font-semibold text-brand-ink hover:bg-brand-hi">Start your own bot battle</Link>}
      />
    </div>
  );
}
