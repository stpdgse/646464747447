"use client";

import { knockoutBracket, roundName, standings, type MatchState, type Participant, type Spec } from "@/lib/botTournament";
import type { Bot } from "@/lib/engine/bots";
import type { StoredGame } from "@/lib/botTournamentRows";

const Face = ({ p, bots }: { p: Participant | null; bots: Bot[] }) => {
  const bot = p ? bots.find((b) => b.id === p.id) : undefined;
  if (!p) return <span aria-hidden className="grid size-6 place-items-center rounded bg-surface-2 text-xs text-dim">?</span>;
  return bot?.avatarImage ? <img src={bot.avatarImage} alt="" width={24} height={24} className="size-6 rounded" /> : <span aria-hidden className="grid size-6 place-items-center rounded bg-surface-2 text-base">{bot?.avatar ?? "🤖"}</span>;
};

const fmt = (n: number) => (Number.isInteger(n) ? String(n) : n.toFixed(1));

function MatchCard({ m, games, bots, selected, onSelect, final }: { m: MatchState; games: StoredGame[]; bots: Bot[]; selected: string | null; onSelect: (id: string) => void; final: boolean }) {
  const row = (p: Participant | null, score: number) => {
    const won = !!m.winner && p && m.winner.id === p.id;
    const lost = !!m.winner && p && m.winner.id !== p.id;
    return (
      <div className={`flex items-center gap-2 px-2 py-1.5 ${won ? "bg-brand/15 font-bold" : ""} ${lost ? "text-dim" : ""}`}>
        <Face p={p} bots={bots} />
        <span className="min-w-0 flex-1 truncate text-sm">{p ? p.name : "To be decided"}</span>
        {p && m.b && m.a && <span className="tabular-nums text-sm">{fmt(score)}</span>}
        {won && final && <span aria-label="Champion">🏆</span>}
      </div>
    );
  };
  return (
    <div className={`overflow-hidden rounded-lg border bg-surface ${m.live ? "border-brand shadow-[0_0_0_1px_var(--brand)]" : "border-line"}`}>
      {row(m.a, m.aScore)}
      <div className="border-t border-line" />
      {row(m.b, m.bScore)}
      {(games.length > 0 || m.live) && (
        <div className="flex flex-wrap items-center gap-1 border-t border-line bg-bg px-2 py-1">
          {m.live && <span className="rounded bg-brand px-1.5 text-[10px] font-extrabold uppercase text-brand-ink">Live</span>}
          {games.map((g) => (
            <button key={g.id} onClick={() => onSelect(g.id)} aria-pressed={selected === g.id} title={g.status === "finished" ? `Game ${g.game_no}: ${g.result}` : `Game ${g.game_no} in progress`}
              className={`rounded px-1.5 py-0.5 text-[11px] font-semibold ${selected === g.id ? "bg-brand text-brand-ink" : "bg-surface-2 hover:bg-line"}`}>
              G{g.game_no}{g.status === "finished" ? ` ${g.result}` : "…"}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

/** The knockout bracket, or the league table for a round-robin. Clicking a game chip puts that game on the board. */
export function BotBracket({ spec, games, bots, selected, onSelect }: { spec: Spec; games: StoredGame[]; bots: Bot[]; selected: string | null; onSelect: (id: string) => void }) {
  if (spec.format === "roundrobin") {
    const table = standings(spec.bots, games);
    const playedGames = games.filter((g) => g.status === "finished").length;
    const total = spec.bots.length * (spec.bots.length - 1);
    return (
      <section aria-label="League table" className="grid gap-3">
        <p className="text-sm text-dim">{playedGames} of {total} games played · win 1, draw ½</p>
        <div className="overflow-x-auto rounded-xl border border-line">
          <table className="w-full min-w-[30rem] text-sm">
            <thead className="bg-surface-2 text-left text-xs uppercase text-dim">
              <tr><th className="px-3 py-2">#</th><th className="px-3 py-2">Bot</th><th className="px-3 py-2 text-right">Pts</th><th className="px-3 py-2 text-right">Played</th><th className="px-3 py-2 text-right">W / D / L</th></tr>
            </thead>
            <tbody>
              {table.map((r, i) => (
                <tr key={r.bot.id} className="border-t border-line">
                  <td className="px-3 py-2 tabular-nums">{i + 1}</td>
                  <td className="px-3 py-2 font-semibold"><span className="mr-2 inline-block align-middle"><Face p={r.bot} bots={bots} /></span>{r.bot.name} <span className="font-normal text-dim">≈{r.bot.rating}</span></td>
                  <td className="px-3 py-2 text-right font-bold tabular-nums">{fmt(r.points)}</td>
                  <td className="px-3 py-2 text-right tabular-nums">{r.played}</td>
                  <td className="px-3 py-2 text-right tabular-nums">{r.wins} / {r.draws} / {r.losses}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="flex flex-wrap gap-1.5" aria-label="Games">
          {games.map((g) => {
            const w = spec.bots.find((b) => b.id === g.white_bot_id)?.name ?? g.white_bot_id, b = spec.bots.find((x) => x.id === g.black_bot_id)?.name ?? g.black_bot_id;
            return (
              <button key={g.id} onClick={() => onSelect(g.id)} aria-pressed={selected === g.id} className={`rounded-md border px-2 py-1 text-xs ${selected === g.id ? "border-brand bg-brand/15" : "border-line bg-surface hover:bg-surface-2"}`}>
                {w} vs {b} {g.status === "finished" ? g.result : "…"}
              </button>
            );
          })}
        </div>
      </section>
    );
  }
  const bracket = knockoutBracket(spec, games);
  return (
    <section aria-label="Bracket" className="overflow-x-auto pb-2">
      <div className="flex min-w-max items-stretch gap-6">
        {bracket.map((matches, r) => (
          <div key={r} className="flex w-60 flex-col">
            <h3 className="mb-2 text-center text-xs font-bold uppercase tracking-wide text-dim">{roundName(r + 1, bracket.length)}</h3>
            <div className="flex flex-1 flex-col justify-around gap-3">
              {matches.map((m, s) => (
                <MatchCard key={s} m={m} games={games.filter((g) => g.round === r + 1 && g.slot === s)} bots={bots} selected={selected} onSelect={onSelect} final={r === bracket.length - 1} />
              ))}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}
