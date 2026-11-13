"use client";

import { useEffect, useRef, useState } from "react";
import { rateMoveWithEngine, type RatedMove } from "@/lib/analysis/moveRating";
import type { GameRecord } from "@/lib/chess/game";
import { UciEngine, createWorkerTransport } from "@/lib/engine/uci";

export type MoveFeedback = RatedMove & { ply: number };

/**
 * After each of YOUR moves, a second Stockfish (in its own worker, so the bot is never slowed down) rates it:
 * Brilliant, Great, Best, Good, Inaccuracy, Mistake or Blunder. Only used against the computer.
 * Taking a move back removes its rating; the rating of a restored game starts again from the next move.
 */
export function useMoveFeedback(game: { id: string; record: GameRecord; player: "white" | "black" } | null, enabled: boolean) {
  const [byPly, setByPly] = useState<Record<number, MoveFeedback>>({});
  const [pending, setPending] = useState(false);
  const engine = useRef<UciEngine | null>(null);
  const chain = useRef<Promise<unknown>>(Promise.resolve());
  const done = useRef(new Set<number>());
  const gameId = useRef<string | null>(null);
  const generation = useRef(0);

  const id = game?.id ?? null;
  const moves = game?.record.moves ?? null;
  const player = game?.player ?? null;
  const plyCount = moves?.length ?? 0;

  // The engine only exists while feedback is on and a game is being played.
  useEffect(() => {
    if (enabled && id) return;
    engine.current?.terminate();
    engine.current = null;
  }, [enabled, id]);
  useEffect(() => () => {
    engine.current?.terminate();
    engine.current = null;
  }, []);

  useEffect(() => {
    if (!id || !moves || !player) {
      gameId.current = null;
      generation.current++;
      done.current.clear();
      queueMicrotask(() => {
        setByPly({});
        setPending(false);
      });
      return;
    }
    // new game, or moves taken back: forget ratings for plies that no longer exist
    if (gameId.current !== id) {
      gameId.current = id;
      generation.current++;
      done.current.clear();
      queueMicrotask(() => setByPly({}));
    } else {
      for (const p of [...done.current]) if (p > moves.length) done.current.delete(p);
      queueMicrotask(() => setByPly((cur) => (Object.keys(cur).some((k) => Number(k) > moves.length) ? Object.fromEntries(Object.entries(cur).filter(([k]) => Number(k) <= moves.length)) : cur)));
    }
    if (!enabled || moves.length === 0) return;
    const ply = moves.length;
    const moverIsPlayer = (ply % 2 === 1) === (player === "white"); // white plays odd plies
    if (!moverIsPlayer || done.current.has(ply)) return;
    done.current.add(ply);

    const gen = generation.current;
    const snapshot = { variant: game!.record.variant, initialFen: game!.record.initialFen };
    const before: GameRecord = { ...snapshot, moves: moves.slice(0, ply - 1) };
    const played = moves[ply - 1];
    setPending(true);
    chain.current = chain.current
      .then(async () => {
        if (gen !== generation.current) return;
        try {
          if (!engine.current) engine.current = new UciEngine(createWorkerTransport());
          const rated = await rateMoveWithEngine(engine.current, before, played);
          if (rated && gen === generation.current && done.current.has(ply)) setByPly((cur) => ({ ...cur, [ply]: { ...rated, ply } }));
        } catch {
          /* feedback is a bonus: if the engine cannot start, the game carries on without it */
        } finally {
          if (gen === generation.current) setPending(false);
        }
      })
      .catch(() => {});
    // `game` also carries clocks; only the move list matters here.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id, plyCount, player, enabled]);

  // The rating of the most recent move of yours that is on the board.
  let latest: MoveFeedback | null = null;
  for (const k of Object.keys(byPly)) {
    const f = byPly[Number(k)];
    if (f.ply <= plyCount && (!latest || f.ply > latest.ply)) latest = f;
  }
  return { latest, pending, byPly };
}
