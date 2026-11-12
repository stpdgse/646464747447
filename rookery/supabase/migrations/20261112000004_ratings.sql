-- Rookery M5: Glicko-2 ratings, rating history, rated pairing.
-- Ratings are computed inside the database at the moment a rated game finishes, so every way a game can end
-- (checkmate, resignation, agreed draw, timeout claimed by anyone) updates them atomically and exactly once.

alter table public.games add column white_rating integer;
alter table public.games add column black_rating integer;
alter table public.seeks add column rated boolean not null default false;

create table public.ratings (
  user_id     uuid not null references public.profiles (id) on delete cascade,
  perf        text not null check (perf in ('bullet', 'blitz', 'rapid', 'chess960', 'kingOfTheHill', 'threeCheck')),
  rating      double precision not null default 1500,
  rd          double precision not null default 350,
  vol         double precision not null default 0.06,
  games       integer not null default 0,
  updated_at  timestamptz not null default now(),
  primary key (user_id, perf)
);
create index ratings_leaderboard_idx on public.ratings (perf, rating desc);

create table public.rating_history (
  id             bigint generated always as identity primary key,
  user_id        uuid not null references public.profiles (id) on delete cascade,
  perf           text not null,
  game_id        uuid not null references public.games (id) on delete cascade,
  rating_before  integer not null,
  rating_after   integer not null,
  rd_after       real not null,
  created_at     timestamptz not null default now(),
  unique (user_id, game_id)
);
create index rating_history_user_idx on public.rating_history (user_id, perf, created_at);
create index rating_history_game_idx on public.rating_history (game_id);

alter table public.ratings enable row level security;
alter table public.rating_history enable row level security;
create policy "ratings are public" on public.ratings for select using (true);
create policy "rating history is public" on public.rating_history for select using (true);
revoke all on table public.ratings from anon, authenticated;
revoke all on table public.rating_history from anon, authenticated;
grant select on table public.ratings to anon, authenticated;
grant select on table public.rating_history to anon, authenticated;

-- Which rating a game counts towards: the variant, or for standard chess the speed (Lichess's base + 40 x increment).
create function private.perf_of(p_variant text, p_ms integer, p_inc integer)
returns text language sql immutable set search_path = '' as $$
  select case
    when p_variant <> 'standard' then p_variant
    when p_ms / 1000.0 + 40 * (p_inc / 1000.0) < 180 then 'bullet'
    when p_ms / 1000.0 + 40 * (p_inc / 1000.0) < 480 then 'blitz'
    else 'rapid'
  end
$$;

-- Glickman's volatility equation f(x).
create function private.glicko_f(x double precision, delta double precision, phi double precision, v double precision, a double precision, tau double precision)
returns double precision language sql immutable set search_path = '' as $$
  select exp(x) * (delta ^ 2 - phi ^ 2 - v - exp(x)) / (2 * (phi ^ 2 + v + exp(x)) ^ 2) - (x - a) / tau ^ 2
$$;

/** One Glicko-2 rating update for one game against one opponent (tau 0.5). */
create function private.glicko2(
  r double precision, rd double precision, vol double precision,
  opp_r double precision, opp_rd double precision, score double precision,
  out new_r double precision, out new_rd double precision, out new_vol double precision
) language plpgsql immutable set search_path = '' as $$
declare
  q constant double precision := 173.7178;
  tau constant double precision := 0.5;
  mu double precision := (r - 1500) / q;
  phi double precision := rd / q;
  mu_j double precision := (opp_r - 1500) / q;
  phi_j double precision := opp_rd / q;
  g double precision := 1 / sqrt(1 + 3 * (opp_rd / q) ^ 2 / pi() ^ 2);
  e double precision;
  v double precision;
  delta double precision;
  a double precision := ln(vol ^ 2);
  big_a double precision; big_b double precision; big_c double precision;
  fa double precision; fb double precision; fc double precision;
  k integer := 1; n integer := 0;
  phi_star double precision; new_phi double precision; new_mu double precision;
begin
  e := 1 / (1 + exp(-g * (mu - mu_j)));
  v := 1 / (g ^ 2 * e * (1 - e));
  delta := v * g * (score - e);

  big_a := a;
  if delta ^ 2 > phi ^ 2 + v then
    big_b := ln(delta ^ 2 - phi ^ 2 - v);
  else
    while private.glicko_f(a - k * tau, delta, phi, v, a, tau) < 0 and k < 100 loop k := k + 1; end loop;
    big_b := a - k * tau;
  end if;
  fa := private.glicko_f(big_a, delta, phi, v, a, tau);
  fb := private.glicko_f(big_b, delta, phi, v, a, tau);
  while abs(big_b - big_a) > 0.000001 and n < 200 loop
    big_c := big_a + (big_a - big_b) * fa / (fb - fa);
    fc := private.glicko_f(big_c, delta, phi, v, a, tau);
    if fc * fb <= 0 then big_a := big_b; fa := fb; else fa := fa / 2; end if;
    big_b := big_c; fb := fc;
    n := n + 1;
  end loop;
  new_vol := exp(big_a / 2);

  phi_star := sqrt(phi ^ 2 + new_vol ^ 2);
  new_phi := 1 / sqrt(1 / phi_star ^ 2 + 1 / v);
  new_mu := mu + new_phi ^ 2 * g * (score - e);
  new_r := greatest(100, q * new_mu + 1500);
  new_rd := least(350, q * new_phi);
end $$;

/** Apply a finished rated game to both players' ratings. Safe to call more than once. */
create function private.apply_ratings(g public.games)
returns void language plpgsql security definer set search_path = '' as $$
declare
  v_perf text; w public.ratings; b public.ratings; ws double precision; wn record; bn record;
begin
  if not g.rated or g.status <> 'finished' or g.result is null then return; end if;
  if exists (select 1 from public.rating_history h where h.game_id = g.id) then return; end if;
  v_perf := private.perf_of(g.variant, g.time_initial_ms, g.time_inc_ms);

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

-- Remember each player's rating when the game starts (shown next to their name).
create function private.snapshot_ratings()
returns trigger language plpgsql security definer set search_path = '' as $$
declare v_perf text := private.perf_of(new.variant, new.time_initial_ms, new.time_inc_ms);
begin
  new.white_rating := coalesce((select round(r.rating)::integer from public.ratings r where r.user_id = new.white_id and r.perf = v_perf), 1500);
  new.black_rating := coalesce((select round(r.rating)::integer from public.ratings r where r.user_id = new.black_id and r.perf = v_perf), 1500);
  return new;
end $$;
create trigger games_snapshot_ratings before insert on public.games
  for each row execute function private.snapshot_ratings();

-- finish() now also settles ratings, and only acts on games that are still running.
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
  return g;
end $$;

-- Pairing now carries the rated flag.
create or replace function private.start_game(s public.seeks, p_joiner uuid, p_fen text)
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
  insert into public.games (variant, initial_fen, white_id, black_id, time_initial_ms, time_inc_ms, white_clock_ms, black_clock_ms, rated)
  values (s.variant, p_fen,
          case when creator_white then s.user_id else p_joiner end,
          case when creator_white then p_joiner else s.user_id end,
          s.time_initial_ms, s.time_inc_ms, s.time_initial_ms, s.time_initial_ms, s.rated)
  returning * into g;
  delete from public.seeks where id = s.id or (target_id is null and user_id in (p_joiner, s.user_id));
  return g;
end $$;

drop function public.server_quick_pair(text, text, integer, integer, text);

create function public.server_quick_pair(p_secret text, p_variant text, p_time_ms integer, p_inc_ms integer, p_initial_fen text, p_rated boolean default false)
returns jsonb language plpgsql security definer set search_path = '' as $$
declare uid uuid; s public.seeks; g public.games; mine public.seeks;
begin
  perform private.check_secret(p_secret);
  uid := private.require_user();
  perform pg_advisory_xact_lock(hashtext(p_variant || ':' || p_time_ms || ':' || p_inc_ms || ':' || p_rated));

  select * into s from public.seeks x
  where x.target_id is null and x.user_id <> uid and x.variant = p_variant and x.rated = p_rated
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
  insert into public.seeks (user_id, variant, time_initial_ms, time_inc_ms, rated)
  values (uid, p_variant, p_time_ms, p_inc_ms, p_rated) returning * into mine;
  return jsonb_build_object('seek', to_jsonb(mine));
end $$;

revoke execute on all functions in schema public from public, anon, authenticated;
grant execute on function public.server_time() to anon, authenticated;
grant execute on function public.server_accept_seek(text, uuid, text) to authenticated;
grant execute on function public.server_quick_pair(text, text, integer, integer, text, boolean) to authenticated;
grant execute on function public.server_move(text, uuid, text, integer, text, text) to authenticated;
grant execute on function public.server_resign(text, uuid) to authenticated;
grant execute on function public.server_abort(text, uuid) to authenticated;
grant execute on function public.server_draw(text, uuid, text) to authenticated;
grant execute on function public.server_claim_timeout(text, uuid, boolean) to authenticated;
grant execute on function public.is_admin() to authenticated;
