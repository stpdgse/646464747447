\set ON_ERROR_STOP on
\set QUIET on
-- Runs after social.sql in the same scratch database (reuses the t.* helpers and its users: olga, pete).
\set olga '00000000-0000-0000-0000-0000000000e3'
\set pete '00000000-0000-0000-0000-0000000000e4'

set role authenticated; select t.as_user(:'olga');
insert into public.bot_games (id, player_id, bot_id, bot_name, bot_rating, player_color, variant, initial_fen)
values ('11111111-1111-1111-1111-111111111111', :'olga', 'alf', 'Pong Alf Femboy', 650, 'white', 'standard', 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1');
select t.ok(true, 'a player can publish their bot game');
update public.bot_games set moves = array['e2e4', 'e7e5'] where id = '11111111-1111-1111-1111-111111111111';
select t.ok((select array_length(moves, 1) from public.bot_games where id = '11111111-1111-1111-1111-111111111111') = 2, 'and add moves');
select t.throws($q$insert into public.bot_games (id, player_id, bot_id, bot_name, bot_rating, player_color, variant, initial_fen) values ('22222222-2222-2222-2222-222222222222', '00000000-0000-0000-0000-0000000000e4', 'alf', 'x', 1, 'white', 'standard', 'x')$q$, 'row-level security', 'you cannot publish a game as someone else');
select t.throws($q$insert into public.bot_games (id, player_id, bot_id, bot_name, bot_rating, player_color, variant, initial_fen, status) values ('33333333-3333-3333-3333-333333333333', '00000000-0000-0000-0000-0000000000e3', 'alf', 'x', 1, 'white', 'standard', 'x', 'finished')$q$, 'row-level security', 'a game must start as playing');
select t.throws($q$update public.bot_games set player_id = '00000000-0000-0000-0000-0000000000e4' where id = '11111111-1111-1111-1111-111111111111'$q$, 'permission denied', 'the owner cannot be changed');
select t.throws($q$update public.bot_games set bot_rating = 4000 where id = '11111111-1111-1111-1111-111111111111'$q$, 'permission denied', 'only moves and the result are updatable');

select t.as_user(:'pete');
update public.bot_games set moves = array['d2d4'] where id = '11111111-1111-1111-1111-111111111111';
select t.ok((select array_length(moves, 1) from public.bot_games where id = '11111111-1111-1111-1111-111111111111') = 2, 'another player cannot change your game');
select t.ok((select count(*) from public.bot_games) = 1, 'but anyone signed in can watch it');
reset role; set role anon; select t.as_user(null);
select t.ok((select count(*) from public.bot_games) = 1, 'and so can signed-out visitors');
select t.throws($q$update public.bot_games set status = 'finished' where id = '11111111-1111-1111-1111-111111111111'$q$, 'permission denied', 'visitors cannot write');
reset role;

set role authenticated; select t.as_user(:'olga');
update public.bot_games set status = 'finished', result = '1-0', reason = 'checkmate' where id = '11111111-1111-1111-1111-111111111111';
select t.ok((select status from public.bot_games where id = '11111111-1111-1111-1111-111111111111') = 'finished', 'the player can finish the game');
reset role;
select t.ok((select updated_at >= created_at from public.bot_games where id = '11111111-1111-1111-1111-111111111111'), 'the server stamps updated_at');
-- private games (history only)
set role authenticated; select t.as_user(:'olga');
insert into public.bot_games (id, player_id, bot_id, bot_name, bot_rating, player_color, variant, initial_fen, watchable, time_control)
values ('44444444-4444-4444-4444-444444444444', :'olga', 'cam', 'Casual Cam', 1000, 'black', 'standard', 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1', false, '5+0');
select t.ok((select count(*) from public.bot_games where id = '44444444-4444-4444-4444-444444444444') = 1, 'the owner sees their private game');
select t.as_user(:'pete');
select t.ok((select count(*) from public.bot_games where id = '44444444-4444-4444-4444-444444444444') = 0, 'other players cannot see a private game');
reset role; set role anon; select t.as_user(null);
select t.ok((select count(*) from public.bot_games where id = '44444444-4444-4444-4444-444444444444') = 0, 'nor can signed-out visitors');
reset role;
do $$ begin raise notice 'ALL BOT GAME TESTS PASSED'; end $$;
