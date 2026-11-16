\set ON_ERROR_STOP on
\set QUIET on
-- Runs after achievements.sql in the same scratch database (users: olga, pete, quinn; quinn gets muted).
\set olga  '00000000-0000-0000-0000-0000000000e3'
\set pete  '00000000-0000-0000-0000-0000000000e4'
\set fen 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1'
update public.profiles set banned_until = null, muted_until = null;

set role authenticated; select t.as_user(:'olga');
insert into public.bot_matches (id, creator_id, white_bot_id, white_bot_name, white_bot_rating, black_bot_id, black_bot_name, black_bot_rating, variant, initial_fen, speed)
values ('99999999-0000-0000-0000-000000000001', :'olga', 'dermann', 'Der Mann', 51, 'ruedenrache', 'Der Mann (Rüdenrache)', 4000, 'standard', :'fen', 'fast');
select t.ok(true, 'a player can start a bot battle');
update public.bot_matches set moves = array['e2e4','e7e5'] where id = '99999999-0000-0000-0000-000000000001';
select t.ok((select array_length(moves, 1) from public.bot_matches where id = '99999999-0000-0000-0000-000000000001') = 2, 'and publish moves');
select t.throws($q$insert into public.bot_matches (id, creator_id, white_bot_id, white_bot_name, white_bot_rating, black_bot_id, black_bot_name, black_bot_rating, variant, initial_fen) values ('99999999-0000-0000-0000-000000000002', '00000000-0000-0000-0000-0000000000e4', 'a', 'a', 1, 'b', 'b', 1, 'standard', 'x')$q$, 'row-level security', 'you cannot start a battle in someone else''s name');
select t.throws($q$update public.bot_matches set white_bot_rating = 1 where id = '99999999-0000-0000-0000-000000000001'$q$, 'permission denied', 'only moves, speed and the result can change');

select t.as_user(:'pete');
update public.bot_matches set moves = array['d2d4'] where id = '99999999-0000-0000-0000-000000000001';
select t.ok((select array_length(moves, 1) from public.bot_matches where id = '99999999-0000-0000-0000-000000000001') = 2, 'other players cannot change someone else''s battle');
select t.ok((select count(*) from public.bot_matches) = 1, 'but anyone can watch it');
reset role; set role anon; select t.as_user(null);
select t.ok((select count(*) from public.bot_matches) = 1, 'signed-out visitors too');
select t.throws($q$update public.bot_matches set status = 'finished' where id = '99999999-0000-0000-0000-000000000001'$q$, 'permission denied', 'visitors cannot write');
reset role;

set role authenticated; select t.as_user(:'olga');
update public.bot_matches set status = 'finished', result = '1/2-1/2', reason = 'move limit' where id = '99999999-0000-0000-0000-000000000001';
select t.ok((select status from public.bot_matches where id = '99999999-0000-0000-0000-000000000001') = 'finished', 'the creator can finish it');
reset role;

-- a muted player cannot start new battles (same rule as chat and posts)
update public.profiles set muted_until = now() + interval '1 hour' where id = :'pete';
set role authenticated; select t.as_user(:'pete');
select t.throws($q$insert into public.bot_matches (id, creator_id, white_bot_id, white_bot_name, white_bot_rating, black_bot_id, black_bot_name, black_bot_rating, variant, initial_fen) values ('99999999-0000-0000-0000-000000000003', '00000000-0000-0000-0000-0000000000e4', 'a', 'a', 1, 'b', 'b', 1, 'standard', 'x')$q$, 'row-level security', 'muted players cannot start battles');
reset role;
update public.profiles set muted_until = null where id = :'pete';
do $$ begin raise notice 'ALL BOT BATTLE TESTS PASSED'; end $$;
