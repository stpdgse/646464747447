import type { GameRow, Participant } from "./botTournament";

/** Shared (server and browser) shape of a bot tournament as stored. Kept out of client components so server pages can import it. */
export type BotTournamentRow = {
  id: string; creator_id: string; name: string; format: "knockout" | "roundrobin"; best_of: 1 | 3; variant: "standard" | "chess960"; speed: "fast" | "normal";
  bots: Participant[]; status: "running" | "finished"; champion_id: string | null; runner_id: string | null; runner_beat_at: string | null;
  created_at: string; updated_at: string; creator: { username: string } | null; runner: { username: string } | null;
};
export const TOURNAMENT_SELECT = "id, creator_id, name, format, best_of, variant, speed, bots, status, champion_id, runner_id, runner_beat_at, created_at, updated_at, creator:profiles!bot_tournaments_creator_id_fkey(username), runner:profiles!bot_tournaments_runner_id_fkey(username)";

export type StoredGame = GameRow & { id: string; tournament_id: string; initial_fen: string; moves: string[]; reason: string | null; created_at: string; updated_at: string };
export const GAME_SELECT = "id, tournament_id, round, slot, game_no, white_bot_id, black_bot_id, initial_fen, moves, status, result, reason, created_at, updated_at";

/** A runner that has not sent a heartbeat for this long is considered gone, and any viewer may take over. */
export const RUNNER_TIMEOUT_MS = 25_000;
