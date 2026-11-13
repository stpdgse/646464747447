"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Board } from "@/components/chess/Board";
import { BoardSettingsPanel } from "@/components/chess/BoardSettingsPanel";
import { ClockBar } from "@/components/chess/ClockBar";
import { MoveList } from "@/components/chess/MoveList";
import { ScreenshotKey } from "@/components/chess/ScreenshotKey";
import { useBoardSettings } from "@/components/chess/settings";
import { viewAt } from "@/lib/chess/game";
import { useBots } from "@/components/computer/useBots";
import { createClient } from "@/lib/supabase/client";

export type BotGameRow = {
  id: string; bot_id: string; bot_name: string; bot_rating: number; player_color: "white" | "black"; variant: "standard" | "chess960";
  initial_fen: string; moves: string[]; status: "playing" | "finished"; result: string | null; reason: string | null; updated_at: string;
  player: { username: string } | null;
};

const SELECT = "id, bot_id, bot_name, bot_rating, player_color, variant, initial_fen, moves, status, result, reason, updated_at, player:profiles(username)";
const btn = "rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-medium hover:bg-surface-2";

/** Read-only view of someone's game against the computer, updated live. */
export function BotSpectator({ initial }: { initial: BotGameRow }) {
  const [row, setRow] = useState(initial);
  const [settings, updateSettings] = useBoardSettings();
  const [flipped, setFlipped] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [now, setNow] = useState(0);
  const allBots = useBots();

  const load = useCallback(async () => {
    setNow(Date.now());
    const { data } = await createClient().from("bot_games").select(SELECT).eq("id", initial.id).maybeSingle();
    if (data) setRow((cur) => ((data as unknown as BotGameRow).moves.length >= cur.moves.length || (data as unknown as BotGameRow).status !== cur.status ? (data as unknown as BotGameRow) : cur));
  }, [initial.id]);

  useEffect(() => {
    queueMicrotask(load);
    const supabase = createClient();
    const channel = supabase
      .channel(`botgame-${initial.id}`)
      .on("postgres_changes", { event: "UPDATE", schema: "public", table: "bot_games", filter: `id=eq.${initial.id}` }, () => load())
      .subscribe();
    const timer = setInterval(() => document.visibilityState === "visible" && load(), 3000);
    return () => {
      clearInterval(timer);
      supabase.removeChannel(channel);
    };
  }, [initial.id, load]);

  const view = useMemo(() => {
    try {
      return viewAt({ variant: row.variant, initialFen: row.initial_fen, moves: row.moves });
    } catch {
      return null;
    }
  }, [row.variant, row.initial_fen, row.moves]);

  if (!view) return <p className="text-dim">This game can&apos;t be shown.</p>;

  const bot = allBots.find((b) => b.id === row.bot_id);
  const playerName = row.player?.username ?? "Player";
  const orientation = flipped ? (row.player_color === "white" ? "black" : "white") : row.player_color;
  const botSide = row.player_color === "white" ? "black" : "white";
  const top = orientation === "white" ? "black" : "white";
  const label = (side: "white" | "black") => (side === botSide ? `${row.bot_name} (bot)` : playerName);
  const playing = row.status === "playing";
  const stale = playing && now > 0 && now - new Date(row.updated_at).getTime() > 10 * 60_000;

  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(0,640px)_minmax(0,1fr)]">
      <div className="grid min-w-0 grid-cols-1 content-start gap-3">
        <ClockBar label={label(top)} avatar={top === botSide ? bot?.avatar : undefined} avatarImage={top === botSide ? bot?.avatarImage : undefined} ms={0} active={playing && view.turn === top} timed={false} />
        <Board view={view} variant={row.variant} orientation={orientation} movable="none" settings={settings} isPromotion={() => false} onMove={() => false} />
        <ClockBar label={label(top === "white" ? "black" : "white")} avatar={top === "white" ? undefined : bot?.avatar} avatarImage={top === "white" ? undefined : bot?.avatarImage} ms={0} active={playing && view.turn !== top} timed={false} />
        <div className="flex flex-wrap gap-2">
          <button className={btn} onClick={() => setFlipped((f) => !f)}>Flip</button>
          <button className={btn} onClick={() => setShowSettings((s) => !s)} aria-expanded={showSettings}>Appearance</button>
          <Link href="/watch" className={`${btn} inline-block`}>Back to Watch</Link>
        </div>
        {showSettings && <BoardSettingsPanel settings={settings} update={updateSettings} />}
      </div>
      <div className="grid min-w-0 grid-cols-1 content-start gap-4">
        <div aria-live="polite" className={`rounded-xl border px-4 py-3 ${playing && !stale ? "border-line bg-surface" : "border-brand bg-brand/15"}`}>
          <p className="font-semibold">
            {playing ? (stale ? "This game looks abandoned" : `${view.turn === botSide ? row.bot_name : playerName} to move${view.check ? " (check)" : ""}`) : `Finished${row.result ? `: ${row.result}` : ""}${row.reason ? ` (${row.reason})` : ""}`}
          </p>
          <p className="text-xs text-dim">{playerName} vs {row.bot_name} (≈{row.bot_rating}){row.variant === "chess960" ? " · Chess960" : ""} · you are watching</p>
        </div>
        <section className="rounded-xl border border-line bg-surface" aria-label="Move list">
          <MoveList sans={view.sans} ply={view.sans.length} />
        </section>
        <ScreenshotKey />
      </div>
    </div>
  );
}
