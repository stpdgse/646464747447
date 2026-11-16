-- Bot battles: a player picks two bots and watches them play. The creator's browser runs both bots (it has the chess engine)
-- and publishes every move here; everyone else watches through the Watch tab. Nothing here affects ratings.

create table public.bot_matches (
  id                uuid primary key,
  creator_id        uuid not null references public.profiles (id) on delete cascade,
  white_bot_id      text not null check (char_length(white_bot_id) <= 32),
  white_bot_name    text not null check (char_length(white_bot_name) between 1 and 40),
  white_bot_rating  integer not null check (white_bot_rating between 0 and 5000),
  black_bot_id      text not null check (char_length(black_bot_id) <= 32),
  black_bot_name    text not null check (char_length(black_bot_name) between 1 and 40),
  black_bot_rating  integer not null check (black_bot_rating between 0 and 5000),
  variant           text not null check (variant in ('standard', 'chess960')),
  initial_fen       text not null check (char_length(initial_fen) <= 100),
  speed             text not null default 'normal' check (speed in ('fast', 'normal', 'slow')),
  moves             text[] not null default '{}' check (coalesce(array_length(moves, 1), 0) <= 1000),
  status            text not null default 'playing' check (status in ('playing', 'finished')),
  result            text check (result is null or result in ('1-0', '0-1', '1/2-1/2')),
  reason            text check (reason is null or char_length(reason) <= 24),
  created_at        timestamptz not null default now(),
  updated_at        timestamptz not null default now()
);
create index bot_matches_updated_idx on public.bot_matches (updated_at desc);
create index bot_matches_creator_idx on public.bot_matches (creator_id, updated_at desc);

alter table public.bot_matches enable row level security;
create policy "anyone can watch bot battles" on public.bot_matches for select using (true);
create policy "start my own bot battle" on public.bot_matches for insert to authenticated
  with check (creator_id = (select auth.uid()) and status = 'playing' and public.can_post());
create policy "update my own bot battle" on public.bot_matches for update to authenticated
  using (creator_id = (select auth.uid())) with check (creator_id = (select auth.uid()));

revoke all on table public.bot_matches from anon, authenticated;
grant select on table public.bot_matches to anon, authenticated;
grant insert (id, creator_id, white_bot_id, white_bot_name, white_bot_rating, black_bot_id, black_bot_name, black_bot_rating, variant, initial_fen, speed, moves, status)
  on table public.bot_matches to authenticated;
grant update (moves, status, result, reason, speed) on table public.bot_matches to authenticated;

create function private.bot_match_stamp() returns trigger language plpgsql security definer set search_path = '' as $$
begin
  new.updated_at := now();
  return new;
end $$;
create trigger bot_matches_stamp before insert or update on public.bot_matches for each row execute function private.bot_match_stamp();
create trigger bot_matches_rate_limit before insert on public.bot_matches for each row execute function private.rate_limit('creator_id', '3600', '30');

do $$
begin
  if exists (select 1 from pg_publication where pubname = 'supabase_realtime') then
    alter publication supabase_realtime add table public.bot_matches;
  end if;
end $$;
