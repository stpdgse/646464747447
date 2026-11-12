-- Rookery M4: live games, seeks (lobby / challenges), and the server-only functions that change game state.
--
-- Trust model: clients can READ games and seeks, never write game state. The Next.js server validates every move
-- with the same chess library as the browser, then calls the server_* functions below with a secret that only the
-- server knows (only its SHA-256 lives in the database). Clocks use the database's own clock, never the client's.

create schema if not exists private;
revoke all on schema private from public, anon, authenticated;

create table private.server_secrets (name text primary key, hash text not null);

create function private.check_secret(p_secret text)
returns void language plpgsql security definer set search_path = '' as $$
begin
  if p_secret is null or not exists (
    select 1 from private.server_secrets s
    where s.name = 'game_server' and s.hash = encode(extensions.digest(p_secret, 'sha256'), 'hex')
  ) then
    raise exception 'forbidden' using errcode = '42501';
  end if;
end $$;

create function private.require_user()
returns uuid language plpgsql security definer set search_path = '' as $$
declare uid uuid := auth.uid();
begin
  if uid is null then raise exception 'not_signed_in' using errcode = '42501'; end if;
  if exists (select 1 from public.profiles p where p.id = uid and p.banned_until is not null and p.banned_until > now()) then
    raise exception 'banned' using errcode = '42501';
  end if;
  return uid;
end $$;

-- ---------------------------------------------------------------------------------------------------------------
-- Games

create table public.games (
  id                uuid primary key default gen_random_uuid(),
  variant           text not null check (variant in ('standard', 'chess960', 'kingOfTheHill', 'threeCheck')),
  initial_fen       text not null,
  moves             text[] not null default '{}',
  white_id          uuid not null references public.profiles (id) on delete cascade,
  black_id          uuid not null references public.profiles (id) on delete cascade,
  status            text not null default 'started' check (status in ('started', 'finished', 'aborted')),
  result            text check (result in ('1-0', '0-1', '1/2-1/2')),
  reason            text,
  time_initial_ms   integer not null check (time_initial_ms between 15000 and 10800000),
  time_inc_ms       integer not null default 0 check (time_inc_ms between 0 and 180000),
  white_clock_ms    integer not null check (white_clock_ms >= 0),
  black_clock_ms    integer not null check (black_clock_ms >= 0),
  clock_updated_at  timestamptz not null default clock_timestamp(),
  draw_offer_by     text check (draw_offer_by in ('white', 'black')),
  rated             boolean not null default false,
  created_at        timestamptz not null default now(),
  ended_at          timestamptz,
  constraint games_two_players check (white_id <> black_id),
  constraint games_result_matches_status check ((status = 'finished') = (result is not null)),
  constraint games_moves_size check (cardinality(moves) <= 1000)
);

create index games_started_idx on public.games (created_at desc) where status = 'started';
create index games_white_idx on public.games (white_id, created_at desc);
create index games_black_idx on public.games (black_id, created_at desc);

alter table public.games enable row level security;
create policy "games are public" on public.games for select using (true);
revoke insert, update, delete on public.games from anon, authenticated;

-- ---------------------------------------------------------------------------------------------------------------
-- Seeks: open seeks (lobby) and direct challenges (target_id set)

create table public.seeks (
  id               uuid primary key default gen_random_uuid(),
  user_id          uuid not null references public.profiles (id) on delete cascade,
  target_id        uuid references public.profiles (id) on delete cascade,
  variant          text not null check (variant in ('standard', 'chess960', 'kingOfTheHill', 'threeCheck')),
  time_initial_ms  integer not null check (time_initial_ms between 15000 and 10800000),
  time_inc_ms      integer not null default 0 check (time_inc_ms between 0 and 180000),
  color            text not null default 'random' check (color in ('white', 'black', 'random')),
  created_at       timestamptz not null default now(),
  expires_at       timestamptz not null default now() + interval '15 minutes',
  constraint seeks_not_self check (target_id is null or target_id <> user_id)
);

create unique index seeks_one_open_per_user on public.seeks (user_id) where target_id is null;
create unique index seeks_one_challenge_per_pair on public.seeks (user_id, target_id) where target_id is not null;

alter table public.seeks enable row level security;
create policy "open seeks are public, challenges are private" on public.seeks for select
  using (expires_at > now() and (target_id is null or user_id = (select auth.uid()) or target_id = (select auth.uid())));
create policy "create own seeks" on public.seeks for insert to authenticated
  with check (user_id = (select auth.uid()));
create policy "creator or target can remove a seek" on public.seeks for delete to authenticated
  using (user_id = (select auth.uid()) or target_id = (select auth.uid()));
revoke update on public.seeks from anon, authenticated;
revoke insert, delete on public.seeks from anon;

-- ---------------------------------------------------------------------------------------------------------------
-- Realtime: stream changes to subscribed clients (RLS still applies to what each client may see).

do $$
begin
  if exists (select 1 from pg_publication where pubname = 'supabase_realtime') then
    alter publication supabase_realtime add table public.games;
    alter publication supabase_realtime add table public.seeks;
  end if;
end $$;

-- ---------------------------------------------------------------------------------------------------------------
-- Functions

create function public.server_time()
returns timestamptz language sql stable set search_path = '' as $$ select clock_timestamp() $$;

/** Create the game for an accepted seek. Internal. */
create function private.start_game(s public.seeks, p_joiner uuid, p_fen text)
returns public.games language plpgsql security definer set search_path = '' as $$
declare creator_white boolean; g public.games;
begin
  if exists (
    select 1 from public.games x
    where x.status = 'started' and (x.white_id in (p_joiner, s.user_id) or x.black_id in (p_joiner, s.user_id))
  ) then
    raise exception 'busy';
  end if;
  creator_white := case s.color when 'white' then true when 'black' then false else random() < 0.5 end;
  insert into public.games (variant, initial_fen, white_id, black_id, time_initial_ms, time_inc_ms, white_clock_ms, black_clock_ms)
  values (s.variant, p_fen,
          case when creator_white then s.user_id else p_joiner end,
          case when creator_white then p_joiner else s.user_id end,
          s.time_initial_ms, s.time_inc_ms, s.time_initial_ms, s.time_initial_ms)
  returning * into g;
  delete from public.seeks where id = s.id or (target_id is null and user_id in (p_joiner, s.user_id));
  return g;
end $$;

create function public.server_accept_seek(p_secret text, p_seek_id uuid, p_initial_fen text)
returns public.games language plpgsql security definer set search_path = '' as $$
declare uid uuid; s public.seeks;
begin
  perform private.check_secret(p_secret);
  uid := private.require_user();
  select * into s from public.seeks where id = p_seek_id for update;
  if not found or s.expires_at <= now() then raise exception 'gone'; end if;
  if s.user_id = uid then raise exception 'own_seek'; end if;
  if s.target_id is not null and s.target_id <> uid then raise exception 'forbidden' using errcode = '42501'; end if;
  return private.start_game(s, uid, p_initial_fen);
end $$;

/** Pair with a compatible open seek, or become one. Atomic per time control, so two people can never miss each other. */
create function public.server_quick_pair(p_secret text, p_variant text, p_time_ms integer, p_inc_ms integer, p_initial_fen text)
returns jsonb language plpgsql security definer set search_path = '' as $$
declare uid uuid; s public.seeks; g public.games; mine public.seeks;
begin
  perform private.check_secret(p_secret);
  uid := private.require_user();
  perform pg_advisory_xact_lock(hashtext(p_variant || ':' || p_time_ms || ':' || p_inc_ms));

  select * into s from public.seeks x
  where x.target_id is null and x.user_id <> uid and x.variant = p_variant
    and x.time_initial_ms = p_time_ms and x.time_inc_ms = p_inc_ms and x.expires_at > now()
    and not exists (select 1 from public.games y where y.status = 'started' and (y.white_id = x.user_id or y.black_id = x.user_id))
  order by x.created_at limit 1 for update skip locked;

  if found then
    g := private.start_game(s, uid, p_initial_fen);
    return jsonb_build_object('game', to_jsonb(g));
  end if;

  if exists (select 1 from public.games y where y.status = 'started' and (y.white_id = uid or y.black_id = uid)) then
    raise exception 'busy';
  end if;
  delete from public.seeks where user_id = uid and target_id is null;
  insert into public.seeks (user_id, variant, time_initial_ms, time_inc_ms)
  values (uid, p_variant, p_time_ms, p_inc_ms) returning * into mine;
  return jsonb_build_object('seek', to_jsonb(mine));
end $$;

/** Shared prologue: require a signed-in, unbanned caller and lock the game row. */
create function private.lock_game(p_game_id uuid)
returns public.games language plpgsql security definer set search_path = '' as $$
declare g public.games;
begin
  perform private.require_user();
  select * into g from public.games where id = p_game_id for update;
  if not found then raise exception 'not_found'; end if;
  return g;
end $$;

create function private.side_of(g public.games)
returns text language sql stable security definer set search_path = '' as $$
  select case when auth.uid() = g.white_id then 'white' when auth.uid() = g.black_id then 'black' end
$$;

create function private.ms_since(t timestamptz) returns bigint language sql stable set search_path = '' as $$
  select floor(extract(epoch from (clock_timestamp() - t)) * 1000)::bigint
$$;

create function private.finish(p_id uuid, p_result text, p_reason text, p_white integer default null, p_black integer default null)
returns public.games language sql security definer set search_path = '' as $$
  update public.games
     set status = 'finished', result = p_result, reason = p_reason, ended_at = now(), draw_offer_by = null,
         white_clock_ms = coalesce(p_white, white_clock_ms), black_clock_ms = coalesce(p_black, black_clock_ms)
   where id = p_id
  returning *
$$;

create function public.server_move(
  p_secret text, p_game_id uuid, p_uci text, p_expected_ply integer, p_result text default null, p_reason text default null
) returns public.games language plpgsql security definer set search_path = '' as $$
declare g public.games; uid uuid; side text; ply integer; left_ms bigint; new_clock integer; mover_clock integer;
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
  if ply >= 2 then
    left_ms := mover_clock - private.ms_since(g.clock_updated_at);
    if left_ms <= 0 then
      -- The flag fell before this move arrived: the move does not count.
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
    draw_offer_by = null
  where id = g.id returning * into g;

  if p_result is not null then
    g := private.finish(g.id, p_result, coalesce(p_reason, 'checkmate'));
  end if;
  return g;
end $$;

create function public.server_resign(p_secret text, p_game_id uuid)
returns public.games language plpgsql security definer set search_path = '' as $$
declare g public.games; uid uuid; side text;
begin
  perform private.check_secret(p_secret);
  g := private.lock_game(p_game_id); uid := auth.uid(); side := private.side_of(g);
  if side is null then raise exception 'not_a_player' using errcode = '42501'; end if;
  if g.status <> 'started' then raise exception 'finished'; end if;
  if cardinality(g.moves) < 2 then
    update public.games set status = 'aborted', reason = 'aborted', ended_at = now(), draw_offer_by = null where id = g.id returning * into g;
    return g;
  end if;
  return private.finish(g.id, case side when 'white' then '0-1' else '1-0' end, 'resignation');
end $$;

create function public.server_abort(p_secret text, p_game_id uuid)
returns public.games language plpgsql security definer set search_path = '' as $$
declare g public.games; uid uuid; side text;
begin
  perform private.check_secret(p_secret);
  g := private.lock_game(p_game_id); uid := auth.uid(); side := private.side_of(g);
  if side is null then raise exception 'not_a_player' using errcode = '42501'; end if;
  if g.status <> 'started' then raise exception 'finished'; end if;
  if cardinality(g.moves) >= 2 then raise exception 'too_late_to_abort'; end if;
  update public.games set status = 'aborted', reason = 'aborted', ended_at = now(), draw_offer_by = null where id = g.id returning * into g;
  return g;
end $$;

create function public.server_draw(p_secret text, p_game_id uuid, p_action text)
returns public.games language plpgsql security definer set search_path = '' as $$
declare g public.games; uid uuid; side text; other text;
begin
  perform private.check_secret(p_secret);
  if p_action not in ('offer', 'accept', 'decline', 'cancel') then raise exception 'bad_action'; end if;
  g := private.lock_game(p_game_id); uid := auth.uid(); side := private.side_of(g);
  if side is null then raise exception 'not_a_player' using errcode = '42501'; end if;
  if g.status <> 'started' then raise exception 'finished'; end if;
  other := case side when 'white' then 'black' else 'white' end;

  if p_action = 'offer' then
    if cardinality(g.moves) < 2 then raise exception 'too_early'; end if;
    if g.draw_offer_by = other then return private.finish(g.id, '1/2-1/2', 'agreement'); end if; -- both want it
    update public.games set draw_offer_by = side where id = g.id returning * into g;
  elsif p_action = 'accept' then
    if g.draw_offer_by is distinct from other then raise exception 'no_offer'; end if;
    return private.finish(g.id, '1/2-1/2', 'agreement');
  elsif p_action = 'decline' then
    if g.draw_offer_by is distinct from other then raise exception 'no_offer'; end if;
    update public.games set draw_offer_by = null where id = g.id returning * into g;
  else -- cancel own offer
    if g.draw_offer_by is distinct from side then raise exception 'no_offer'; end if;
    update public.games set draw_offer_by = null where id = g.id returning * into g;
  end if;
  return g;
end $$;

/**
 * Anyone may ask the server to check a game's clock: if the side to move has run out of time the game ends.
 * `p_other_cannot_mate` (supplied by the server) turns a timeout into a draw when the other side has no mating material.
 * A game where neither side moves within 60 seconds is aborted.
 */
create function public.server_claim_timeout(p_secret text, p_game_id uuid, p_other_cannot_mate boolean default false)
returns public.games language plpgsql security definer set search_path = '' as $$
declare g public.games; uid uuid; side text; ply integer; turn_white boolean; left_ms bigint; loser text;
begin
  perform private.check_secret(p_secret);
  g := private.lock_game(p_game_id); uid := auth.uid(); side := private.side_of(g);
  if g.status <> 'started' then return g; end if;
  ply := cardinality(g.moves);
  if ply < 2 then
    if private.ms_since(g.clock_updated_at) > 60000 then
      update public.games set status = 'aborted', reason = 'abandoned', ended_at = now(), draw_offer_by = null where id = g.id returning * into g;
    end if;
    return g;
  end if;
  turn_white := (ply % 2 = 0);
  left_ms := (case when turn_white then g.white_clock_ms else g.black_clock_ms end) - private.ms_since(g.clock_updated_at);
  if left_ms > 0 then return g; end if;
  loser := case when turn_white then 'white' else 'black' end;
  return private.finish(g.id,
    case when p_other_cannot_mate then '1/2-1/2' when loser = 'white' then '0-1' else '1-0' end, 'timeout',
    case when loser = 'white' then 0 end, case when loser = 'black' then 0 end);
end $$;

-- API surface: the server_* functions are callable through the API but useless without the server secret.
revoke execute on all functions in schema public from public, anon, authenticated;
grant execute on function public.server_time() to anon, authenticated;
grant execute on function public.server_accept_seek(text, uuid, text) to authenticated;
grant execute on function public.server_quick_pair(text, text, integer, integer, text) to authenticated;
grant execute on function public.server_move(text, uuid, text, integer, text, text) to authenticated;
grant execute on function public.server_resign(text, uuid) to authenticated;
grant execute on function public.server_abort(text, uuid) to authenticated;
grant execute on function public.server_draw(text, uuid, text) to authenticated;
grant execute on function public.server_claim_timeout(text, uuid, boolean) to authenticated;
grant execute on function public.is_admin() to authenticated;
