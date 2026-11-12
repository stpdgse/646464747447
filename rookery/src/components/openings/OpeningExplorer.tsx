"use client";

import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Board } from "@/components/chess/Board";
import { BoardSettingsPanel } from "@/components/chess/BoardSettingsPanel";
import { useBoardSettings } from "@/components/chess/settings";
import { needsPromotion, newRecord, toPgn, tryMove, tryUci, viewAt, type GameRecord, type PromotionRole } from "@/lib/chess/game";
import { plural } from "@/lib/ratings";
import { createClient } from "@/lib/supabase/client";

type Lib = typeof import("@/lib/openings");
type StatRow = { move: string; games: number; white_wins: number; draws: number; black_wins: number };

const btn = "rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-medium hover:bg-surface-2 disabled:opacity-40";

/** Explore opening theory (from the Lichess opening list) next to what your class actually played. */
export function OpeningExplorer() {
  const router = useRouter();
  const [settings, updateSettings] = useBoardSettings();
  const [record, setRecord] = useState<GameRecord>(() => newRecord("standard"));
  const [orientation, setOrientation] = useState<"white" | "black">("white");
  const [lib, setLib] = useState<Lib | null>(null);
  const [stats, setStats] = useState<{ key: string; rows: StatRow[] } | null>(null);
  const [statsFailed, setStatsFailed] = useState(false);
  const [showSettings, setShowSettings] = useState(false);

  useEffect(() => {
    let dead = false;
    import("@/lib/openings").then((m) => !dead && setLib(m));
    return () => { dead = true; };
  }, []);

  const key = record.moves.join(" ");
  useEffect(() => {
    let dead = false;
    createClient()
      .rpc("opening_stats", { p_prefix: key ? key.split(" ") : [] })
      .then(({ data, error }) => {
        if (dead) return;
        setStatsFailed(!!error);
        setStats({ key, rows: ((data ?? []) as StatRow[]).map((r) => ({ ...r, games: Number(r.games), white_wins: Number(r.white_wins), draws: Number(r.draws), black_wins: Number(r.black_wins) })) });
      });
    return () => { dead = true; };
  }, [key]);

  const view = useMemo(() => viewAt(record), [record]);
  const hit = lib ? lib.openingAt(record) : null;
  const book = lib ? lib.bookMoves(record) : [];
  const classRows = stats ? stats.rows : null; // keep the previous answer on screen while the next one loads
  const classStale = !!stats && stats.key !== key;
  const totalClass = classRows?.reduce((a, r) => a + r.games, 0) ?? 0;

  const san = useCallback((uci: string) => {
    const res = tryUci(record, uci);
    return res.ok ? viewAt(res.record).sans.at(-1) ?? uci : uci;
  }, [record]);

  const playUci = (uci: string) => {
    const res = tryUci(record, uci);
    if (res.ok) setRecord(res.record);
  };
  const onMove = useCallback((from: string, to: string, promotion?: PromotionRole) => {
    const res = tryMove(record, from as never, to as never, promotion);
    if (!res.ok) return false;
    setRecord(res.record);
    return true;
  }, [record]);
  const isPromotion = useCallback((from: string, to: string) => needsPromotion(record, from as never, to as never), [record]);

  const analyse = () => {
    try { window.sessionStorage.setItem("rookery.import", toPgn(record, { Event: "Opening explorer" })); } catch { /* ignore */ }
    router.push("/play/board");
  };

  const line = record.moves.length === 0 ? "Starting position" : view.sans.map((s, i) => (i % 2 === 0 ? `${i / 2 + 1}. ${s}` : s)).join(" ");

  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(0,560px)_minmax(0,1fr)]">
      <div className="grid min-w-0 grid-cols-1 content-start gap-3">
        <Board view={view} variant="standard" orientation={orientation} movable="both" settings={settings} isPromotion={isPromotion} onMove={onMove} />
        <div className="flex flex-wrap gap-2" role="toolbar" aria-label="Explorer controls">
          <button className={btn} onClick={() => setRecord({ ...record, moves: record.moves.slice(0, -1) })} disabled={record.moves.length === 0}>Back</button>
          <button className={btn} onClick={() => setRecord(newRecord("standard"))} disabled={record.moves.length === 0}>Reset</button>
          <button className={btn} onClick={() => setOrientation((o) => (o === "white" ? "black" : "white"))}>Flip</button>
          <button className={btn} onClick={analyse} disabled={record.moves.length === 0}>Analyse</button>
          <button className={btn} onClick={() => setShowSettings((s) => !s)} aria-expanded={showSettings}>Appearance</button>
        </div>
        {showSettings && <BoardSettingsPanel settings={settings} update={updateSettings} />}
      </div>

      <div className="grid min-w-0 grid-cols-1 content-start gap-6">
        <div className="rounded-xl border border-line bg-surface px-4 py-3" aria-live="polite">
          <p className="text-lg font-bold">{hit ? hit.opening.name : record.moves.length === 0 ? "Choose a first move" : lib ? "Out of book" : "Loading…"}</p>
          <p className="text-sm text-dim">{hit ? `${hit.opening.eco}${hit.ply < record.moves.length ? ` · last named ${record.moves.length - hit.ply} ${record.moves.length - hit.ply === 1 ? "move" : "moves"} ago` : ""} · ` : ""}{line}</p>
        </div>

        <section aria-labelledby="book" className="grid gap-2">
          <h2 id="book" className="text-xl font-bold">Opening book</h2>
          {!lib ? (
            <div className="h-24 animate-pulse rounded-xl bg-surface" />
          ) : book.length === 0 ? (
            <p className="text-sm text-dim">No named lines continue from here. You are out of book.</p>
          ) : (
            <div className="overflow-hidden rounded-xl border border-line bg-surface">
              <table className="w-full text-left text-sm">
                <thead className="bg-surface-2 text-dim"><tr><th className="px-3 py-2 font-medium">Move</th><th className="px-3 py-2 font-medium">Opening</th><th className="px-3 py-2 text-right font-medium">Lines</th></tr></thead>
                <tbody>
                  {book.slice(0, 12).map((m) => (
                    <tr key={m.uci} className="border-t border-line">
                      <td className="px-3 py-1.5"><button className="font-bold text-brand underline" onClick={() => playUci(m.uci)}>{san(m.uci)}</button></td>
                      <td className="px-3 py-1.5 text-dim">{m.reaches ? m.reaches.name : m.leadsTo ? `leads to ${m.leadsTo.name}` : ""}</td>
                      <td className="px-3 py-1.5 text-right tabular-nums text-dim">{m.lines}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>

        <section aria-labelledby="class" className="grid gap-2">
          <h2 id="class" className="text-xl font-bold">Your class</h2>
          {statsFailed ? (
            <p className="text-sm text-dim">Class statistics are unavailable right now.</p>
          ) : classRows === null ? (
            <div className="h-24 animate-pulse rounded-xl bg-surface" />
          ) : classRows.length === 0 ? (
            <p className="text-sm text-dim">Nobody has played on from this position yet. Finished standard games on the site show up here.</p>
          ) : (
            <div className={`overflow-hidden rounded-xl border border-line bg-surface transition-opacity ${classStale ? "opacity-50" : ""}`} aria-busy={classStale}>
              <table className="w-full text-left text-sm">
                <thead className="bg-surface-2 text-dim"><tr><th className="px-3 py-2 font-medium">Move</th><th className="px-3 py-2 font-medium">Games</th><th className="px-3 py-2 font-medium">Results</th></tr></thead>
                <tbody>
                  {classRows.map((r) => (
                    <tr key={r.move} className="border-t border-line">
                      <td className="px-3 py-1.5"><button className="font-bold text-brand underline" onClick={() => playUci(r.move)}>{san(r.move)}</button></td>
                      <td className="px-3 py-1.5 tabular-nums">{r.games} <span className="text-xs text-dim">({Math.round((r.games / totalClass) * 100)}%)</span></td>
                      <td className="px-3 py-1.5">
                        <div className="flex h-3 w-full min-w-24 overflow-hidden rounded-full" role="img"
                          aria-label={`${plural(r.white_wins, "win")} for White, ${plural(r.draws, "draw")}, ${plural(r.black_wins, "win")} for Black`}>
                          {[[r.white_wins, "#f1efe9"], [r.draws, "var(--ink-dim)"], [r.black_wins, "#4a4743"]].map(([n, c], i) => (n as number) > 0 && <div key={i} style={{ width: `${((n as number) / r.games) * 100}%`, background: c as string, marginLeft: i === 0 ? 0 : 2 }} />)}
                        </div>
                        <p className="mt-0.5 text-xs text-dim">{r.white_wins}-{r.draws}-{r.black_wins}</p>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              <p className="border-t border-line px-3 py-2 text-xs text-dim">Results are White wins, draws, Black wins, from finished standard games on this site.</p>
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
