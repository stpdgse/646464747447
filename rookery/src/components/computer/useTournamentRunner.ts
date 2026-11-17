"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { champion, nextGame, type Spec } from "@/lib/botTournament";
import { GAME_SELECT, type BotTournamentRow, type StoredGame } from "@/lib/botTournamentRows";
import { newRecord, tryUci, viewAt, type GameRecord } from "@/lib/chess/game";
import { pickBotMove, SPEEDS } from "@/lib/engine/botMove";
import type { Bot } from "@/lib/engine/bots";
import type { UciEngine } from "@/lib/engine/uci";
import { createClient } from "@/lib/supabase/client";

const PLY_LIMIT = 300;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export type Runner = { running: boolean; lost: boolean; error: string | null; liveMoves: Record<string, string[]> };

/**
 * Plays the whole tournament in this browser: asks `nextGame` what is next, makes the two bots play it, and publishes every move.
 * A heartbeat keeps the claim; if it is lost (another viewer took over because this tab went quiet), the loop stops cleanly.
 * Started games are resumed from their saved moves, which is also how a new runner picks up where the last one stopped.
 */
export function useTournamentRunner({ tournament, bots, engine, active, onGames }: {
  tournament: BotTournamentRow; bots: Bot[]; engine: UciEngine | null; active: boolean; onGames: (games: StoredGame[]) => void;
}) {
  const [state, setState] = useState<Runner>({ running: false, lost: false, error: null, liveMoves: {} });
  const token = useRef(0);
  const tid = tournament.id;
  const spec: Spec = { format: tournament.format, bestOf: tournament.best_of, bots: tournament.bots };
  const specRef = useRef(spec);
  useEffect(() => {
    specRef.current = spec;
  });

  const stop = useCallback(() => {
    token.current++;
    setState((s) => ({ ...s, running: false }));
  }, []);

  useEffect(() => {
    if (!active || !engine) return;
    const my = ++token.current;
    const supabase = createClient();
    const alive = () => my === token.current;
    queueMicrotask(() => setState({ running: true, lost: false, error: null, liveMoves: {} }));

    const lose = () => {
      if (!alive()) return;
      token.current++;
      setState((s) => ({ ...s, running: false, lost: true }));
    };
    const beat = setInterval(async () => {
      const { data } = await supabase.rpc("beat_bot_tournament", { p_id: tid });
      if (data === false) lose();
    }, 5000);

    (async () => {
      engine.newGame();
      while (alive()) {
        // Always start from what the database says (a previous runner may have played games).
        const { data } = await supabase.from("bot_tournament_games").select(GAME_SELECT).eq("tournament_id", tid).order("created_at", { ascending: true });
        if (!alive()) return;
        let games = (data ?? []) as StoredGame[];
        onGames(games);

        const plan = nextGame(specRef.current, games);
        if (!plan) {
          const champ = champion(specRef.current, games);
          await supabase.from("bot_tournaments").update({ status: "finished", champion_id: champ?.id ?? null }).eq("id", tid);
          if (alive()) setState((s) => ({ ...s, running: false }));
          return;
        }

        let row: StoredGame;
        let record: GameRecord;
        if (plan.kind === "resume") {
          row = plan.game as StoredGame;
          record = { variant: tournament.variant, initialFen: row.initial_fen, moves: row.moves ?? [] };
        } else {
          record = newRecord(tournament.variant);
          const { data: inserted, error } = await supabase.from("bot_tournament_games").insert({
            tournament_id: tid, round: plan.round, slot: plan.slot, game_no: plan.gameNo, white_bot_id: plan.whiteId, black_bot_id: plan.blackId,
            initial_fen: record.initialFen, moves: [], status: "playing",
          }).select(GAME_SELECT).single();
          if (error || !inserted) {
            if (error?.code === "23505") continue; // someone else already added it: reload and look again
            if (alive()) setState((s) => ({ ...s, running: false, error: error?.message.includes("row-level security") ? "Another browser has taken over this tournament." : "Could not save the game." }));
            return;
          }
          row = inserted as StoredGame;
          games = [...games, row];
          onGames(games);
        }
        const white = bots.find((b) => b.id === row.white_bot_id), black = bots.find((b) => b.id === row.black_bot_id);
        if (!white || !black) {
          if (alive()) setState((s) => ({ ...s, running: false, error: "One of the bots is no longer available." }));
          return;
        }
        const sp = SPEEDS[tournament.speed];

        // play the game
        let result: string | null = null, reason = "";
        while (alive()) {
          const v = viewAt(record);
          if (v.outcome) { result = v.outcome.result; reason = v.outcome.reason; break; }
          if (record.moves.length >= PLY_LIMIT) { result = "1/2-1/2"; reason = "move limit"; break; }
          const bot = v.turn === "white" ? white : black;
          const began = performance.now();
          const uci = await pickBotMove(engine, bot, record, { movetimeCap: sp.capMs });
          if (!alive()) return;
          if (!uci) { await sleep(300); continue; }
          const wait = Math.max(0, sp.pauseMs - (performance.now() - began));
          if (wait > 0) await sleep(wait);
          if (!alive()) return;
          const next = tryUci(record, uci);
          if (!next.ok) { await sleep(300); continue; }
          record = next.record;
          setState((s) => ({ ...s, liveMoves: { ...s.liveMoves, [row.id]: record.moves } }));
          // publish without waiting, so the engine is never held up by the network
          void supabase.from("bot_tournament_games").update({ moves: record.moves }).eq("id", row.id).then(({ error }) => { if (error) lose(); });
        }
        if (!alive() || !result) return;
        const { error: endErr } = await supabase.from("bot_tournament_games").update({ moves: record.moves, status: "finished", result, reason }).eq("id", row.id);
        if (endErr) { lose(); return; }
        await sleep(1500); // a breath between games so spectators can see the result
      }
    })().catch(() => {
      if (alive()) setState((s) => ({ ...s, running: false, error: "The tournament stopped unexpectedly. Press the button to continue it." }));
    });

    return () => {
      clearInterval(beat);
      token.current++;
    };
    // The loop restarts only when we start or stop running or the engine becomes available.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, engine, tid]);

  return { ...state, stop };
}
