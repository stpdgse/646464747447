-- Bot tournaments (knockout or round-robin) that anyone can create and everyone can watch.
-- A browser runs the games (it has the chess engine). Only one browser "runs" a tournament at a time (a claim with a heartbeat);
-- if the runner leaves, any viewer can take over. Limits below keep this from being used to spam.

create table public.bot_tournaments (
  id             uuid primary key default gen_random_uuid(),
  creator_id     uuid not null references public.profiles (id) on delete cascade,
  name           text not null check (char_length(name) between 3 and 50),
  format         text not null check (format in ('knockout', 'roundrobin')),
  best_of        integer not null default 1 check (best_of in (1, 3)),
  variant        text not null default 'standard' check (variant in ('standard', 'chess960')),
  speed          text not null default 'fast' check (speed in ('fast', 'normal')),
  -- [{"id": "pawnie", "name": "Pawnie", "rating": 400}, ...] in entry order
  bots           jsonb not null,
  status         text not null default 'running' check (status in ('running', 'finished')),
  champion_id    text check (champion_id is null or char_length(champion_id) <= 32),
  runner_id      uuid references public.profiles (id) on delete set null,
  runner_beat_at timestamptz,
  created_at     timestamptz not null default now(),
  updated_at     timestamptz not null default now()
);
create index bot_tournaments_updated_idx on public.bot_tournaments (updated_at desc);
create index bot_tournaments_creator_idx on public.bot_tournaments (creator_id, updated_at desc);
create index bot_tournaments_runner_idx on public.bot_tournaments (runner_id);

create table public.bot_tournament_games (
  id             uuid primary key default gen_random_uuid(),
  tournament_id  uuid not null references public.bot_tournaments (id) on delete cascade,
  round          integer not null check (round between 1 and 30),
  slot           integer not null check (slot between 0 and 30),
  game_no        integer not null check (game_no between 1 and 6),
  white_bot_id   text not null check (char_length(white_bot_id) <= 32),
  black_bot_id   text not null check (char_length(black_bot_id) <= 32),
  initial_fen    text not null check (char_length(initial_fen) <= 100),
  moves          text[] not null default '{}' check (coalesce(array_length(moves, 1), 0) <= 1000),
  status         text not null default 'playing' check (status in ('playing', 'finished')),
  result         text check (result is null or result in ('1-0', '0-1', '1/2-1/2')),
  reason         text check (reason is null or char_length(reason) <= 24),
  created_at     timestamptz not null default now(),
  updated_at     timestamptz not null default now(),
  unique (tournament_id, round, slot, game_no)
);
create index bot_tournament_games_t_idx on public.bot_tournament_games (tournament_id, created_at);

-- ---------------------------------------------------------------------------------------------------------------------
-- Limits (the anti-spam part)

create function private.bot_tournament_limits() returns trigger language plpgsql security definer set search_path = '' as $$
declare n integer; ids text[];
begin
  if jsonb_typeof(new.bots) <> 'array' then raise exception 'bad_bots'; end if;
  n := jsonb_array_length(new.bots);
  if new.format = 'knockout' and n not in (4, 8, 16) then raise exception 'bad_size'; end if;
  if new.format = 'roundrobin' and (n < 3 or n > 8) then raise exception 'bad_size'; end if;
  select array_agg(e ->> 'id') into ids from jsonb_array_elements(new.bots) e;
  if (select count(distinct x) from unnest(ids) x) <> n or exists (select 1 from unnest(ids) x where x is null or char_length(x) > 32) then raise exception 'bad_bots'; end if;
  if exists (select 1 from jsonb_array_elements(new.bots) e
             where jsonb_typeof(e -> 'name') <> 'string' or char_length(e ->> 'name') > 40 or jsonb_typeof(e -> 'rating') <> 'number') then raise exception 'bad_bots'; end if;
  -- one tournament at a time per person, and a site-wide cap; anything idle for a day no longer counts
  if (select count(*) from public.bot_tournaments t where t.creator_id = new.creator_id and t.status = 'running' and t.updated_at > now() - interval '1 day') >= 1 then
    raise exception 'too_many_tournaments';
  end if;
  if (select count(*) from public.bot_tournaments t where t.status = 'running' and t.updated_at > now() - interval '1 day') >= 8 then
    raise exception 'site_busy';
  end if;
  return new;
end $$;
create trigger bot_tournaments_limits before insert on public.bot_tournaments for each row execute function private.bot_tournament_limits();
-- at most 5 tournaments per person per day
create trigger bot_tournaments_rate_limit before insert on public.bot_tournaments for each row execute function private.rate_limit('creator_id', '86400', '5');

create function private.bot_tournament_game_limits() returns trigger language plpgsql security definer set search_path = '' as $$
begin
  if (select count(*) from public.bot_tournament_games g where g.tournament_id = new.tournament_id) >= 130 then raise exception 'too_many_games'; end if;
  return new;
end $$;
create trigger bot_tournament_games_limits before insert on public.bot_tournament_games for each row execute function private.bot_tournament_game_limits();

create function private.bot_tournament_stamp() returns trigger language plpgsql security definer set search_path = '' as $$
begin
  new.updated_at := now();
  return new;
end $$;
create trigger bot_tournaments_stamp before insert or update on public.bot_tournaments for each row execute function private.bot_tournament_stamp();
create trigger bot_tournament_games_stamp before insert or update on public.bot_tournament_games for each row execute function private.bot_tournament_stamp();

-- a finished game keeps its tournament "alive" for the idle rule
create function private.bot_tournament_touch() returns trigger language plpgsql security definer set search_path = '' as $$
begin
  update public.bot_tournaments set updated_at = now() where id = new.tournament_id;
  return new;
end $$;
create trigger bot_tournament_games_touch after insert or update on public.bot_tournament_games for each row execute function private.bot_tournament_touch();

-- ---------------------------------------------------------------------------------------------------------------------
-- Access

alter table public.bot_tournaments enable row level security;
alter table public.bot_tournament_games enable row level security;
create policy "everyone can watch tournaments" on public.bot_tournaments for select using (true);
create policy "anyone can create a bot tournament" on public.bot_tournaments for insert to authenticated
  with check (creator_id = (select auth.uid()) and status = 'running' and public.can_post());
create policy "the runner reports the winner" on public.bot_tournaments for update to authenticated
  using (runner_id = (select auth.uid())) with check (runner_id = (select auth.uid()));
create policy "creator or moderators remove a tournament" on public.bot_tournaments for delete to authenticated
  using (creator_id = (select auth.uid()) or public.is_mod());

create policy "everyone can watch tournament games" on public.bot_tournament_games for select using (true);
create policy "the runner adds games" on public.bot_tournament_games for insert to authenticated
  with check (exists (select 1 from public.bot_tournaments t where t.id = tournament_id and t.runner_id = (select auth.uid()) and t.status = 'running'));
create policy "the runner updates games" on public.bot_tournament_games for update to authenticated
  using (exists (select 1 from public.bot_tournaments t where t.id = tournament_id and t.runner_id = (select auth.uid())))
  with check (exists (select 1 from public.bot_tournaments t where t.id = tournament_id and t.runner_id = (select auth.uid())));

revoke all on table public.bot_tournaments, public.bot_tournament_games from anon, authenticated;
grant select on table public.bot_tournaments, public.bot_tournament_games to anon, authenticated;
grant insert (creator_id, name, format, best_of, variant, speed, bots) on table public.bot_tournaments to authenticated;
grant update (status, champion_id) on table public.bot_tournaments to authenticated;
grant delete on table public.bot_tournaments to authenticated;
grant insert (tournament_id, round, slot, game_no, white_bot_id, black_bot_id, initial_fen, moves, status) on table public.bot_tournament_games to authenticated;
grant update (moves, status, result, reason) on table public.bot_tournament_games to authenticated;

-- ---------------------------------------------------------------------------------------------------------------------
-- Who runs it: a claim that only one browser can hold, kept alive by a heartbeat. A claim older than 25 seconds can be taken over.

create function public.claim_bot_tournament(p_id uuid) returns boolean language plpgsql security definer set search_path = '' as $$
declare uid uuid := private.require_user(); got uuid;
begin
  update public.bot_tournaments set runner_id = uid, runner_beat_at = now()
   where id = p_id and status = 'running' and (runner_id is null or runner_id = uid or runner_beat_at is null or runner_beat_at < now() - interval '25 seconds')
  returning id into got;
  return got is not null;
end $$;

create function public.beat_bot_tournament(p_id uuid) returns boolean language plpgsql security definer set search_path = '' as $$
declare uid uuid := private.require_user(); got uuid;
begin
  update public.bot_tournaments set runner_beat_at = now() where id = p_id and runner_id = uid and status = 'running' returning id into got;
  return got is not null;
end $$;

create function public.release_bot_tournament(p_id uuid) returns void language plpgsql security definer set search_path = '' as $$
declare uid uuid := private.require_user();
begin
  update public.bot_tournaments set runner_id = null, runner_beat_at = null where id = p_id and runner_id = uid;
end $$;

revoke execute on function public.claim_bot_tournament(uuid), public.beat_bot_tournament(uuid), public.release_bot_tournament(uuid) from public, anon, authenticated;
grant execute on function public.claim_bot_tournament(uuid), public.beat_bot_tournament(uuid), public.release_bot_tournament(uuid) to authenticated;

do $$
begin
  if exists (select 1 from pg_publication where pubname = 'supabase_realtime') then
    alter publication supabase_realtime add table public.bot_tournaments;
    alter publication supabase_realtime add table public.bot_tournament_games;
  end if;
end $$;
