import { toPgn } from "@/lib/chess/game";
import { recordOf, type GameRow } from "@/lib/live/types";
import { createClient } from "@/lib/supabase/server";
import { usernameSchema } from "@/lib/validation";

/** Every finished game of a player as one downloadable PGN file. */
export async function GET(_req: Request, ctx: RouteContext<"/u/[username]/pgn">) {
  const { username } = await ctx.params;
  if (!usernameSchema.safeParse(username).success) return new Response("Not found", { status: 404 });

  const supabase = await createClient();
  const { data: profile } = await supabase.from("profiles").select("id, username").eq("username", username).maybeSingle();
  if (!profile) return new Response("Not found", { status: 404 });

  const { data } = await supabase
    .from("games")
    .select("*, white:profiles!games_white_id_fkey(username), black:profiles!games_black_id_fkey(username)")
    .eq("status", "finished")
    .or(`white_id.eq.${profile.id},black_id.eq.${profile.id}`)
    .order("ended_at", { ascending: false })
    .limit(300);

  const games = (data ?? []) as unknown as (GameRow & { white: { username: string }; black: { username: string } })[];
  const pgn = games
    .map((g) =>
      toPgn(recordOf(g), {
        Event: `Rookery ${g.rated ? "rated" : "casual"} game`,
        Site: "Rookery",
        Date: g.created_at.slice(0, 10).replaceAll("-", "."),
        White: g.white.username,
        Black: g.black.username,
        Result: g.result ?? "*",
        WhiteElo: g.white_rating ? String(g.white_rating) : undefined,
        BlackElo: g.black_rating ? String(g.black_rating) : undefined,
        TimeControl: `${g.time_initial_ms / 1000}+${g.time_inc_ms / 1000}`,
        Termination: g.reason ?? undefined,
      }),
    )
    .join("\n\n");

  return new Response(pgn || "", {
    headers: {
      "Content-Type": "application/x-chess-pgn; charset=utf-8",
      "Content-Disposition": `attachment; filename="${profile.username}-rookery.pgn"`,
    },
  });
}
