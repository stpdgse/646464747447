"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { toPgn } from "@/lib/chess/game";
import { plural } from "@/lib/ratings";

export type BotHistoryRow = {
  id: string; bot_id: string; bot_name: string; bot_rating: number; player_color: "white" | "black"; variant: "standard" | "chess960";
  initial_fen: string; moves: string[]; status: "playing" | "finished"; result: string | null; reason: string | null; time_control: string;
  watchable: boolean; updated_at: string; abandoned: boolean;
};

const outcomeOf = (g: BotHistoryRow): "Win" | "Loss" | "Draw" | null =>
  !g.result ? null : g.result === "1/2-1/2" ? "Draw" : (g.result === "1-0") === (g.player_color === "white") ? "Win" : "Loss";

/** Games against the computer: record, hardest bot beaten, and one-click review with the engine. */
export function BotHistory({ rows, username, isOwner }: { rows: BotHistoryRow[]; username: string; isOwner: boolean }) {
  const router = useRouter();
  const finished = rows.filter((g) => g.status === "finished" && g.result);
  const wins = finished.filter((g) => outcomeOf(g) === "Win");
  const losses = finished.filter((g) => outcomeOf(g) === "Loss").length;
  const draws = finished.filter((g) => outcomeOf(g) === "Draw").length;
  const hardest = wins.reduce<BotHistoryRow | null>((best, g) => (!best || g.bot_rating > best.bot_rating ? g : best), null);

  const pgnOf = (g: BotHistoryRow) =>
    toPgn({ variant: g.variant, initialFen: g.initial_fen, moves: g.moves }, {
      Event: "Game against the computer", Site: "Rookery", Date: g.updated_at.slice(0, 10).replaceAll("-", "."),
      White: g.player_color === "white" ? username : g.bot_name, Black: g.player_color === "black" ? username : g.bot_name,
      Result: g.result ?? "*",
    });
  const review = (g: BotHistoryRow) => {
    try {
      window.sessionStorage.setItem("rookery.import", pgnOf(g));
      window.sessionStorage.setItem("rookery.review", "1");
    } catch {
      /* storage blocked: the analysis board opens empty */
    }
    router.push("/play/board");
  };
  const download = (g: BotHistoryRow) => {
    const a = document.createElement("a");
    a.href = URL.createObjectURL(new Blob([pgnOf(g)], { type: "application/x-chess-pgn" }));
    a.download = `${username}-vs-${g.bot_id}.pgn`;
    a.click();
    URL.revokeObjectURL(a.href);
  };

  if (rows.length === 0) return <p className="text-sm text-dim">No games against the computer yet.{isOwner ? " Games you play while logged in are saved here." : ""}</p>;
  return (
    <div className="grid gap-3">
      <p className="text-sm text-dim">
        <strong className="text-ink">{wins.length}</strong> won · <strong className="text-ink">{draws}</strong> drawn · <strong className="text-ink">{losses}</strong> lost
        {hardest && <> · hardest bot beaten: <strong className="text-ink">{hardest.bot_name}</strong> (≈{hardest.bot_rating})</>}
      </p>
      <ul className="grid gap-2">
        {rows.slice(0, 25).map((g) => {
          const outcome = outcomeOf(g);
          const label = outcome ?? (g.abandoned ? "Left" : "Playing");
          return (
            <li key={g.id} className="flex flex-wrap items-center gap-x-4 gap-y-2 rounded-xl border border-line bg-surface px-4 py-3">
              <span className={`w-14 text-sm font-bold ${outcome === "Win" ? "text-brand-hi" : outcome === "Loss" ? "text-danger" : "text-dim"}`}>{label}</span>
              <span className="min-w-0 flex-1">vs <strong>{g.bot_name}</strong> <span className="text-sm text-dim">(≈{g.bot_rating})</span>
                <span className="block text-xs text-dim">
                  {g.variant === "chess960" ? "Chess960 · " : ""}{g.time_control === "unlimited" ? "no clock" : g.time_control} · {plural(Math.ceil(g.moves.length / 2), "move")} · {g.updated_at.slice(0, 10)}{!g.watchable ? " · private" : ""}
                </span>
              </span>
              <span className="flex gap-2">
                {g.moves.length >= 2 && <button className="rounded-md border border-line bg-bg px-3 py-1.5 text-sm font-medium hover:bg-surface-2" onClick={() => review(g)}>Review</button>}
                {g.moves.length >= 1 && <button className="rounded-md border border-line bg-bg px-3 py-1.5 text-sm font-medium hover:bg-surface-2" onClick={() => download(g)}>PGN</button>}
                {isOwner && g.status === "playing" && !g.abandoned && <Link href="/play/computer" className="rounded-md bg-brand px-3 py-1.5 text-sm font-semibold text-brand-ink hover:bg-brand-hi">Continue</Link>}
              </span>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
