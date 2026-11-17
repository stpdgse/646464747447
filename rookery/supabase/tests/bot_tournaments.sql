\set ON_ERROR_STOP on
\set QUIET on
-- Runs after bot_matches.sql in the same scratch database (users: olga, pete, quinn, rex; noah is a moderator).
\set noah  '00000000-0000-0000-0000-0000000000e2'
\set olga  '00000000-0000-0000-0000-0000000000e3'
\set pete  '00000000-0000-0000-0000-0000000000e4'
\set quinn '00000000-0000-0000-0000-0000000000e5'
\set rex   '00000000-0000-0000-0000-0000000000e6'
\set fen 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1'
update public.profiles set banned_until = null, muted_until = null;

create function t.bots(n int) returns jsonb language sql as $$
  select jsonb_agg(jsonb_build_object('id', 'bot' || i, 'name', 'Bot ' || i, 'rating', 500 + i * 100)) from generate_series(1, n) i $$;
grant execute on function t.bots(int) to authenticated, anon;

-- ============================================================ creating
set role authenticated; select t.as_user(:'olga');
insert into public.bot_tournaments (creator_id, name, format, best_of, bots)
values (:'olga', 'Olga Cup', 'knockout', 1, t.bots(8));
select t.ok(true, 'any player can create a bot tournament');
reset role; update public.bot_tournaments set id = 'aaaaaaaa-0000-0000-0000-000000000001' where name = 'Olga Cup'; set role authenticated; select t.as_user(:'olga');
select t.throws($q$insert into public.bot_tournaments (creator_id, name, format, bots) values ('00000000-0000-0000-0000-0000000000e3', 'Second', 'knockout', t.bots(4))$q$, 'too_many_tournaments', 'but only one running at a time');
select t.as_user(:'pete');
select t.throws($q$insert into public.bot_tournaments (creator_id, name, format, bots) values ('00000000-0000-0000-0000-0000000000e5', 'Not mine', 'knockout', t.bots(4))$q$, 'row-level security', 'you cannot create one in someone else''s name');
select t.throws($q$insert into public.bot_tournaments (creator_id, name, format, bots) values ('00000000-0000-0000-0000-0000000000e4', 'Bad size', 'knockout', t.bots(5))$q$, 'bad_size', 'a knockout needs 4, 8 or 16 bots');
select t.throws($q$insert into public.bot_tournaments (creator_id, name, format, bots) values ('00000000-0000-0000-0000-0000000000e4', 'Too many', 'roundrobin', t.bots(9))$q$, 'bad_size', 'a round-robin takes at most 8');
select t.throws($q$insert into public.bot_tournaments (creator_id, name, format, bots) values ('00000000-0000-0000-0000-0000000000e4', 'Dupes', 'knockout', '[{"id":"a","name":"A","rating":1},{"id":"a","name":"A","rating":1},{"id":"b","name":"B","rating":1},{"id":"c","name":"C","rating":1}]'::jsonb)$q$, 'bad_bots', 'a bot cannot enter twice');
select t.throws($q$insert into public.bot_tournaments (creator_id, name, format, bots) values ('00000000-0000-0000-0000-0000000000e4', 'Junk', 'knockout', '"hello"'::jsonb)$q$, 'bad_bots', 'the bot list must be a real list');
select t.throws($q$insert into public.bot_tournaments (creator_id, name, format, bots, status) values ('00000000-0000-0000-0000-0000000000e4', 'Pre-won', 'knockout', t.bots(4), 'finished')$q$, 'permission denied', 'a tournament cannot start out finished');
select t.throws($q$insert into public.bot_tournaments (creator_id, name, format, bots, champion_id) values ('00000000-0000-0000-0000-0000000000e4', 'Rigged', 'knockout', t.bots(4), 'bot1')$q$, 'permission denied', 'nor have a champion picked at creation');
insert into public.bot_tournaments (creator_id, name, format, best_of, bots) values (:'pete', 'Pete League', 'roundrobin', 1, t.bots(4));
select t.ok(true, 'a different player can run their own at the same time');
reset role;
update public.bot_tournaments set id = 'aaaaaaaa-0000-0000-0000-000000000002' where name = 'Pete League';

-- site-wide cap: 8 running tournaments
insert into public.bot_tournaments (creator_id, name, format, bots)
select p.id, 'Filler ' || row_number() over (), 'knockout', t.bots(4) from (select id from public.profiles where id not in (:'olga', :'pete', :'rex') limit 6) p;
set role authenticated; select t.as_user(:'rex');
select t.throws($q$insert into public.bot_tournaments (creator_id, name, format, bots) values ('00000000-0000-0000-0000-0000000000e6', 'One too many', 'knockout', t.bots(4))$q$, 'site_busy', 'the site-wide cap stops a flood (8 at a time)');
reset role;
delete from public.bot_tournaments where name like 'Filler %';

-- ============================================================ running: the claim
set role authenticated; select t.as_user(:'olga');
select t.ok(public.claim_bot_tournament('aaaaaaaa-0000-0000-0000-000000000001'), 'the creator claims the tournament');
select t.as_user(:'pete');
select t.ok(not public.claim_bot_tournament('aaaaaaaa-0000-0000-0000-000000000001'), 'nobody else can claim it while the runner is active');
select t.ok(not public.beat_bot_tournament('aaaaaaaa-0000-0000-0000-000000000001'), 'or fake the heartbeat');
select t.throws($q$insert into public.bot_tournament_games (tournament_id, round, slot, game_no, white_bot_id, black_bot_id, initial_fen) values ('aaaaaaaa-0000-0000-0000-000000000001', 1, 0, 1, 'bot8', 'bot1', 'x')$q$, 'row-level security', 'only the runner can add games');
select t.as_user(:'olga');
insert into public.bot_tournament_games (tournament_id, round, slot, game_no, white_bot_id, black_bot_id, initial_fen) values ('aaaaaaaa-0000-0000-0000-000000000001', 1, 0, 1, 'bot8', 'bot1', :'fen');
update public.bot_tournament_games set moves = array['e2e4','e7e5'] where tournament_id = 'aaaaaaaa-0000-0000-0000-000000000001';
select t.ok(public.beat_bot_tournament('aaaaaaaa-0000-0000-0000-000000000001'), 'the runner adds games, publishes moves and keeps the heartbeat');
select t.throws($q$insert into public.bot_tournament_games (tournament_id, round, slot, game_no, white_bot_id, black_bot_id, initial_fen) values ('aaaaaaaa-0000-0000-0000-000000000001', 1, 0, 1, 'bot8', 'bot1', 'x')$q$, 'duplicate key', 'the same game cannot be added twice');
select t.throws($q$update public.bot_tournament_games set white_bot_id = 'bot2' where tournament_id = 'aaaaaaaa-0000-0000-0000-000000000001'$q$, 'permission denied', 'only moves and the result can change');
select t.as_user(:'pete');
update public.bot_tournament_games set moves = '{}' where tournament_id = 'aaaaaaaa-0000-0000-0000-000000000001';
select t.ok((select array_length(moves, 1) from public.bot_tournament_games where tournament_id = 'aaaaaaaa-0000-0000-0000-000000000001') = 2, 'other players cannot change the games');
reset role;

-- a runner who has gone quiet loses the claim to a viewer
update public.bot_tournaments set runner_beat_at = now() - interval '1 minute' where id = 'aaaaaaaa-0000-0000-0000-000000000001';
set role authenticated; select t.as_user(:'pete');
select t.ok(public.claim_bot_tournament('aaaaaaaa-0000-0000-0000-000000000001'), 'a viewer can take over once the runner has been silent for 25 seconds');
select t.ok(public.beat_bot_tournament('aaaaaaaa-0000-0000-0000-000000000001'), 'and becomes the runner');
select t.as_user(:'olga');
update public.bot_tournament_games set moves = '{}' where tournament_id = 'aaaaaaaa-0000-0000-0000-000000000001';
select t.ok((select array_length(moves, 1) from public.bot_tournament_games where tournament_id = 'aaaaaaaa-0000-0000-0000-000000000001') = 2, 'the old runner can no longer write');
select t.as_user(:'pete');
update public.bot_tournament_games set status = 'finished', result = '1-0', reason = 'checkmate' where tournament_id = 'aaaaaaaa-0000-0000-0000-000000000001';
update public.bot_tournaments set status = 'finished', champion_id = 'bot8' where id = 'aaaaaaaa-0000-0000-0000-000000000001';
select t.ok((select status from public.bot_tournaments where id = 'aaaaaaaa-0000-0000-0000-000000000001') = 'finished', 'the runner reports the winner');
select t.ok(not public.claim_bot_tournament('aaaaaaaa-0000-0000-0000-000000000001'), 'a finished tournament cannot be claimed');
reset role;

-- ============================================================ watching and removing
set role anon; select t.as_user(null);
select t.ok((select count(*) from public.bot_tournaments) >= 2 and (select count(*) from public.bot_tournament_games) = 1, 'signed-out visitors can watch');
select t.throws($q$update public.bot_tournaments set champion_id = 'x'$q$, 'permission denied', 'but not write');
reset role;
set role authenticated; select t.as_user(:'rex');
delete from public.bot_tournaments where id = 'aaaaaaaa-0000-0000-0000-000000000002';
select t.ok((select count(*) from public.bot_tournaments where id = 'aaaaaaaa-0000-0000-0000-000000000002') = 1, 'strangers cannot delete a tournament');
select t.as_user(:'noah');
delete from public.bot_tournaments where id = 'aaaaaaaa-0000-0000-0000-000000000002';
select t.ok((select count(*) from public.bot_tournaments where id = 'aaaaaaaa-0000-0000-0000-000000000002') = 0, 'a moderator can');
reset role;

-- the daily limit: 5 per person (idle ones do not block, but still count towards the day)
update public.bot_tournaments set status = 'finished' where creator_id = :'olga';
set role authenticated; select t.as_user(:'olga');
do $$ begin for i in 1..4 loop
  insert into public.bot_tournaments (creator_id, name, format, bots) values ('00000000-0000-0000-0000-0000000000e3', 'Olga ' || i, 'knockout', t.bots(4));
  update public.bot_tournaments set status = 'finished' where creator_id = '00000000-0000-0000-0000-0000000000e3' and status = 'running' and false;
  reset role; update public.bot_tournaments set status = 'finished' where creator_id = '00000000-0000-0000-0000-0000000000e3'; set role authenticated;
end loop; end $$;
select t.throws($q$insert into public.bot_tournaments (creator_id, name, format, bots) values ('00000000-0000-0000-0000-0000000000e3', 'Sixth', 'knockout', t.bots(4))$q$, 'slow_down', 'five tournaments a day is the limit');
reset role;
do $$ begin raise notice 'ALL BOT TOURNAMENT TESTS PASSED'; end $$;
