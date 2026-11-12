\set ON_ERROR_STOP on
\set QUIET on
-- Runs after daily_puzzles.sql in the same scratch database.

insert into auth.users (id, raw_user_meta_data) values
  ('00000000-0000-0000-0000-0000000000d1', '{"username":"lena"}'),
  ('00000000-0000-0000-0000-0000000000d2', '{"username":"milo"}');

create function t.game(mv text[], res text, v text default 'standard', st text default 'finished') returns void language sql as $$
  insert into public.games (variant, initial_fen, white_id, black_id, time_initial_ms, time_inc_ms, white_clock_ms, black_clock_ms, moves, status, result, reason)
  values (v, 'x', '00000000-0000-0000-0000-0000000000d1', '00000000-0000-0000-0000-0000000000d2', 180000, 0, 0, 0, mv, st,
          case when st = 'finished' then res end, case when st = 'finished' then 'resignation' end) $$;

-- Earlier suites left games behind; start from nothing so the counts below are exact.
truncate public.games cascade;

select t.game('{e2e4,e7e5,g1f3,b8c6}', '1-0');
select t.game('{e2e4,e7e5,f1c4}', '0-1');
select t.game('{e2e4,c7c5,g1f3}', '1/2-1/2');
select t.game('{e2e4,e7e5,g1f3,g8f6}', '1-0');
select t.game('{d2d4,d7d5}', '0-1');
select t.game('{e2e4}', '1-0');                                   -- counts for move 1 only (nothing follows it)
select t.game('{e2e4,e7e5,g1f3}', null, 'standard', 'started');   -- unfinished: ignored
select t.game('{e2e4,e7e5,g1f3}', '1-0', 'chess960');             -- other variants: ignored
select t.game('{e2e4,e7e5,g1f3}', null, 'standard', 'aborted');   -- aborted: ignored

set role anon;

-- first move
select t.ok((select games = 5 and white_wins = 3 and draws = 1 and black_wins = 1 from public.opening_stats('{}') where move = 'e2e4'), '1.e4: 5 finished games (3 white wins, 1 draw, 1 black win)');
select t.ok((select games = 1 and white_wins = 0 and black_wins = 1 from public.opening_stats('{}') where move = 'd2d4'), '1.d4: one game, a black win');
select t.ok((select move = 'e2e4' from public.opening_stats('{}') limit 1), 'the busiest move comes first');
select t.ok((select count(*) = 2 from public.opening_stats('{}')), 'only moves that were actually played are listed');

-- second move
select t.ok((select games = 3 and white_wins = 2 and black_wins = 1 and draws = 0 from public.opening_stats('{e2e4}') where move = 'e7e5'), '1.e4 e5: 3 games (2-1 to White)');
select t.ok((select games = 1 and draws = 1 from public.opening_stats('{e2e4}') where move = 'c7c5'), '1.e4 c5: one drawn game');
select t.ok((select count(*) = 2 from public.opening_stats('{e2e4}')), 'a game with nothing after 1.e4 adds no second move');

-- deeper
select t.ok((select games = 2 and white_wins = 2 from public.opening_stats('{e2e4,e7e5}') where move = 'g1f3'), '1.e4 e5 2.Nf3: two White wins');
select t.ok((select games = 1 and black_wins = 1 from public.opening_stats('{e2e4,e7e5}') where move = 'f1c4'), '1.e4 e5 2.Bc4: one Black win');
select t.ok((select count(*) = 2 from public.opening_stats('{e2e4,e7e5,g1f3}')), 'two replies to 2.Nf3 (unfinished, aborted and chess960 games do not count)');
select t.ok((select count(*) = 0 from public.opening_stats('{e2e4,e7e5,g1f3,b8c6}')), 'a game that ends there has no next move');

-- lines nobody has played, and bad input
select t.ok((select count(*) = 0 from public.opening_stats('{a2a3}')), 'unplayed lines return nothing');
select t.ok((select count(*) = 0 from public.opening_stats(array_fill('e2e4'::text, array[41]))), 'absurdly long prefixes return nothing');

-- order does not matter for permissions: both signed-in and signed-out visitors can read it
reset role;
set role authenticated; select t.as_user('00000000-0000-0000-0000-0000000000d1');
select t.ok((select sum(games) = 6 from public.opening_stats('{}')), 'signed-in players can read it too');
reset role;

\echo ALL OPENING TESTS PASSED
