"use server";

import { z } from "zod";
import { requireProfile } from "@/lib/auth";
import { startFen, type Variant } from "@/lib/chess/variants";
import { createClient } from "@/lib/supabase/server";

export type TournamentPlay = { ok: true; gameId: string } | { ok: true; waiting: true } | { ok: false; error: string };

const MESSAGES: Record<string, string> = {
  not_started: "This tournament hasn't started yet.",
  ended: "This tournament is over.",
  not_joined: "Join the tournament first.",
  busy: "You are already in another game. Finish it first.",
  not_found: "Tournament not found.",
  banned: "Your account is suspended.",
  forbidden: "Live games aren't switched on yet.",
};

/**
 * "Play": join the queue or get paired. Called again every few seconds while waiting, which keeps the player in the queue
 * and picks up a game that someone else started for them.
 */
export async function playTournament(tournamentId: string): Promise<TournamentPlay> {
  await requireProfile();
  const secret = process.env.GAME_SERVER_SECRET;
  if (!secret) return { ok: false, error: "Live games are not switched on yet (server secret missing)." };
  if (!z.string().uuid().safeParse(tournamentId).success) return { ok: false, error: MESSAGES.not_found };
  const supabase = await createClient();
  const { data: t } = await supabase.from("tournaments").select("variant").eq("id", tournamentId).maybeSingle();
  if (!t) return { ok: false, error: MESSAGES.not_found };
  const { data, error } = await supabase.rpc("server_tournament_play", {
    p_secret: secret, p_tournament: tournamentId, p_initial_fen: startFen((t as { variant: Variant }).variant),
  });
  if (error) {
    const key = Object.keys(MESSAGES).find((k) => error.message.includes(k));
    return { ok: false, error: key ? MESSAGES[key] : "Something went wrong. Please try again." };
  }
  const out = data as { game_id?: string; waiting?: boolean };
  if (out.game_id) return { ok: true, gameId: out.game_id };
  return { ok: true, waiting: true };
}
