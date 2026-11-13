\set ON_ERROR_STOP on
\set QUIET on
-- Runs after tournaments.sql in the same scratch database.
insert into auth.users (id, raw_user_meta_data) values
  ('00000000-0000-0000-0000-0000000000f1', '{"username":"zed"}'),
  ('00000000-0000-0000-0000-0000000000f2', '{"username":"yan"}');
\set zed '00000000-0000-0000-0000-0000000000f1'
\set yan '00000000-0000-0000-0000-0000000000f2'
\set fen 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1'

-- zed: a win against yan (4 plies), a rating, a bot win against Der Mann, ten solved puzzles, a rush score, a lesson, a friend
insert into public.games (variant, initial_fen, white_id, black_id, status, result, reason, moves, time_initial_ms, time_inc_ms, white_clock_ms, black_clock_ms, mode)
values ('standard', :'fen', :'zed', :'yan', 'finished', '1-0', 'checkmate', array['e2e4','e7e5','g1f3','b8c6'], 180000, 0, 1000, 1000, 'live');
insert into public.games (variant, initial_fen, white_id, black_id, status, result, reason, moves, time_initial_ms, time_inc_ms, white_clock_ms, black_clock_ms, mode)
values ('standard', :'fen', :'yan', :'zed', 'finished', '1-0', 'resignation', array['e2e4'], 180000, 0, 1000, 1000, 'live');  -- too short to count
insert into public.ratings (user_id, perf, rating, games) values (:'zed', 'blitz', 1650, 12);
insert into public.bot_games (id, player_id, bot_id, bot_name, bot_rating, player_color, variant, initial_fen, moves, status, result)
values ('55555555-5555-5555-5555-555555555555', :'zed', 'dermann', 'Der Mann', 51, 'white', 'standard', :'fen', array['e2e4','e7e5','g1f3','b8c6','f1c4','g8f6','d2d4','e5d4','e1g1','f6e4'], 'finished', '1-0');
insert into public.bot_games (id, player_id, bot_id, bot_name, bot_rating, player_color, variant, initial_fen, moves, status, result)
values ('66666666-6666-6666-6666-666666666666', :'zed', 'tess', 'Tactician Tess', 1500, 'black', 'standard', :'fen', array['e2e4','e7e5','g1f3','b8c6','f1c4','g8f6','d2d4','e5d4','e1g1','f6e4'], 'finished', '0-1');
insert into public.bot_games (id, player_id, bot_id, bot_name, bot_rating, player_color, variant, initial_fen, moves, status, result)
values ('77777777-0000-0000-0000-777777777777', :'zed', 'max', 'Master Max', 2300, 'white', 'standard', :'fen', array['e2e4'], 'finished', '1-0');  -- too short, does not count
insert into public.puzzle_attempts (user_id, puzzle_id, puzzle_rating, solved, rating_before, rating_after)
select :'zed', 'p' || n, 1000, true, 1500, 1500 from generate_series(1, 10) n;
insert into public.rush_scores (user_id, score) values (:'zed', 12);
insert into public.lesson_progress (user_id, lesson_id, item_id) select :'zed', 'basics', 'b' || n from generate_series(1, 8) n;
insert into public.friendships (requester_id, addressee_id, status) values (:'zed', :'yan', 'accepted');

set role authenticated; select t.as_user(:'zed');
create temp table first_call as select public.check_achievements() as keys;
select t.ok((select keys from first_call) = array['basics_done','bot_beat_1000','bot_beat_1500','bot_dermann','bot_first_win','first_friend','first_game','first_win','lesson_first','puzzles_10','rated_1600','rush_10'],
  'zed earns exactly the badges the data supports: ' || (select array_to_string(keys, ',') from first_call));
select t.ok(public.check_achievements() = '{}', 'asking again returns nothing new');
select t.ok((select count(*) from public.achievements where user_id = :'zed') = 12, 'the badges are stored once');
select t.throws($q$insert into public.achievements (user_id, key) values ('00000000-0000-0000-0000-0000000000f1', 'wins_50')$q$, 'permission denied', 'badges cannot be written directly');
select t.throws($q$update public.achievements set key = 'wins_50' where user_id = '00000000-0000-0000-0000-0000000000f1'$q$, 'permission denied', 'or edited');

select t.as_user(:'yan');
select t.ok(public.check_achievements() = array['first_friend','first_game'], 'yan (the opponent, and a friend) earns the badges that fit');
reset role;

set role anon; select t.as_user(null);
select t.ok((select count(*) from public.achievements) >= 12, 'badges are public');
select t.throws($q$select public.check_achievements()$q$, 'permission denied', 'visitors cannot run the check');
reset role;

-- a puzzle count past a threshold adds the next badge
insert into public.puzzle_attempts (user_id, puzzle_id, puzzle_rating, solved, rating_before, rating_after)
select :'zed', 'q' || n, 1000, true, 1500, 1900 from generate_series(1, 95) n;
set role authenticated; select t.as_user(:'zed');
select t.ok(public.check_achievements() = array['puzzle_1800','puzzles_100'], 'later progress adds new badges only');
reset role;
do $$ begin raise notice 'ALL ACHIEVEMENT TESTS PASSED'; end $$;
