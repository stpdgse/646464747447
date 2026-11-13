-- Arena tournaments: a time window, players join, press "Play" to be paired with another waiting player, and every finished
-- game scores points (win 2, draw 1, loss 0). Standings are the points. Games are ordinary live games tagged with the tournament.

create table public.tournaments (
  id               uuid primary key default gen_random_uuid(),
  name             text not null check (char_length(name) between 3 and 60),
  description      text not null default '' check (char_length(description) <= 400),
  created_by       uuid not null references public.profiles (id) on delete cascade,
  variant          text not null check (variant in ('standard', 'chess960')),
  time_initial_ms  integer not null check (time_initial_ms between 15000 and 1800000),
  time_inc_ms      integer not null default 0 check (time_inc_ms between 0 and 60000),
  rated            boolean not null default false,
  starts_at        timestamptz not null,
  ends_at          timestamptz not null,
  created_at       timestamptz not null default now(),
  constraint tournaments_window check (ends_at > starts_at + interval '5 minutes' and ends_at <= starts_at + interval '6 hours')
);
create index tournaments_time_idx on public.tournaments (starts_at desc);
create index tournaments_creator_idx on public.tournaments (created_by);

create table public.tournament_players (
  tournament_id  uuid not null references public.tournaments (id) on delete cascade,
  user_id        uuid not null references public.profiles (id) on delete cascade,
  points         integer not null default 0,
  games          integer not null default 0,
  wins           integer not null default 0,
  draws          integer not null default 0,
  losses         integer not null default 0,
  waiting_since  timestamptz,
  /** false once the player has left (standings keep anyone who has played) */
  active         boolean not null default true,
  last_opponent  uuid references public.profiles (id) on delete set null,
  joined_at      timestamptz not null default now(),
  last_game_at   timestamptz,
  primary key (tournament_id, user_id)
);
create index tournament_players_user_idx on public.tournament_players (user_id);
create index tournament_players_standings_idx on public.tournament_players (tournament_id, points desc, wins desc);

alter table public.games add column tournament_id uuid references public.tournaments (id) on delete set null;
create index games_tournament_idx on public.games (tournament_id) where tournament_id is not null;

alter table public.tournaments enable row level security;
alter table public.tournament_players enable row level security;
create policy "tournaments are public" on public.tournaments for select using (true);
create policy "moderators create tournaments" on public.tournaments for insert to authenticated
  with check (created_by = (select auth.uid()) and public.is_mod() and starts_at > now() - interval '10 minutes');
create policy "creator or moderators cancel a tournament" on public.tournaments for delete to authenticated
  using (created_by = (select auth.uid()) or public.is_mod());
create policy "standings are public" on public.tournament_players for select using (true);

revoke all on table public.tournaments, public.tournament_players from anon, authenticated;
grant select on table public.tournaments, public.tournament_players to anon, authenticated;
grant insert (name, description, created_by, variant, time_initial_ms, time_inc_ms, rated, starts_at, ends_at) on table public.tournaments to authenticated;
grant delete on table public.tournaments to authenticated;

create trigger tournaments_rate_limit before insert on public.tournaments for each row execute function private.rate_limit('created_by', '86400', '8');

-- ---------------------------------------------------------------------------------------------------------------------
-- Joining and leaving (no secret needed: nothing here decides a game)

create function public.tournament_join(p_tournament uuid) returns void
language plpgsql security definer set search_path = '' as $$
declare uid uuid := private.require_user(); t public.tournaments;
begin
  select * into t from public.tournaments where id = p_tournament;
  if not found then raise exception 'not_found'; end if;
  if now() >= t.ends_at then raise exception 'ended'; end if;
  insert into public.tournament_players (tournament_id, user_id) values (p_tournament, uid)
  on conflict (tournament_id, user_id) do update set active = true;
end $$;

create function public.tournament_leave(p_tournament uuid) returns void
language plpgsql security definer set search_path = '' as $$
declare uid uuid := private.require_user();
begin
  -- Standings keep anyone who has played; leaving just takes you out of the queue and the player list.
  update public.tournament_players set active = false, waiting_since = null where tournament_id = p_tournament and user_id = uid;
end $$;

create function public.tournament_stop_waiting(p_tournament uuid) returns void
language plpgsql security definer set search_path = '' as $$
declare uid uuid := private.require_user();
begin
  update public.tournament_players set waiting_since = null where tournament_id = p_tournament and user_id = uid;
end $$;

-- ---------------------------------------------------------------------------------------------------------------------
-- Pairing. Called again every few seconds while a player waits; that call both keeps them in the queue and tries to pair them.

create function public.server_tournament_play(p_secret text, p_tournament uuid, p_initial_fen text) returns jsonb
language plpgsql security definer set search_path = '' as $$
declare uid uuid; t public.tournaments; me public.tournament_players; opp uuid; g public.games; gid uuid; gt uuid; creator_white boolean;
begin
  perform private.check_secret(p_secret);
  uid := private.require_user();
  perform pg_advisory_xact_lock(hashtext('tournament:' || p_tournament::text));
  select * into t from public.tournaments where id = p_tournament;
  if not found then raise exception 'not_found'; end if;
  if now() < t.starts_at then raise exception 'not_started'; end if;
  if now() >= t.ends_at then raise exception 'ended'; end if;
  select * into me from public.tournament_players where tournament_id = p_tournament and user_id = uid and active;
  if not found then raise exception 'not_joined'; end if;

  -- A game already running: go there (this is also how the player who was paired by someone else finds their game).
  select x.id, x.tournament_id into gid, gt from public.games x
   where x.status = 'started' and x.mode = 'live' and (x.white_id = uid or x.black_id = uid) limit 1;
  if found then
    if gt is distinct from p_tournament then raise exception 'busy'; end if;
    return jsonb_build_object('game_id', gid);
  end if;

  -- Another player who is waiting (asked within the last 25 s) and free; prefer someone other than the last opponent.
  select tp.user_id into opp from public.tournament_players tp
   where tp.tournament_id = p_tournament and tp.user_id <> uid and tp.active and tp.waiting_since > now() - interval '25 seconds'
     and not exists (select 1 from public.games y where y.status = 'started' and y.mode = 'live' and (y.white_id = tp.user_id or y.black_id = tp.user_id))
   order by (tp.user_id is not distinct from me.last_opponent), tp.waiting_since limit 1;

  if opp is null then
    update public.tournament_players set waiting_since = now() where tournament_id = p_tournament and user_id = uid;
    return jsonb_build_object('waiting', true);
  end if;

  creator_white := random() < 0.5;
  insert into public.games (variant, initial_fen, white_id, black_id, time_initial_ms, time_inc_ms, white_clock_ms, black_clock_ms, rated, mode, tournament_id)
  values (t.variant, p_initial_fen, case when creator_white then uid else opp end, case when creator_white then opp else uid end,
          t.time_initial_ms, t.time_inc_ms, t.time_initial_ms, t.time_initial_ms, t.rated, 'live', p_tournament)
  returning * into g;
  update public.tournament_players set waiting_since = null,
         last_opponent = case when user_id = uid then opp else uid end
   where tournament_id = p_tournament and user_id in (uid, opp);
  return jsonb_build_object('game_id', g.id);
end $$;

-- ---------------------------------------------------------------------------------------------------------------------
-- Scoring: when a tournament game ends. A game needs at least two moves each to score (so nobody can farm points by resigning at once).

create function private.tournament_game_done() returns trigger language plpgsql security definer set search_path = '' as $$
declare w int; b int; scored boolean;
begin
  if new.tournament_id is null or old.status <> 'started' or new.status = 'started' then return new; end if;
  scored := new.status = 'finished' and cardinality(new.moves) >= 4;
  w := case when not scored then 0 when new.result = '1-0' then 2 when new.result = '1/2-1/2' then 1 else 0 end;
  b := case when not scored then 0 when new.result = '0-1' then 2 when new.result = '1/2-1/2' then 1 else 0 end;
  update public.tournament_players set
    points = points + case when user_id = new.white_id then w else b end,
    games = games + case when scored then 1 else 0 end,
    wins = wins + case when scored and ((user_id = new.white_id and new.result = '1-0') or (user_id = new.black_id and new.result = '0-1')) then 1 else 0 end,
    draws = draws + case when scored and new.result = '1/2-1/2' then 1 else 0 end,
    losses = losses + case when scored and ((user_id = new.white_id and new.result = '0-1') or (user_id = new.black_id and new.result = '1-0')) then 1 else 0 end,
    last_game_at = now(), waiting_since = null
  where tournament_id = new.tournament_id and user_id in (new.white_id, new.black_id);
  return new;
end $$;
create trigger games_tournament_done after update of status on public.games for each row execute function private.tournament_game_done();

revoke execute on function public.tournament_join(uuid), public.tournament_leave(uuid), public.tournament_stop_waiting(uuid),
  public.server_tournament_play(text, uuid, text) from public, anon, authenticated;
grant execute on function public.tournament_join(uuid), public.tournament_leave(uuid), public.tournament_stop_waiting(uuid),
  public.server_tournament_play(text, uuid, text) to authenticated;

do $$
begin
  if exists (select 1 from pg_publication where pubname = 'supabase_realtime') then
    alter publication supabase_realtime add table public.tournament_players;
  end if;
end $$;
