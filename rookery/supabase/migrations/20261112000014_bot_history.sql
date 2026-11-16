-- Bot game history: every signed-in player's game against the computer is kept (not just the ones they let others watch).
-- Games marked private are visible only to their owner.

alter table public.bot_games add column watchable boolean not null default true;
alter table public.bot_games add column time_control text not null default 'unlimited' check (char_length(time_control) <= 12);
create index bot_games_player_updated_idx on public.bot_games (player_id, updated_at desc);

drop policy "anyone can watch bot games" on public.bot_games;
create policy "watch public bot games, see your own" on public.bot_games for select
  using (watchable or player_id = (select auth.uid()));

grant insert (watchable, time_control) on table public.bot_games to authenticated;
