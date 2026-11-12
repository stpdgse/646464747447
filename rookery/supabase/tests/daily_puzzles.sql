\set ON_ERROR_STOP on
\set QUIET on
-- Runs after ratings.sql in the same scratch database (reuses the t.* helpers).

insert into auth.users (id, raw_user_meta_data) values
  ('00000000-0000-0000-0000-0000000000c1', '{"username":"ivy"}'),
  ('00000000-0000-0000-0000-0000000000c2', '{"username":"jack"}'),
  ('00000000-0000-0000-0000-0000000000c3', '{"username":"kim"}');
\set ivy  '00000000-0000-0000-0000-0000000000c1'
\set jack '00000000-0000-0000-0000-0000000000c2'
\set kim  '00000000-0000-0000-0000-0000000000c3'
\set fen 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1'

-- ============================================================ daily offers
set role authenticated; select t.as_user(:'ivy');
insert into public.seeks (user_id, variant, time_initial_ms, time_inc_ms, mode, days_per_move) values (:'ivy', 'standard', 0, 0, 'daily', 3);
select t.ok((select expires_at > now() + interval '2 days 23 hours' from public.seeks where mode = 'daily'), 'daily offers last three days');
select t.throws($q$insert into public.seeks (user_id, variant, time_initial_ms, mode) values ('00000000-0000-0000-0000-0000000000c1', 'standard', 0, 'live')$q$, 'violates check', 'a live seek needs a real time control');
select t.throws($q$insert into public.seeks (user_id, variant, time_initial_ms, mode) values ('00000000-0000-0000-0000-0000000000c1', 'standard', 0, 'daily')$q$, 'violates check', 'a daily seek needs days per move');
select t.throws($q$insert into public.seeks (user_id, variant, time_initial_ms, mode, days_per_move) values ('00000000-0000-0000-0000-0000000000c1', 'standard', 0, 'daily', 99)$q$, 'violates check', 'days per move is limited');
select t.throws($q$insert into public.seeks (user_id, variant, time_initial_ms, mode, days_per_move) values ('00000000-0000-0000-0000-0000000000c1', 'chess960', 0, 'daily', 2)$q$, 'duplicate key', 'only one open daily offer per player');
insert into public.seeks (user_id, variant, time_initial_ms, time_inc_ms) values (:'ivy', 'standard', 300000, 0);
select t.ok((select count(*) = 2 from public.seeks where user_id = :'ivy'), 'a live and a daily offer can coexist');
reset role;

-- accepting the daily offer
select id as dseek from public.seeks where mode = 'daily' and user_id = :'ivy' \gset
set role authenticated; select t.as_user(:'jack');
select (public.server_accept_seek('s3cret', :'dseek', :'fen')).id as d1 \gset
reset role;
select t.ok((select mode = 'daily' and days_per_move = 3 and white_clock_ms = 0 and black_clock_ms = 0 and time_initial_ms = 0 and cardinality(moves) = 0 from public.games where id = :'d1'), 'a daily game has no clock and a days-per-move rule');
select t.ok((select move_deadline between now() + interval '2 days 23 hours' and now() + interval '3 days 1 hour' from public.games where id = :'d1'), 'white has three days for the first move');
select t.ok((select count(*) = 1 from public.seeks where user_id = :'ivy'), 'only the accepted daily offer was consumed');
select white_id as dw, black_id as db from public.games where id = :'d1' \gset
select t.ok((select count(*) = 1 from public.notifications where user_id = :'dw' and kind = 'your_turn' and game_id = :'d1'), 'white is told it is their turn');

-- a daily game does not block a live game
set role authenticated; select t.as_user(:'jack');
select id as lseek from public.seeks where user_id = :'ivy' and mode = 'live' \gset
select t.ok((select mode = 'live' from public.server_accept_seek('s3cret', :'lseek', :'fen')), 'players in a daily game can still play live');
reset role;

-- ============================================================ daily moves
select t.as_user(:'dw');
set role authenticated; select t.as_user(:'dw');
select t.ok((select moves = '{e2e4}' and white_clock_ms = 0 and move_deadline > now() + interval '2 days 23 hours' from public.server_move('s3cret', :'d1', 'e2e4', 0)), 'daily move: no clock charge, deadline reset');
reset role;
select t.ok((select count(*) = 1 from public.notifications where user_id = :'db' and kind = 'your_turn' and game_id = :'d1' and read_at is null), 'black is notified');
set role authenticated; select t.as_user(:'db');
select t.ok((select moves = '{e2e4,e7e5}' from public.server_move('s3cret', :'d1', 'e7e5', 1)), 'black replies days later is fine');
reset role;

-- own notifications only
set role authenticated; select t.as_user(:'db');
select t.ok((select count(*) from public.notifications) = (select count(*) from public.notifications where user_id = :'db'), 'players only see their own notifications');
update public.notifications set read_at = now() where user_id = :'db';
select t.ok((select count(*) from public.notifications where read_at is not null) = (select count(*) from public.notifications), 'a player can mark their notifications read');
select t.throws($q$update public.notifications set kind = 'game_over'$q$, 'permission denied', 'only read_at is editable');
select t.throws($q$insert into public.notifications (user_id, kind) values ('00000000-0000-0000-0000-0000000000c3', 'your_turn')$q$, 'permission denied', 'players cannot create notifications');
reset role;
select t.ok((select count(*) from public.notifications where user_id = :'dw' and read_at is not null) = 0, 'marking read does not touch other players');

-- ============================================================ deadlines
create function t.daily(w uuid, b uuid, mv text[], deadline interval, rated boolean default false, variant text default 'standard') returns uuid language sql as $$
  insert into public.games (variant, initial_fen, white_id, black_id, time_initial_ms, time_inc_ms, white_clock_ms, black_clock_ms, moves, rated, mode, days_per_move, move_deadline)
  values (variant, 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1', w, b, 0, 0, 0, 0, mv, rated, 'daily', 2, now() + deadline) returning id $$;

-- a late move loses on time
select t.daily(:'ivy', :'kim', '{e2e4,e7e5}', interval '-1 hour') as lm \gset
set role authenticated; select t.as_user(:'ivy');
select t.ok((select status = 'finished' and result = '0-1' and reason = 'timeout' and cardinality(moves) = 2 from public.server_move('s3cret', :'lm', 'g1f3', 2)), 'a daily move after the deadline loses on time');
reset role;
-- a bystander claims
select t.daily(:'ivy', :'kim', '{e2e4,e7e5,g1f3}', interval '-1 minute') as cm \gset
select t.daily(:'ivy', :'kim', '{e2e4,e7e5,g1f3}', interval '1 day') as nm \gset
set role authenticated; select t.as_user(:'jack');
select t.ok((select status = 'finished' and result = '1-0' and reason = 'timeout' from public.server_claim_timeout('s3cret', :'cm')), 'a bystander can claim an expired daily game (black ran out of time)');
select t.ok((select status = 'started' from public.server_claim_timeout('s3cret', :'nm')), 'a game within its deadline is left alone');
reset role;
-- nobody moved at all
select t.daily(:'ivy', :'kim', '{}', interval '-1 hour') as am \gset
set role authenticated; select t.as_user(:'jack');
select t.ok((select status = 'aborted' and reason = 'abandoned' from public.server_claim_timeout('s3cret', :'am')), 'no first move before the deadline: aborted');
reset role;
-- draw offers work in daily games
set role authenticated; select t.as_user(:'ivy');
select t.ok((select draw_offer_by = 'white' from public.server_draw('s3cret', :'nm', 'offer')), 'draw offers work in daily games');
reset role;

-- the background sweep (a cron job has no signed-in user)
select t.as_user(null);
select t.daily(:'ivy', :'kim', '{e2e4,e7e5,g1f3}', interval '-2 hours') as s1 \gset
select t.daily(:'ivy', :'kim', '{e2e4,e7e5}', interval '-2 hours') as s2 \gset
select t.daily(:'ivy', :'kim', '{}', interval '-2 hours') as s3 \gset
select t.daily(:'ivy', :'kim', '{e2e4,e7e5}', interval '5 hours') as s4 \gset
select t.ok(private.sweep_daily() >= 3, 'the sweep handles every expired daily game');
select t.ok((select result = '1-0' and reason = 'timeout' from public.games where id = :'s1'), 'sweep: black to move ran out of time, white wins');
select t.ok((select result = '0-1' and reason = 'timeout' from public.games where id = :'s2'), 'sweep: white to move ran out of time, black wins');
select t.ok((select status = 'aborted' from public.games where id = :'s3'), 'sweep: untouched game is aborted');
select t.ok((select status = 'started' from public.games where id = :'s4'), 'sweep: a game still within its deadline is untouched');
select t.near((select count(*) from public.notifications where game_id = :'s1' and kind = 'game_over')::float8, 2, 0, 'sweep notifies both players the game ended');

-- rated daily games use their own rating
select t.daily(:'ivy', :'kim', '{e2e4,e7e5}', interval '1 day', true) as rd \gset
set role authenticated; select t.as_user(:'ivy');
select t.ok((select status = 'finished' from public.server_resign('s3cret', :'rd')), 'rated daily game resigned');
reset role;
select t.ok((select count(*) = 2 from public.ratings where perf = 'daily' and user_id in (:'ivy', :'kim')), 'daily games have their own rating');
select t.ok((select count(*) = 0 from public.ratings where perf in ('bullet', 'blitz', 'rapid') and user_id = :'kim'), 'and do not touch the live ratings');
select t.ok((select count(*) = 1 from public.notifications where game_id = :'rd' and kind = 'game_over' and user_id = :'kim'), 'the opponent is told about the resignation');
select t.ok((select count(*) = 0 from public.notifications where game_id = :'rd' and kind = 'game_over' and user_id = :'ivy'), 'but the player who resigned is not');
select t.daily(:'ivy', :'kim', '{e2e4,e7e5}', interval '1 day', true, 'chess960') as rv \gset
set role authenticated; select t.as_user(:'kim');
select public.server_resign('s3cret', :'rv');
reset role;
select t.ok((select games = 2 from public.ratings where perf = 'daily' and user_id = :'kim'), 'daily variant games share the daily rating');

-- ============================================================ puzzles
set role authenticated; select t.as_user(:'kim');
select t.throws($q$select public.server_record_puzzle('nope', 'abc12', 1500, 80, true)$q$, 'forbidden', 'puzzle results need the server secret');
select t.throws($q$select public.server_record_puzzle('s3cret', 'abc12', 100, 80, true)$q$, 'bad_puzzle', 'puzzle ratings are range checked');
-- expected 1675.030 from the glicko2 npm library: a 1500/350 newcomer solving a 1500/80 puzzle
select t.near((select (public.server_record_puzzle('s3cret', 'abc12', 1500, 80, true)->>'after')::float8), 1675, 0.6, 'solving a 1500 puzzle as a newcomer matches the reference library');
reset role;
select t.near((select rating from public.ratings where user_id = :'kim' and perf = 'puzzle'), 1675.030, 0.01, 'puzzle rating row stores the exact value');
set role authenticated; select t.as_user(:'kim');
select t.ok((select public.server_record_puzzle('s3cret', 'abc12', 1500, 80, false) ->> 'repeat' = 'true'), 'a puzzle counts only once');
select t.ok((select (x ->> 'after')::int < (x ->> 'before')::int from (select public.server_record_puzzle('s3cret', 'zzz99', 1900, 80, false) as x) q), 'failing a puzzle lowers the rating');
reset role;
select t.ok((select count(*) = 2 and bool_or(solved) and bool_or(not solved) from public.puzzle_attempts where user_id = :'kim'), 'attempts are stored');
set role authenticated; select t.as_user(:'kim');
select t.throws($q$update public.puzzle_attempts set solved = true$q$, 'permission denied', 'attempts cannot be edited');
select t.throws($q$select * from private.rush_sessions$q$, 'permission denied', 'rush sessions are private');
reset role;

-- puzzle rush
set role authenticated; select t.as_user(:'kim');
select public.server_rush_start('s3cret') as rs \gset
select t.ok((select public.server_rush_finish('s3cret', :'rs', 5) ->> 'rejected' = 'implausible'), 'a score faster than one puzzle per second is rejected');
select t.throws(format($q$select public.server_rush_finish('s3cret', %L, 1)$q$, :'rs'), 'already_finished', 'a rejected session cannot be retried with a lower score');
select public.server_rush_start('s3cret') as rs2 \gset
select t.as_user(:'ivy');
select t.throws(format($q$select public.server_rush_finish('s3cret', %L, 0)$q$, :'rs2'), 'no_session', 'cannot finish someone elses session');
reset role;
update private.rush_sessions set started_at = clock_timestamp() - interval '181 seconds' where id = :'rs2';
set role authenticated; select t.as_user(:'kim');
select t.ok((select (public.server_rush_finish('s3cret', :'rs2', 31) ->> 'best')::int = 31), 'a plausible run is recorded with the personal best');
select public.server_rush_start('s3cret') as rs3 \gset
reset role;
update private.rush_sessions set started_at = clock_timestamp() - interval '400 seconds' where id = :'rs3';
set role authenticated; select t.as_user(:'kim');
select t.ok((select public.server_rush_finish('s3cret', :'rs3', 10) ->> 'rejected' = 'expired'), 'very old sessions expire');
reset role;
set role anon;
select t.ok((select count(*) = 1 and max(score) = 31 from public.rush_scores), 'rush scores are public');
reset role;

\echo ALL DAILY AND PUZZLE TESTS PASSED
