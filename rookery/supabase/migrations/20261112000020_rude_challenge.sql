-- The Rüdenrache holiday challenge: a limited-time event. Beat a ladder of 20 "Rüde" bots (rated 100, 200 ... 2000) one after another;
-- a draw or a loss ends the attempt, and the highest Rüde you beat is your score. Eight attempts per player for the whole event.
-- The games are played in the browser (and are watchable on the Watch tab through bot_games); the Next.js server checks that a
-- claimed win is a real, legal, finished game before it reports it here with the server secret.

create table public.events (
  key        text primary key check (key ~ '^[a-z0-9-]{3,40}$'),
  name       text not null check (char_length(name) between 3 and 80),
  starts_at  timestamptz not null,
  ends_at    timestamptz not null check (ends_at > starts_at)
);
alter table public.events enable row level security;
create policy "events are public" on public.events for select using (true);
revoke all on table public.events from anon, authenticated;
grant select on table public.events to anon, authenticated;
-- Runs from now until the 7th of January. Change the dates with: update public.events set starts_at = ..., ends_at = ... where key = 'rude-holiday-2026';
insert into public.events (key, name, starts_at, ends_at) values ('rude-holiday-2026', 'Rüdenrache Holiday Challenge', now(), '2027-01-07 00:00:00+00');

create table public.rude_attempts (
  id          uuid primary key default gen_random_uuid(),
  user_id     uuid not null references public.profiles (id) on delete cascade,
  event_key   text not null references public.events (key) on delete cascade,
  attempt_no  integer not null check (attempt_no between 1 and 8),
  -- the Rüde to beat next (100 ... 2000, or 2100 once the whole ladder is cleared)
  level       integer not null default 100 check (level between 100 and 2100 and level % 100 = 0),
  best_level  integer not null default 0 check (best_level between 0 and 2000 and best_level % 100 = 0),
  cleared     boolean not null default false,
  status      text not null default 'playing' check (status in ('playing', 'finished')),
  started_at  timestamptz not null default now(),
  updated_at  timestamptz not null default now(),
  ended_at    timestamptz,
  unique (user_id, event_key, attempt_no)
);
create index rude_attempts_board_idx on public.rude_attempts (event_key, best_level desc);
create index rude_attempts_user_idx on public.rude_attempts (user_id);
alter table public.rude_attempts enable row level security;
create policy "the leaderboard is public" on public.rude_attempts for select using (true);
revoke all on table public.rude_attempts from anon, authenticated;
grant select on table public.rude_attempts to anon, authenticated;

-- Start an attempt, or carry on the one in progress. An attempt nobody has touched for 30 minutes counts as over.
create function public.rude_start(p_event text) returns public.rude_attempts
language plpgsql security definer set search_path = '' as $$
declare uid uuid := private.require_user(); ev public.events; a public.rude_attempts; n integer;
begin
  select * into ev from public.events where key = p_event;
  if not found then raise exception 'not_found'; end if;
  if now() < ev.starts_at then raise exception 'not_started'; end if;
  if now() >= ev.ends_at then raise exception 'event_over'; end if;
  update public.rude_attempts set status = 'finished', ended_at = updated_at
   where user_id = uid and event_key = p_event and status = 'playing' and updated_at < now() - interval '30 minutes';
  select * into a from public.rude_attempts where user_id = uid and event_key = p_event and status = 'playing' order by attempt_no desc limit 1;
  if found then return a; end if;
  select count(*) into n from public.rude_attempts where user_id = uid and event_key = p_event;
  if n >= 8 then raise exception 'no_attempts'; end if;
  insert into public.rude_attempts (user_id, event_key, attempt_no) values (uid, p_event, n + 1) returning * into a;
  return a;
end $$;

-- Keeps an attempt alive while a long game is being played.
create function public.rude_beat(p_attempt uuid) returns void
language plpgsql security definer set search_path = '' as $$
declare uid uuid := private.require_user();
begin
  update public.rude_attempts set updated_at = now() where id = p_attempt and user_id = uid and status = 'playing';
end $$;

-- Called by the Next.js server after it has checked the game. A win moves up the ladder; anything else ends the attempt.
create function public.server_rude_report(p_secret text, p_attempt uuid, p_level integer, p_outcome text) returns public.rude_attempts
language plpgsql security definer set search_path = '' as $$
declare uid uuid; a public.rude_attempts;
begin
  perform private.check_secret(p_secret);
  uid := private.require_user();
  select * into a from public.rude_attempts where id = p_attempt and user_id = uid for update;
  if not found then raise exception 'not_found'; end if;
  if a.status <> 'playing' then raise exception 'finished'; end if;
  if a.level <> p_level then raise exception 'wrong_level'; end if;
  if p_outcome = 'win' then
    update public.rude_attempts set
      best_level = a.level, level = a.level + 100, cleared = (a.level = 2000),
      status = case when a.level = 2000 then 'finished' else 'playing' end,
      ended_at = case when a.level = 2000 then now() end, updated_at = now()
    where id = a.id returning * into a;
  elsif p_outcome in ('loss', 'draw') then
    update public.rude_attempts set status = 'finished', ended_at = now(), updated_at = now() where id = a.id returning * into a;
  else
    raise exception 'bad_outcome';
  end if;
  return a;
end $$;

revoke execute on function public.rude_start(text), public.rude_beat(uuid), public.server_rude_report(text, uuid, integer, text) from public, anon, authenticated;
grant execute on function public.rude_start(text), public.rude_beat(uuid), public.server_rude_report(text, uuid, integer, text) to authenticated;

-- Badges for the event (added to the badge checker)
create or replace function public.check_achievements() returns text[]
language plpgsql security definer set search_path = '' as $$
declare uid uuid := (select auth.uid()); fresh text[];
begin
  if uid is null then return '{}'; end if;
  with
  g as (select count(*) as played,
               count(*) filter (where (result = '1-0' and white_id = uid) or (result = '0-1' and black_id = uid)) as wins,
               count(*) filter (where mode = 'daily' and ((result = '1-0' and white_id = uid) or (result = '0-1' and black_id = uid))) as daily_wins
          from public.games where status = 'finished' and cardinality(moves) >= 4 and uid in (white_id, black_id)),
  r as (select coalesce(max(rating) filter (where games >= 10), 0) as best from public.ratings where user_id = uid),
  bw as (select bot_id, bot_rating from public.bot_games
          where player_id = uid and status = 'finished' and cardinality(moves) >= 10
            and ((result = '1-0' and player_color = 'white') or (result = '0-1' and player_color = 'black'))),
  pz as (select count(*) filter (where solved) as solved, coalesce(max(rating_after), 0) as best from public.puzzle_attempts where user_id = uid),
  rush as (select coalesce(max(score), 0) as best from public.rush_scores where user_id = uid),
  lp as (select count(*) filter (where lesson_id <> 'endgames') as lessons,
                count(*) filter (where lesson_id = 'basics') as basics,
                count(*) filter (where lesson_id = 'endgames') as endgames
           from public.lesson_progress where user_id = uid),
  fr as (select count(*) as n from public.friendships where status = 'accepted' and uid in (requester_id, addressee_id)),
  cl as (select count(*) filter (where status = 'active') as member, count(*) filter (where status = 'active' and role = 'owner') as owner
           from public.club_members where user_id = uid),
  fp as (select count(*) as n from public.forum_posts where author_id = uid and deleted_at is null),
  tp as (select count(*) filter (where games > 0) as played from public.tournament_players where user_id = uid),
  ranked as (select tp2.tournament_id, tp2.user_id,
                    rank() over (partition by tp2.tournament_id order by tp2.points desc, tp2.wins desc) as pos,
                    count(*) over (partition by tp2.tournament_id) as size
               from public.tournament_players tp2 join public.tournaments tt on tt.id = tp2.tournament_id
              where tt.ends_at < now() and tp2.games > 0),
  tr as (select count(*) filter (where pos <= 3 and size >= 3) as podium, count(*) filter (where pos = 1 and size >= 3) as won from ranked where user_id = uid),
  ru as (select coalesce(max(best_level), 0) as best, count(*) as attempts, coalesce(bool_or(cleared), false) as cleared from public.rude_attempts where user_id = uid),
  earned(key) as (
    select 'first_game' from g where played >= 1
    union all select 'games_25' from g where played >= 25
    union all select 'first_win' from g where wins >= 1
    union all select 'wins_10' from g where wins >= 10
    union all select 'wins_50' from g where wins >= 50
    union all select 'daily_win' from g where daily_wins >= 1
    union all select 'rated_1600' from r where best >= 1600
    union all select 'rated_1800' from r where best >= 1800
    union all select 'rated_2000' from r where best >= 2000
    union all select 'bot_first_win' where exists (select 1 from bw)
    union all select 'bot_beat_1000' where exists (select 1 from bw where bot_rating >= 1000)
    union all select 'bot_beat_1500' where exists (select 1 from bw where bot_rating >= 1500)
    union all select 'bot_beat_2000' where exists (select 1 from bw where bot_rating >= 2000)
    union all select 'bot_beat_2500' where exists (select 1 from bw where bot_rating >= 2500)
    union all select 'bot_dermann' where exists (select 1 from bw where bot_id = 'dermann')
    union all select 'bot_ruedenrache' where exists (select 1 from bw where bot_id = 'ruedenrache')
    union all select 'puzzles_10' from pz where solved >= 10
    union all select 'puzzles_100' from pz where solved >= 100
    union all select 'puzzles_500' from pz where solved >= 500
    union all select 'puzzle_1800' from pz where best >= 1800
    union all select 'rush_10' from rush where best >= 10
    union all select 'rush_25' from rush where best >= 25
    union all select 'lesson_first' from lp where lessons >= 1
    union all select 'lessons_50' from lp where lessons >= 50
    union all select 'basics_done' from lp where basics >= 8
    union all select 'endgame_first' from lp where endgames >= 1
    union all select 'endgames_all' from lp where endgames >= 9
    union all select 'first_friend' from fr where n >= 1
    union all select 'friends_5' from fr where n >= 5
    union all select 'club_member' from cl where member >= 1
    union all select 'club_owner' from cl where owner >= 1
    union all select 'forum_first' from fp where n >= 1
    union all select 'forum_20' from fp where n >= 20
    union all select 'tournament_played' from tp where played >= 1
    union all select 'tournament_podium' from tr where podium >= 1
    union all select 'tournament_winner' from tr where won >= 1
    union all select 'rude_holiday' from ru where attempts >= 1
    union all select 'rude_first' from ru where best >= 100
    union all select 'rude_500' from ru where best >= 500
    union all select 'rude_1000' from ru where best >= 1000
    union all select 'rude_1500' from ru where best >= 1500
    union all select 'rude_2000' from ru where cleared
  ),
  added as (
    insert into public.achievements (user_id, key) select uid, key from earned on conflict do nothing returning key
  )
  select coalesce(array_agg(key order by key), '{}') into fresh from added;
  return fresh;
end $$;


revoke execute on function public.check_achievements() from public, anon, authenticated;
grant execute on function public.check_achievements() to authenticated;
