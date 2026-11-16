"use client";

import { useMemo, useState } from "react";
import { Board } from "@/components/chess/Board";
import { BoardSettingsPanel } from "@/components/chess/BoardSettingsPanel";
import { ClockBar } from "@/components/chess/ClockBar";
import { MoveList } from "@/components/chess/MoveList";
import { ScreenshotKey } from "@/components/chess/ScreenshotKey";
import { useBoardSettings } from "@/components/chess/settings";
import { viewAt, type GameRecord } from "@/lib/chess/game";
import type { Bot } from "@/lib/engine/bots";

export type Side = { id: string; name: string; rating: number };
const btn = "rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-medium hover:bg-surface-2 disabled:opacity-40";

/** Board, both bots and the move list for a bot battle. Used by the host and by spectators. */
export function BotMatchBoard({ record, white, black, bots, status, controls }: { record: GameRecord; white: Side; black: Side; bots: Bot[]; status: React.ReactNode; controls?: React.ReactNode }) {
  const [settings, updateSettings] = useBoardSettings();
  const [flipped, setFlipped] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const view = useMemo(() => {
    try {
      return viewAt(record);
    } catch {
      return null;
    }
  }, [record]);
  if (!view) return <p className="text-dim">This game can&apos;t be shown.</p>;
  const orientation = flipped ? "black" : "white";
  const top = flipped ? white : black;
  const bottom = flipped ? black : white;
  const topSide = flipped ? "white" : "black";
  const av = (s: Side) => bots.find((b) => b.id === s.id);
  const label = (s: Side, side: string) => `${s.name} (${side}, ≈${s.rating})`;
  const bar = (s: Side, side: "white" | "black") => (
    <ClockBar label={label(s, side)} avatar={av(s)?.avatar} avatarImage={av(s)?.avatarImage} ms={0} active={!view.outcome && view.turn === side} timed={false} />
  );
  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(0,640px)_minmax(0,1fr)]">
      <div className="grid min-w-0 grid-cols-1 content-start gap-3">
        {bar(top, topSide)}
        <Board view={view} variant={record.variant} orientation={orientation} movable="none" settings={settings} isPromotion={() => false} onMove={() => false} />
        {bar(bottom, topSide === "white" ? "black" : "white")}
        <div className="flex flex-wrap gap-2">
          <button className={btn} onClick={() => setFlipped((f) => !f)}>Flip</button>
          <button className={btn} onClick={() => setShowSettings((s) => !s)} aria-expanded={showSettings}>Appearance</button>
        </div>
        {showSettings && <BoardSettingsPanel settings={settings} update={updateSettings} />}
      </div>
      <div className="grid min-w-0 grid-cols-1 content-start gap-4">
        <div aria-live="polite" className="rounded-xl border border-line bg-surface px-4 py-3">{status}</div>
        {controls}
        <section className="rounded-xl border border-line bg-surface" aria-label="Move list">
          <MoveList sans={view.sans} ply={view.sans.length} />
        </section>
        <ScreenshotKey />
      </div>
    </div>
  );
}
