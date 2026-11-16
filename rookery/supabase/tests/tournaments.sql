\set ON_ERROR_STOP on
\set QUIET on
-- Runs after custom_bots.sql in the same scratch database. mia = admin, noah = moderator; alice, bob, cara, dan play.
\set mia   '00000000-0000-0000-0000-0000000000e1'
\set noah  '00000000-0000-0000-0000-0000000000e2'
\set olga  '00000000-0000-0000-0000-0000000000e3'
\set alice '00000000-0000-0000-0000-0000000000a1'
\set bob   '00000000-0000-0000-0000-0000000000a2'
\set cara  '00000000-0000-0000-0000-0000000000a3'
\set dan   '00000000-0000-0000-0000-0000000000a4'
\set fen 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1'
-- earlier suites leave games open; close them so players are free
update public.games set status = 'aborted' where status = 'started';
update public.profiles set banned_until = null, muted_until = null; -- earlier suites ban and mute people

-- ============================================================ creating
set role authenticated; select t.as_user(:'alice');
select t.throws($q$insert into public.tournaments (name, created_by, variant, time_initial_ms, starts_at, ends_at) values ('Players cannot', '00000000-0000-0000-0000-0000000000a1', 'standard', 180000, now(), now() + interval '1 hour')$q$, 'row-level security', 'players cannot create tournaments');
select t.as_user(:'noah');
insert into public.tournaments (name, created_by, variant, time_initial_ms, time_inc_ms, starts_at, ends_at)
values ('Friday Arena', :'noah', 'standard', 180000, 2000, now() - interval '1 minute', now() + interval '1 hour');
select t.ok(true, 'moderators can create tournaments');
select t.throws($q$insert into public.tournaments (name, created_by, variant, time_initial_ms, starts_at, ends_at) values ('Too short', '00000000-0000-0000-0000-0000000000e2', 'standard', 180000, now(), now() + interval '2 minutes')$q$, 'check constraint', 'a tournament must last at least five minutes');
select t.throws($q$insert into public.tournaments (name, created_by, variant, time_initial_ms, starts_at, ends_at) values ('Too long', '00000000-0000-0000-0000-0000000000e2', 'standard', 180000, now(), now() + interval '7 hours')$q$, 'check constraint', 'and at most six hours');
select t.throws($q$insert into public.tournaments (name, created_by, variant, time_initial_ms, starts_at, ends_at) values ('Backdated', '00000000-0000-0000-0000-0000000000e2', 'standard', 180000, now() - interval '2 hours', now() + interval '1 hour')$q$, 'row-level security', 'it cannot start in the past');
reset role;
update public.tournaments set id = '77777777-7777-7777-7777-777777777777' where name = 'Friday Arena';
-- a future one for the "not started" check
insert into public.tournaments (id, name, created_by, variant, time_initial_ms, starts_at, ends_at)
values ('88888888-8888-8888-8888-888888888888', 'Later', :'noah', 'standard', 180000, now() + interval '1 day', now() + interval '1 day 1 hour');

-- ============================================================ joining
set role authenticated; select t.as_user(:'alice');
select public.tournament_join('77777777-7777-7777-7777-777777777777');
select public.tournament_join('77777777-7777-7777-7777-777777777777');
select t.ok((select count(*) from public.tournament_players where user_id = :'alice') = 1, 'joining twice is harmless');
select t.as_user(:'bob'); select public.tournament_join('77777777-7777-7777-7777-777777777777');
select t.as_user(:'cara'); select public.tournament_join('77777777-7777-7777-7777-777777777777');
select t.throws($q$select public.tournament_join('99999999-9999-9999-9999-999999999999')$q$, 'not_found', 'unknown tournament');
reset role; set role anon; select t.as_user(null);
select t.ok((select count(*) from public.tournament_players) = 3, 'standings are public');
select t.throws($q$select public.tournament_join('77777777-7777-7777-7777-777777777777')$q$, 'permission denied', 'visitors cannot join');
reset role;

-- ============================================================ pairing
set role authenticated; select t.as_user(:'dan');
select t.throws($q$select public.server_tournament_play('s3cret', '77777777-7777-7777-7777-777777777777', 'x')$q$, 'not_joined', 'you must join first');
select t.as_user(:'alice');
select t.throws($q$select public.server_tournament_play('wrong', '77777777-7777-7777-7777-777777777777', 'x')$q$, 'forbidden', 'the server secret is required');
select t.throws($q$select public.server_tournament_play('s3cret', '88888888-8888-8888-8888-888888888888', 'x')$q$, 'not_started', 'cannot play before the tournament starts');
select t.ok((select public.server_tournament_play('s3cret', '77777777-7777-7777-7777-777777777777', :'fen') ->> 'waiting') = 'true', 'the first player waits');
select t.as_user(:'bob');
create temp table g1 as select (public.server_tournament_play('s3cret', '77777777-7777-7777-7777-777777777777', :'fen') ->> 'game_id')::uuid as id;
select t.ok((select id from g1) is not null, 'the second player is paired at once');
reset role;
select t.ok((select tournament_id from public.games where id = (select id from g1)) = '77777777-7777-7777-7777-777777777777', 'the game is tagged with the tournament');
select t.ok((select time_initial_ms = 180000 and time_inc_ms = 2000 and mode = 'live' and not rated from public.games where id = (select id from g1)), 'with the tournament time control, casual');
set role authenticated; select t.as_user(:'alice');
select t.ok((select public.server_tournament_play('s3cret', '77777777-7777-7777-7777-777777777777', :'fen') ->> 'game_id')::uuid = (select id from g1), 'the player who was paired finds the same game when they ask again');
select t.as_user(:'cara');
select t.ok((select public.server_tournament_play('s3cret', '77777777-7777-7777-7777-777777777777', :'fen') ->> 'waiting') = 'true', 'a third player waits (the others are busy)');
reset role;

-- ============================================================ scoring
-- a game that ends at once scores nothing
create temp table g2 as select id from public.games where id = (select id from g1);
update public.games set status = 'finished', result = '1-0', reason = 'resignation', ended_at = now() where id = (select id from g1);
select t.ok((select points from public.tournament_players where user_id = :'alice') = 0 and (select points from public.tournament_players where user_id = :'bob') = 0, 'a game with fewer than two moves each scores nothing');
select t.ok((select games from public.tournament_players where user_id = :'alice') = 0, 'and does not count as a game played');

-- a real game: white wins. cara has been waiting, so bob (free again) is paired with her at once.
set role authenticated; select t.as_user(:'bob');
create temp table g3 as select (public.server_tournament_play('s3cret', '77777777-7777-7777-7777-777777777777', :'fen') ->> 'game_id')::uuid as id;
reset role;
select t.ok((select id from g3) is not null, 'after the first game the players are free to play again');
select t.ok((select array_agg(x order by x) from (select white_id as x from public.games where id = (select id from g3) union all select black_id from public.games where id = (select id from g3)) q) = array[:'bob', :'cara']::uuid[], 'bob is paired with the waiting player, cara');
select t.ok((select last_opponent from public.tournament_players where user_id = :'bob') = :'cara', 'and the new opponent is remembered');
update public.games set moves = array['e2e4','e7e5','g1f3','b8c6'], status = 'finished', result = '1-0', reason = 'checkmate', ended_at = now() where id = (select id from g3);
select t.ok((select points from public.tournament_players where user_id = (select white_id from public.games where id = (select id from g3))) = 2, 'a win scores 2');
select t.ok((select points from public.tournament_players where user_id = (select black_id from public.games where id = (select id from g3))) = 0, 'a loss scores 0');
select t.ok((select wins from public.tournament_players where user_id = (select white_id from public.games where id = (select id from g3))) = 1
        and (select losses from public.tournament_players where user_id = (select black_id from public.games where id = (select id from g3))) = 1, 'wins and losses are counted');

-- alice waits; dan joins late and is paired with her; then a draw
update public.games set moves = array['e2e4','e7e5','g1f3','b8c6'], status = 'finished', result = '1-0', reason = 'checkmate', ended_at = now() where id = (select id from g3) and status = 'started';
set role authenticated; select t.as_user(:'alice'); select public.server_tournament_play('s3cret', '77777777-7777-7777-7777-777777777777', :'fen');
select t.as_user(:'dan'); select public.tournament_join('77777777-7777-7777-7777-777777777777');
select t.ok((select public.server_tournament_play('s3cret', '77777777-7777-7777-7777-777777777777', :'fen') ->> 'game_id') is not null, 'a late joiner is paired with the waiting player');
reset role;
create temp table g4 as select id, white_id, black_id from public.games where status = 'started' and tournament_id is not null;
create temp table before_draw as select user_id, points from public.tournament_players;
update public.games set moves = array['e2e4','e7e5','g1f3','b8c6'], status = 'finished', result = '1/2-1/2', reason = 'agreement', ended_at = now() where id = (select id from g4);
select t.ok((select bool_and(a.points = b.points + 1) from public.tournament_players a join before_draw b using (user_id) where a.user_id in (select white_id from g4 union select black_id from g4)), 'a draw scores 1 each');
-- an aborted game scores nothing and frees both
update public.tournament_players set waiting_since = now() where user_id in (:'bob', :'dan');
set role authenticated; select t.as_user(:'bob');
create temp table g5 as select public.server_tournament_play('s3cret', '77777777-7777-7777-7777-777777777777', :'fen') as r;
reset role;
update public.games set status = 'aborted' where status = 'started' and tournament_id is not null;
select t.ok((select count(*) from public.tournament_players where waiting_since is not null) = 0, 'when a game ends nobody stays in the queue');
select t.ok((select sum(points) from public.tournament_players) = 2 + 2, 'total points: one win (2) and one draw (1 + 1)');

-- ============================================================ time window and busy players
update public.tournaments set starts_at = now() - interval '2 hours', ends_at = now() - interval '1 hour' where id = '77777777-7777-7777-7777-777777777777';
set role authenticated; select t.as_user(:'alice');
select t.throws($q$select public.server_tournament_play('s3cret', '77777777-7777-7777-7777-777777777777', 'x')$q$, 'ended', 'no games can start after the end');
select t.throws($q$select public.tournament_join('77777777-7777-7777-7777-777777777777')$q$, 'ended', 'nor can anyone join');
reset role;

-- leaving
set role authenticated; select t.as_user(:'cara');
select public.tournament_leave('77777777-7777-7777-7777-777777777777');
select t.ok((select count(*) from public.tournament_players where user_id = :'cara') = 1 and (select games from public.tournament_players where user_id = :'cara') >= 1, 'someone who has played stays in the standings');
select t.ok(not (select active from public.tournament_players where user_id = :'cara'), 'but is marked as having left');
reset role;
do $$ begin raise notice 'ALL TOURNAMENT TESTS PASSED'; end $$;
