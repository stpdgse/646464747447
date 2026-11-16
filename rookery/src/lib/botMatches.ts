/** Shared (server and browser) description of a published bot battle. Kept out of the client component so the server page can use it. */
export type BotMatchRow = {
  id: string; white_bot_id: string; white_bot_name: string; white_bot_rating: number; black_bot_id: string; black_bot_name: string; black_bot_rating: number;
  variant: "standard" | "chess960"; initial_fen: string; moves: string[]; status: "playing" | "finished"; result: string | null; reason: string | null; speed: string;
  updated_at: string; creator: { username: string } | null;
};
export const MATCH_SELECT = "id, white_bot_id, white_bot_name, white_bot_rating, black_bot_id, black_bot_name, black_bot_rating, variant, initial_fen, moves, status, result, reason, speed, updated_at, creator:profiles(username)";
