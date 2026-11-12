-- Rookery M6 + M7: daily (correspondence) games, in-app notifications, puzzle ratings and Puzzle Rush.

-- =============================================================================================================
-- Daily games: same tables as live games, with a per-move deadline (in days) instead of a running clock.

alter table public.games add column mode text not null default 'live' check (mode in ('live', 'daily'));
alter table public.games add column days_per_move integer check (days_per_move between 1 and 14);
alter table public.games add column move_deadline timestamptz;
alter table public.games drop constraint games_time_initial_ms_check;
alter table public.games add constraint games_time_check check (
  (mode = 'live' and time_initial_ms between 15000 and 10800000 and days_per_move is null)
  or (mode = 'daily' and time_initial_ms = 0 and time_inc_ms = 0 and days_per_move is not null)
);
create index games_daily_deadline_idx on public.games (move_deadline) where status = 'started' and mode = 'daily';

alter table public.seeks add column mode text not null default 'live' check (mode in ('live', 'daily'));
alter table public.seeks add column days_per_move integer check (days_per_move between 1 and 14);
alter table public.seeks drop constraint seeks_time_initial_ms_check;
alter table public.seeks add constraint seeks_time_check check (
  (mode = 'live' and time_initial_ms between 15000 and 10800000 and days_per_move is null)
  or (mode = 'daily' and time_initial_ms = 0 and time_inc_ms = 0 and days_per_move is not null)
);
drop index public.seeks_one_open_per_user;
drop index public.seeks_one_challenge_per_pair;
create unique index seeks_one_open_per_user_mode on public.seeks (user_id, mode) where target_id is null;
create unique index seeks_one_challenge_per_pair_mode on public.seeks (user_id, target_id, mode) where target_id is not null;

-- Daily offers stay open for three days.
create function private.seek_defaults()
returns trigger language plpgsql set search_path = '' as $$
begin
  if new.mode = 'daily' then new.expires_at := now() + interval '3 days'; end if;
  return new;
end $$;
create trigger seeks_defaults before insert on public.seeks for each row execute function private.seek_defaults();

-- =============================================================================================================
-- Notifications (in-app)

create table public.notifications (
  id          bigint generated always as identity primary key,
  user_id     uuid not null references public.profiles (id) on delete cascade,
  kind        text not null check (kind in ('your_turn', 'game_over')),
  game_id     uuid references public.games (id) on delete cascade,
  created_at  timestamptz not null default now(),
  read_at     timestamptz
);
create index notifications_user_idx on public.notifications (user_id, created_at desc);
alter table public.notifications enable row level security;
create policy "read own notifications" on public.notifications for select to authenticated using (user_id = (select auth.uid()));
create policy "mark own notifications read" on public.notifications for update to authenticated
  using (user_id = (select auth.uid())) with check (user_id = (select auth.uid()));
revoke all on table public.notifications from anon, authenticated;
grant select on table public.notifications to authenticated;
grant update (read_at) on table public.notifications to authenticated;

do $$
begin
  if exists (select 1 from pg_publication where pubname = 'supabase_realtime') then
    alter publication supabase_realtime add table public.notifications;
  end if;
end $$;

create function private.notify(p_user uuid, p_kind text, p_game uuid)
returns void language sql security definer set search_path = '' as $$
  insert into public.notifications (user_id, kind, game_id) values (p_user, p_kind, p_game)
$$;

-- =============================================================================================================
-- Ratings: daily games and puzzles get their own rating

alter table public.ratings drop constraint ratings_perf_check;
alter table public.ratings add constraint ratings_perf_check
  check (perf in ('bullet', 'blitz', 'rapid', 'daily', 'chess960', 'kingOfTheHill', 'threeCheck', 'puzzle'));

create function private.perf_of(p_variant text, p_ms integer, p_inc integer, p_mode text)
returns text language sql immutable set search_path = '' as $$
  select case
    when p_mode = 'daily' then 'daily'
    when p_variant <> 'standard' then p_variant
    when p_ms / 1000.0 + 40 * (p_inc / 1000.0) < 180 then 'bullet'
    when p_ms / 1000.0 + 40 * (p_inc / 1000.0) < 480 then 'blitz'
    else 'rapid'
  end
$$;

create or replace function private.snapshot_ratings()
returns trigger language plpgsql security definer set search_path = '' as $$
declare v_perf text := private.perf_of(new.variant, new.time_initial_ms, new.time_inc_ms, new.mode);
begin
  new.white_rating := coalesce((select round(r.rating)::integer from public.ratings r where r.user_id = new.white_id and r.perf = v_perf), 1500);
  new.black_rating := coalesce((select round(r.rating)::integer from public.ratings r where r.user_id = new.black_id and r.perf = v_perf), 1500);
  return new;
end $$;

create or replace function private.apply_ratings(g public.games)
returns void language plpgsql security definer set search_path = '' as $$
declare
  v_perf text; w public.ratings; b public.ratings; ws double precision; wn record; bn record;
begin
  if not g.rated or g.status <> 'finished' or g.result is null then return; end if;
  if exists (select 1 from public.rating_history h where h.game_id = g.id) then return; end if;
  v_perf := private.perf_of(g.variant, g.time_initial_ms, g.time_inc_ms, g.mode);

  insert into public.ratings (user_id, perf) values (g.white_id, v_perf), (g.black_id, v_perf) on conflict do nothing;
  perform 1 from public.ratings r where r.perf = v_perf and r.user_id in (g.white_id, g.black_id) order by r.user_id for update;
  select * into w from public.ratings r where r.user_id = g.white_id and r.perf = v_perf;
  select * into b from public.ratings r where r.user_id = g.black_id and r.perf = v_perf;

  ws := case g.result when '1-0' then 1 when '0-1' then 0 else 0.5 end;
  select * into wn from private.glicko2(w.rating, w.rd, w.vol, b.rating, b.rd, ws);
  select * into bn from private.glicko2(b.rating, b.rd, b.vol, w.rating, w.rd, 1 - ws);

  update public.ratings set rating = wn.new_r, rd = wn.new_rd, vol = wn.new_vol, games = games + 1, updated_at = now()
   where user_id = g.white_id and perf = v_perf;
  update public.ratings set rating = bn.new_r, rd = bn.new_rd, vol = bn.new_vol, games = games + 1, updated_at = now()
   where user_id = g.black_id and perf = v_perf;
  insert into public.rating_history (user_id, perf, game_id, rating_before, rating_after, rd_after) values
    (g.white_id, v_perf, g.id, round(w.rating), round(wn.new_r), wn.new_rd),
    (g.black_id, v_perf, g.id, round(b.rating), round(bn.new_r), bn.new_rd);
end $$;

drop function private.perf_of(text, integer, integer);

-- finish(): ratings, and a "game over" notification for the other player of a daily game.
create or replace function private.finish(p_id uuid, p_result text, p_reason text, p_white integer default null, p_black integer default null)
returns public.games language plpgsql security definer set search_path = '' as $$
declare g public.games;
begin
  update public.games
     set status = 'finished', result = p_result, reason = p_reason, ended_at = now(), draw_offer_by = null,
         white_clock_ms = coalesce(p_white, white_clock_ms), black_clock_ms = coalesce(p_black, black_clock_ms)
   where id = p_id and status = 'started'
  returning * into g;
  if not found then
    select * into g from public.games where id = p_id;
    return g;
  end if;
  perform private.apply_ratings(g);
  if g.mode = 'daily' then
    insert into public.notifications (user_id, kind, game_id)
    select u, 'game_over', g.id from unnest(array[g.white_id, g.black_id]) as u where u is distinct from auth.uid();
  end if;
  return g;
end $$;

-- =============================================================================================================
-- Pairing: live and daily offers are kept apart.

create or replace function private.start_game(s public.seeks, p_joiner uuid, p_fen text)
returns public.games language plpgsql security definer set search_path = '' as $$
declare creator_white boolean; g public.games;
begin
  if s.mode = 'live' and exists (
    select 1 from public.games x
    where x.status = 'started' and x.mode = 'live' and (x.white_id in (p_joiner, s.user_id) or x.black_id in (p_joiner, s.user_id))
  ) then
    raise exception 'busy';
  end if;
  if s.mode = 'daily' and (
    select count(*) from public.games x
    where x.status = 'started' and x.mode = 'daily' and (x.white_id in (p_joiner, s.user_id) or x.black_id in (p_joiner, s.user_id))
  ) >= 40 then
    raise exception 'too_many_daily';
  end if;
  creator_white := case s.color when 'white' then true when 'black' then false else random() < 0.5 end;
  insert into public.games (variant, initial_fen, white_id, black_id, time_initial_ms, time_inc_ms, white_clock_ms, black_clock_ms, rated,
                            mode, days_per_move, move_deadline)
  values (s.variant, p_fen,
          case when creator_white then s.user_id else p_joiner end,
          case when creator_white then p_joiner else s.user_id end,
          s.time_initial_ms, s.time_inc_ms, s.time_initial_ms, s.time_initial_ms, s.rated,
          s.mode, s.days_per_move, case when s.mode = 'daily' then now() + make_interval(days => s.days_per_move) end)
  returning * into g;
  delete from public.seeks where id = s.id or (s.mode = 'live' and mode = 'live' and target_id is null and user_id in (p_joiner, s.user_id));
  if g.mode = 'daily' then perform private.notify(case when creator_white then s.user_id else p_joiner end, 'your_turn', g.id); end if;
  return g;
end $$;

create or replace function public.server_quick_pair(p_secret text, p_variant text, p_time_ms integer, p_inc_ms integer, p_initial_fen text, p_rated boolean default false)
returns jsonb language plpgsql security definer set search_path = '' as $$
declare uid uuid; s public.seeks; g public.games; mine public.seeks;
begin
  perform private.check_secret(p_secret);
  uid := private.require_user();
  perform pg_advisory_xact_lock(hashtext(p_variant || ':' || p_time_ms || ':' || p_inc_ms || ':' || p_rated));

  select * into s from public.seeks x
  where x.mode = 'live' and x.target_id is null and x.user_id <> uid and x.variant = p_variant and x.rated = p_rated
    and x.time_initial_ms = p_time_ms and x.time_inc_ms = p_inc_ms and x.expires_at > now()
    and not exists (select 1 from public.games y where y.status = 'started' and y.mode = 'live' and (y.white_id = x.user_id or y.black_id = x.user_id))
  order by x.created_at limit 1 for update skip locked;

  if found then
    g := private.start_game(s, uid, p_initial_fen);
    return jsonb_build_object('game', to_jsonb(g));
  end if;

  if exists (select 1 from public.games y where y.status = 'started' and y.mode = 'live' and (y.white_id = uid or y.black_id = uid)) then
    raise exception 'busy';
  end if;
  delete from public.seeks where user_id = uid and target_id is null and mode = 'live';
  insert into public.seeks (user_id, variant, time_initial_ms, time_inc_ms, rated)
  values (uid, p_variant, p_time_ms, p_inc_ms, p_rated) returning * into mine;
  return jsonb_build_object('seek', to_jsonb(mine));
end $$;

-- =============================================================================================================
-- Moves and timeouts: daily games use the deadline instead of a clock.

create or replace function public.server_move(
  p_secret text, p_game_id uuid, p_uci text, p_expected_ply integer, p_result text default null, p_reason text default null
) returns public.games language plpgsql security definer set search_path = '' as $$
declare g public.games; uid uuid; side text; ply integer; left_ms bigint; new_clock integer; mover_clock integer; opp uuid;
begin
  perform private.check_secret(p_secret);
  g := private.lock_game(p_game_id); uid := auth.uid(); side := private.side_of(g);
  if side is null then raise exception 'not_a_player' using errcode = '42501'; end if;
  if g.status <> 'started' then raise exception 'finished'; end if;
  ply := cardinality(g.moves);
  if ply <> p_expected_ply then raise exception 'stale'; end if;
  if (ply % 2 = 0) <> (side = 'white') then raise exception 'not_your_turn'; end if;
  if p_uci !~ '^[a-h][1-8][a-h][1-8][qrbn]?$' then raise exception 'bad_move'; end if;
  if p_result is not null and p_result not in ('1-0', '0-1', '1/2-1/2') then raise exception 'bad_result'; end if;

  mover_clock := case side when 'white' then g.white_clock_ms else g.black_clock_ms end;
  new_clock := mover_clock;
  if g.mode = 'daily' then
    if now() > g.move_deadline then
      if ply < 2 then
        update public.games set status = 'aborted', reason = 'abandoned', ended_at = now(), draw_offer_by = null where id = g.id returning * into g;
        return g;
      end if;
      return private.finish(g.id, case side when 'white' then '0-1' else '1-0' end, 'timeout');
    end if;
  elsif ply >= 2 then
    left_ms := mover_clock - private.ms_since(g.clock_updated_at);
    if left_ms <= 0 then
      return private.finish(g.id, case side when 'white' then '0-1' else '1-0' end, 'timeout',
                            case side when 'white' then 0 end, case side when 'black' then 0 end);
    end if;
    new_clock := (left_ms + g.time_inc_ms)::integer;
  end if;

  update public.games set
    moves = moves || p_uci,
    white_clock_ms = case side when 'white' then new_clock else white_clock_ms end,
    black_clock_ms = case side when 'black' then new_clock else black_clock_ms end,
    clock_updated_at = clock_timestamp(),
    draw_offer_by = null,
    move_deadline = case when g.mode = 'daily' then now() + make_interval(days => g.days_per_move) end
  where id = g.id returning * into g;

  if p_result is not null then
    return private.finish(g.id, p_result, coalesce(p_reason, 'checkmate'));
  end if;
  if g.mode = 'daily' then
    opp := case side when 'white' then g.black_id else g.white_id end;
    perform private.notify(opp, 'your_turn', g.id);
  end if;
  return g;
end $$;

create or replace function public.server_claim_timeout(p_secret text, p_game_id uuid, p_other_cannot_mate boolean default false)
returns public.games language plpgsql security definer set search_path = '' as $$
declare g public.games; uid uuid; side text; ply integer; turn_white boolean; left_ms bigint; loser text;
begin
  perform private.check_secret(p_secret);
  g := private.lock_game(p_game_id); uid := auth.uid(); side := private.side_of(g);
  if g.status <> 'started' then return g; end if;
  ply := cardinality(g.moves);
  turn_white := (ply % 2 = 0);
  loser := case when turn_white then 'white' else 'black' end;

  if g.mode = 'daily' then
    if now() <= g.move_deadline then return g; end if;
    if ply < 2 then
      update public.games set status = 'aborted', reason = 'abandoned', ended_at = now(), draw_offer_by = null where id = g.id returning * into g;
      return g;
    end if;
    return private.finish(g.id, case when p_other_cannot_mate then '1/2-1/2' when loser = 'white' then '0-1' else '1-0' end, 'timeout');
  end if;

  if ply < 2 then
    if private.ms_since(g.clock_updated_at) > 60000 then
      update public.games set status = 'aborted', reason = 'abandoned', ended_at = now(), draw_offer_by = null where id = g.id returning * into g;
    end if;
    return g;
  end if;
  left_ms := (case when turn_white then g.white_clock_ms else g.black_clock_ms end) - private.ms_since(g.clock_updated_at);
  if left_ms > 0 then return g; end if;
  return private.finish(g.id,
    case when p_other_cannot_mate then '1/2-1/2' when loser = 'white' then '0-1' else '1-0' end, 'timeout',
    case when loser = 'white' then 0 end, case when loser = 'black' then 0 end);
end $$;

/** Background sweep: end daily games whose deadline has passed, even if nobody is looking. Runs every 10 minutes. */
create function private.sweep_daily()
returns integer language plpgsql security definer set search_path = '' as $$
declare g public.games; n integer := 0;
begin
  for g in select * from public.games where status = 'started' and mode = 'daily' and move_deadline < now() order by move_deadline limit 200 for update skip locked loop
    if cardinality(g.moves) < 2 then
      update public.games set status = 'aborted', reason = 'abandoned', ended_at = now(), draw_offer_by = null where id = g.id;
    else
      perform private.finish(g.id, case when cardinality(g.moves) % 2 = 0 then '0-1' else '1-0' end, 'timeout');
    end if;
    n := n + 1;
  end loop;
  return n;
end $$;

do $$
begin
  if exists (select 1 from pg_available_extensions where name = 'pg_cron') then
    begin
      create extension if not exists pg_cron;
      perform cron.schedule('rookery-daily-sweep', '*/10 * * * *', 'select private.sweep_daily()');
    exception when others then
      raise notice 'pg_cron not scheduled: %', sqlerrm;
    end;
  end if;
end $$;

-- =============================================================================================================
-- Puzzles

create table public.puzzle_attempts (
  id               bigint generated always as identity primary key,
  user_id          uuid not null references public.profiles (id) on delete cascade,
  puzzle_id        text not null check (char_length(puzzle_id) between 1 and 16),
  puzzle_rating    integer not null,
  solved           boolean not null,
  rating_before    integer not null,
  rating_after     integer not null,
  created_at       timestamptz not null default now(),
  unique (user_id, puzzle_id)
);
create index puzzle_attempts_user_idx on public.puzzle_attempts (user_id, created_at);

create table public.rush_scores (
  id          bigint generated always as identity primary key,
  user_id     uuid not null references public.profiles (id) on delete cascade,
  score       integer not null check (score between 0 and 200),
  created_at  timestamptz not null default now()
);
create index rush_scores_top_idx on public.rush_scores (score desc, created_at);
create index rush_scores_user_idx on public.rush_scores (user_id, score desc);

create table private.rush_sessions (
  id          uuid primary key default gen_random_uuid(),
  user_id     uuid not null,
  started_at  timestamptz not null default clock_timestamp(),
  finished    boolean not null default false
);

alter table public.puzzle_attempts enable row level security;
alter table public.rush_scores enable row level security;
create policy "puzzle attempts are public" on public.puzzle_attempts for select using (true);
create policy "rush scores are public" on public.rush_scores for select using (true);
revoke all on table public.puzzle_attempts from anon, authenticated;
revoke all on table public.rush_scores from anon, authenticated;
grant select on table public.puzzle_attempts to anon, authenticated;
grant select on table public.rush_scores to anon, authenticated;

/** Record a rated puzzle result. The puzzle's rating comes from the server's own puzzle set, never from the client. */
create function public.server_record_puzzle(p_secret text, p_puzzle_id text, p_puzzle_rating integer, p_puzzle_rd integer, p_solved boolean)
returns jsonb language plpgsql security definer set search_path = '' as $$
declare uid uuid; r public.ratings; nr record; prev_id bigint;
begin
  perform private.check_secret(p_secret);
  uid := private.require_user();
  if p_puzzle_rating not between 200 and 3500 or p_puzzle_rd not between 20 and 500 then raise exception 'bad_puzzle'; end if;

  insert into public.ratings (user_id, perf) values (uid, 'puzzle') on conflict do nothing;
  select * into r from public.ratings x where x.user_id = uid and x.perf = 'puzzle' for update;
  select a.id into prev_id from public.puzzle_attempts a where a.user_id = uid and a.puzzle_id = p_puzzle_id;
  if prev_id is not null then
    return jsonb_build_object('repeat', true, 'rating', round(r.rating)::integer);
  end if;

  select * into nr from private.glicko2(r.rating, r.rd, r.vol, p_puzzle_rating, p_puzzle_rd, case when p_solved then 1 else 0 end);
  update public.ratings set rating = nr.new_r, rd = nr.new_rd, vol = nr.new_vol, games = games + 1, updated_at = now()
   where user_id = uid and perf = 'puzzle';
  insert into public.puzzle_attempts (user_id, puzzle_id, puzzle_rating, solved, rating_before, rating_after)
  values (uid, p_puzzle_id, p_puzzle_rating, p_solved, round(r.rating), round(nr.new_r));
  return jsonb_build_object('repeat', false, 'before', round(r.rating)::integer, 'after', round(nr.new_r)::integer, 'rd', nr.new_rd);
end $$;

create function public.server_rush_start(p_secret text)
returns uuid language plpgsql security definer set search_path = '' as $$
declare uid uuid; sid uuid;
begin
  perform private.check_secret(p_secret);
  uid := private.require_user();
  insert into private.rush_sessions (user_id) values (uid) returning id into sid;
  return sid;
end $$;

/** Finish a Puzzle Rush run. The server checks the run lasted about as long as claimed: at least a second per puzzle. */
create function public.server_rush_finish(p_secret text, p_session uuid, p_score integer)
returns jsonb language plpgsql security definer set search_path = '' as $$
declare uid uuid; s private.rush_sessions; elapsed double precision;
begin
  perform private.check_secret(p_secret);
  uid := private.require_user();
  select * into s from private.rush_sessions x where x.id = p_session and x.user_id = uid for update;
  if not found then raise exception 'no_session'; end if;
  if s.finished then raise exception 'already_finished'; end if;
  elapsed := extract(epoch from clock_timestamp() - s.started_at);
  -- The session is used up whether or not the score is accepted, so a rejected run cannot be retried with a tweaked score.
  update private.rush_sessions set finished = true where id = s.id;
  if p_score < 0 or p_score > 200 or p_score > floor(elapsed) then return jsonb_build_object('rejected', 'implausible'); end if;
  if elapsed > 300 then return jsonb_build_object('rejected', 'expired'); end if;
  insert into public.rush_scores (user_id, score) values (uid, p_score);
  return jsonb_build_object('score', p_score, 'best', (select max(x.score) from public.rush_scores x where x.user_id = uid));
end $$;

-- =============================================================================================================
-- API surface

revoke execute on all functions in schema public from public, anon, authenticated;
grant execute on function public.server_time() to anon, authenticated;
grant execute on function public.server_accept_seek(text, uuid, text) to authenticated;
grant execute on function public.server_quick_pair(text, text, integer, integer, text, boolean) to authenticated;
grant execute on function public.server_move(text, uuid, text, integer, text, text) to authenticated;
grant execute on function public.server_resign(text, uuid) to authenticated;
grant execute on function public.server_abort(text, uuid) to authenticated;
grant execute on function public.server_draw(text, uuid, text) to authenticated;
grant execute on function public.server_claim_timeout(text, uuid, boolean) to authenticated;
grant execute on function public.server_record_puzzle(text, text, integer, integer, boolean) to authenticated;
grant execute on function public.server_rush_start(text) to authenticated;
grant execute on function public.server_rush_finish(text, uuid, integer) to authenticated;
grant execute on function public.is_admin() to authenticated;
