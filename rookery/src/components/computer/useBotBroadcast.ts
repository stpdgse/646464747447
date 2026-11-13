"use client";

import { useEffect, useRef, useState } from "react";
import { createClient } from "@/lib/supabase/client";

export type Broadcast = {
  id: string;
  botId: string;
  botName: string;
  botRating: number;
  player: "white" | "black";
  variant: "standard" | "chess960";
  initialFen: string;
  moves: string[];
  /** Set once the game is over. */
  end: { result: string; how: string } | null;
};

/**
 * Mirrors a game against the computer to the `bot_games` table so classmates can spectate it.
 * The game itself keeps running in the player's browser; if anything here fails, the player never notices.
 * Writes are chained so a fast sequence of moves always lands in order.
 */
export function useBotBroadcast(game: Broadcast | null, enabled: boolean) {
  const [uid, setUid] = useState<string | null>(null);
  // Bumped when the connection returns, so moves played offline are published right away.
  const [resync, setResync] = useState(0);
  const created = useRef(new Set<string>());
  const finished = useRef(new Set<string>());
  const chain = useRef<Promise<unknown>>(Promise.resolve());
  const lastId = useRef<string | null>(null);

  useEffect(() => {
    createClient().auth.getSession().then(({ data }) => setUid(data.session?.user.id ?? null)).catch(() => {});
    const onOnline = () => setResync((n) => n + 1);
    window.addEventListener("online", onOnline);
    return () => window.removeEventListener("online", onOnline);
  }, []);

  const id = game?.id ?? null;
  const movesKey = game ? game.moves.join(" ") : "";
  const endKey = game?.end ? `${game.end.result}:${game.end.how}` : "";

  useEffect(() => {
    const prev = lastId.current;
    lastId.current = id;
    if (!uid) return;
    const supabase = createClient();
    const enqueue = (work: () => PromiseLike<unknown>) => {
      chain.current = chain.current.then(() => work()).catch(() => {});
    };
    // The player moved on (new opponent): close the previous game so it leaves the "live" list.
    if (prev && prev !== id && created.current.has(prev) && !finished.current.has(prev)) {
      finished.current.add(prev);
      enqueue(() => supabase.from("bot_games").update({ status: "finished" }).eq("id", prev));
    }
    if (!game || !id || !enabled) return;

    enqueue(async () => {
      if (!created.current.has(id)) {
        created.current.add(id);
        const { error } = await supabase.from("bot_games").insert({
          id, player_id: uid, bot_id: game.botId, bot_name: game.botName, bot_rating: game.botRating,
          player_color: game.player, variant: game.variant, initial_fen: game.initialFen, moves: game.moves, status: "playing",
        });
        // 23505 = already published (a restored game): carry on and update it.
        if (error && error.code !== "23505") {
          created.current.delete(id);
          return;
        }
      }
      if (game.end) finished.current.add(id);
      await supabase.from("bot_games").update({
        moves: game.moves,
        status: game.end ? "finished" : "playing",
        result: game.end?.result ?? null,
        reason: game.end?.how ?? null,
      }).eq("id", id);
    });
    // `game` itself changes every tick (clocks); only moves and the result matter here.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [uid, enabled, id, movesKey, endKey, resync]);
}
