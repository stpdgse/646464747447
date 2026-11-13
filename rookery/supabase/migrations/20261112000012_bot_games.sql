-- Watchable bot games: a signed-in player's browser publishes its game against the computer so classmates can spectate.
-- The game itself still runs in the player's browser; this is a read-only mirror, never used for ratings.

create table public.bot_games (
  id            uuid primary key,
  player_id     uuid not null references public.profiles (id) on delete cascade,
  bot_id        text not null check (char_length(bot_id) <= 32),
  bot_name      text not null check (char_length(bot_name) between 1 and 40),
  bot_rating    integer not null check (bot_rating between 0 and 5000),
  player_color  text not null check (player_color in ('white', 'black')),
  variant       text not null check (variant in ('standard', 'chess960')),
  initial_fen   text not null check (char_length(initial_fen) <= 100),
  moves         text[] not null default '{}' check (coalesce(array_length(moves, 1), 0) <= 1000),
  status        text not null default 'playing' check (status in ('playing', 'finished')),
  result        text check (result is null or result in ('1-0', '0-1', '1/2-1/2')),
  reason        text check (reason is null or char_length(reason) <= 20),
  created_at    timestamptz not null default now(),
  updated_at    timestamptz not null default now()
);
create index bot_games_updated_idx on public.bot_games (updated_at desc);
create index bot_games_player_idx on public.bot_games (player_id);

alter table public.bot_games enable row level security;
create policy "anyone can watch bot games" on public.bot_games for select using (true);
create policy "publish my own bot game" on public.bot_games for insert to authenticated
  with check (player_id = (select auth.uid()) and status = 'playing');
create policy "update my own bot game" on public.bot_games for update to authenticated
  using (player_id = (select auth.uid())) with check (player_id = (select auth.uid()));

revoke all on table public.bot_games from anon, authenticated;
grant select on table public.bot_games to anon, authenticated;
grant insert (id, player_id, bot_id, bot_name, bot_rating, player_color, variant, initial_fen, moves, status) on table public.bot_games to authenticated;
grant update (moves, status, result, reason) on table public.bot_games to authenticated;

-- Server time stamps every change (so "still being played" never depends on a school laptop's clock).
create function private.bot_game_stamp() returns trigger language plpgsql security definer set search_path = '' as $$
begin
  new.updated_at := now();
  return new;
end $$;
create trigger bot_games_stamp before insert or update on public.bot_games for each row execute function private.bot_game_stamp();
create trigger bot_games_rate_limit before insert on public.bot_games for each row execute function private.rate_limit('player_id', '3600', '40');

do $$
begin
  if exists (select 1 from pg_publication where pubname = 'supabase_realtime') then
    alter publication supabase_realtime add table public.bot_games;
  end if;
end $$;
