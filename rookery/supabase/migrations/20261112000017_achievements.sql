-- Achievements ("badges"). Computed from real data by check_achievements(): the caller asks, and every badge they now deserve is
-- recorded once. Nothing here can be written by the client directly.

create table public.achievements (
  user_id    uuid not null references public.profiles (id) on delete cascade,
  key        text not null check (key ~ '^[a-z0-9_]{2,40}$'),
  earned_at  timestamptz not null default now(),
  primary key (user_id, key)
);
create index achievements_key_idx on public.achievements (key, earned_at);
alter table public.achievements enable row level security;
create policy "badges are public" on public.achievements for select using (true);
revoke all on table public.achievements from anon, authenticated;
grant select on table public.achievements to anon, authenticated;

create function public.check_achievements() returns text[]
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
  ),
  added as (
    insert into public.achievements (user_id, key) select uid, key from earned on conflict do nothing returning key
  )
  select coalesce(array_agg(key order by key), '{}') into fresh from added;
  return fresh;
end $$;

revoke execute on function public.check_achievements() from public, anon, authenticated;
grant execute on function public.check_achievements() to authenticated;
