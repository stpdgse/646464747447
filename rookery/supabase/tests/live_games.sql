\set ON_ERROR_STOP on
\set QUIET on
-- Run by supabase/tests/run.sh against a scratch database. Every check raises on failure.

create schema t;
create function t.ok(cond boolean, msg text) returns void language plpgsql as $$
begin if cond is not true then raise exception 'FAILED: %', msg; end if; raise notice 'ok   %', msg; end $$;

/** Runs `sql`, expects it to raise something containing `expected`. */
create function t.throws(sql text, expected text, msg text) returns void language plpgsql as $$
begin
  begin execute sql; exception when others then
    if sqlerrm like '%' || expected || '%' then raise notice 'ok   %', msg; return; end if;
    raise exception 'FAILED: % (wanted "%", got "%")', msg, expected, sqlerrm;
  end;
  raise exception 'FAILED: % (did not raise)', msg;
end $$;

create function t.as_user(u uuid) returns void language plpgsql as $$
begin perform set_config('request.jwt.claim.sub', coalesce(u::text, ''), false); end $$;

grant usage on schema t to anon, authenticated;
grant execute on all functions in schema t to anon, authenticated;

-- fixtures: four players
insert into auth.users (id, raw_user_meta_data) values
  ('00000000-0000-0000-0000-0000000000a1', '{"username":"alice"}'),
  ('00000000-0000-0000-0000-0000000000a2', '{"username":"bob"}'),
  ('00000000-0000-0000-0000-0000000000a3', '{"username":"cara"}'),
  ('00000000-0000-0000-0000-0000000000a4', '{"username":"dan"}');
\set alice '00000000-0000-0000-0000-0000000000a1'
\set bob   '00000000-0000-0000-0000-0000000000a2'
\set cara  '00000000-0000-0000-0000-0000000000a3'
\set dan   '00000000-0000-0000-0000-0000000000a4'
insert into private.server_secrets values ('game_server', encode(extensions.digest('s3cret', 'sha256'), 'hex'));

\set fen 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1'

-- ============================================================ secret gating
set role authenticated;
select t.as_user(:'alice');
select t.throws($q$select public.server_quick_pair('wrong', 'standard', 180000, 2000, 'x')$q$, 'forbidden', 'wrong secret is rejected');
select t.throws($q$select public.server_quick_pair(null, 'standard', 180000, 2000, 'x')$q$, 'forbidden', 'missing secret is rejected');
select t.throws($q$select * from public.server_move('wrong', gen_random_uuid(), 'e2e4', 0)$q$, 'forbidden', 'move needs the secret');
select t.throws($q$select * from public.server_resign('wrong', gen_random_uuid())$q$, 'forbidden', 'resign needs the secret');
reset role;
select t.ok(not has_function_privilege('anon', 'public.server_move(text,uuid,text,integer,text,text)', 'execute'), 'anon cannot execute server_move');
select t.ok(not has_schema_privilege('authenticated', 'private', 'usage'), 'private schema is not reachable');

-- ============================================================ clients cannot write games directly
set role authenticated; select t.as_user(:'alice');
select t.throws($q$insert into public.games (variant, initial_fen, white_id, black_id, time_initial_ms, white_clock_ms, black_clock_ms) values ('standard', 'x', '00000000-0000-0000-0000-0000000000a1', '00000000-0000-0000-0000-0000000000a2', 60000, 60000, 60000)$q$, 'permission denied', 'cannot insert a game');
select t.throws($q$update public.games set result = '1-0'$q$, 'permission denied', 'cannot update games');
select t.throws($q$delete from public.games$q$, 'permission denied', 'cannot delete games');
select t.throws($q$select * from private.server_secrets$q$, 'permission denied', 'cannot read secrets');
reset role;

-- ============================================================ seeks and RLS
set role authenticated; select t.as_user(:'alice');
insert into public.seeks (user_id, variant, time_initial_ms, time_inc_ms) values (:'alice', 'standard', 180000, 2000);
select t.throws($q$insert into public.seeks (user_id, variant, time_initial_ms) values ('00000000-0000-0000-0000-0000000000a2', 'standard', 60000)$q$, 'row-level security', 'cannot create a seek as someone else');
select t.throws($q$insert into public.seeks (user_id, variant, time_initial_ms) values ('00000000-0000-0000-0000-0000000000a1', 'standard', 60000)$q$, 'duplicate key', 'only one open seek per user');
select t.throws($q$insert into public.seeks (user_id, variant, time_initial_ms) values ('00000000-0000-0000-0000-0000000000a1', 'standard', 1000)$q$, 'violates check', 'time control has a minimum');
select t.throws($q$update public.seeks set time_initial_ms = 60000$q$, 'permission denied', 'seeks are not editable');
-- challenge alice -> cara
insert into public.seeks (user_id, target_id, variant, time_initial_ms) values (:'alice', :'cara', 'chess960', 300000);
select t.as_user(:'bob');
select t.ok((select count(*) from public.seeks) = 1, 'bob sees the open seek but not the private challenge');
delete from public.seeks where user_id = :'alice' and target_id is null;   -- bob tries to delete alice's seek
reset role;
select t.ok((select count(*) from public.seeks where user_id = :'alice' and target_id is null) = 1, 'bob cannot delete alices seek');
set role authenticated; select t.as_user(:'cara');
select t.ok((select count(*) from public.seeks) = 2, 'cara sees the open seek and the challenge addressed to her');
select t.as_user(:'alice');
select t.ok((select count(*) from public.seeks) = 2, 'alice sees both of her own seeks');
reset role;
update public.seeks set expires_at = now() - interval '1 minute' where target_id is not null;
set role authenticated; select t.as_user(:'cara');
select t.ok((select count(*) from public.seeks) = 1, 'expired challenges disappear');
reset role;
delete from public.seeks;

-- ============================================================ quick pairing
set role authenticated; select t.as_user(:'alice');
select t.ok((select public.server_quick_pair('s3cret', 'standard', 180000, 2000, :'fen') ? 'seek'), 'first player waits as a seek');
select t.ok((select count(*) from public.seeks) = 1, 'seek exists');
select t.as_user(:'bob');
select t.ok((select public.server_quick_pair('s3cret', 'standard', 300000, 0, :'fen') ? 'seek'), 'different time control does not match');
select t.ok((select count(*) from public.seeks) = 2, 'both seeks waiting');
select t.as_user(:'bob');
select t.ok((select public.server_quick_pair('s3cret', 'standard', 180000, 2000, :'fen') ? 'game'), 'matching time control pairs');
reset role;
select t.ok((select count(*) from public.seeks) = 0, 'seeks of both players are cleared');
select t.ok((select count(*) from public.games) = 1, 'one game created');
select t.ok((select white_clock_ms = 180000 and black_clock_ms = 180000 and time_inc_ms = 2000 and status = 'started' and cardinality(moves) = 0 from public.games), 'game starts with full clocks and no moves');
select t.ok((select initial_fen = :'fen' from public.games), 'server-supplied start position is stored');
select t.ok((select white_id <> black_id and white_id in (:'alice', :'bob') and black_id in (:'alice', :'bob') from public.games), 'both players seated');

-- busy players cannot start a second live game
set role authenticated; select t.as_user(:'alice');
select t.throws($q$select public.server_quick_pair('s3cret', 'standard', 60000, 0, 'x')$q$, 'busy', 'a player in a game cannot start another');
reset role;

-- ============================================================ accepting seeks and challenges
set role authenticated; select t.as_user(:'cara');
insert into public.seeks (user_id, target_id, variant, time_initial_ms, color) values (:'cara', :'dan', 'chess960', 600000, 'white');
select id as challenge_id from public.seeks where user_id = :'cara' \gset
select t.as_user(:'bob');
select t.throws(format($q$select public.server_accept_seek('s3cret', %L, 'x')$q$, :'challenge_id'), 'forbidden', 'only the target can accept a challenge');
select t.as_user(:'cara');
select t.throws(format($q$select public.server_accept_seek('s3cret', %L, 'x')$q$, :'challenge_id'), 'own_seek', 'cannot accept your own seek');
select t.as_user(:'dan');
select t.throws($q$select public.server_accept_seek('s3cret', gen_random_uuid(), 'x')$q$, 'gone', 'missing seek is gone');
select (public.server_accept_seek('s3cret', :'challenge_id', 'bbqnnrkr/pppppppp/8/8/8/8/PPPPPPPP/BBQNNRKR w FDfd - 0 1')).id as g2 \gset
reset role;
select t.ok((select white_id = :'cara' and black_id = :'dan' and variant = 'chess960' from public.games where id = :'g2'), 'creator chose white and the challenge became a game');
select t.ok((select count(*) from public.seeks) = 0, 'challenge consumed');

-- ============================================================ moves
select id as g1 from public.games where id <> :'g2' \gset
select white_id as w1, black_id as b1 from public.games where id = :'g1' \gset
set role authenticated; select t.as_user(:'w1');
select t.throws(format($q$select * from public.server_move('s3cret', %L, 'e2e4', 5)$q$, :'g1'), 'stale', 'wrong expected ply is stale');
select t.throws(format($q$select * from public.server_move('s3cret', %L, 'e2-e4', 0)$q$, :'g1'), 'bad_move', 'malformed move rejected');
select t.as_user(:'b1');
select t.throws(format($q$select * from public.server_move('s3cret', %L, 'e7e5', 0)$q$, :'g1'), 'not_your_turn', 'black cannot open');
select t.as_user(:'cara');
select t.throws(format($q$select * from public.server_move('s3cret', %L, 'e2e4', 0)$q$, :'g1'), 'not_a_player', 'spectators cannot move');
select t.as_user(:'w1');
select t.ok((select moves = '{e2e4}' from public.server_move('s3cret', :'g1', 'e2e4', 0)), 'white plays e2e4');
select t.throws(format($q$select * from public.server_move('s3cret', %L, 'd2d4', 1)$q$, :'g1'), 'not_your_turn', 'white cannot move twice');
select t.as_user(:'b1');
select t.ok((select moves = '{e2e4,e7e5}' from public.server_move('s3cret', :'g1', 'e7e5', 1)), 'black replies');
reset role;
select t.ok((select white_clock_ms = 180000 and black_clock_ms = 180000 from public.games where id = :'g1'), 'no time is charged for the first move of each side');

-- clocks: pretend white has been thinking for 10 s
update public.games set clock_updated_at = clock_timestamp() - interval '10 seconds' where id = :'g1';
set role authenticated; select t.as_user(:'w1');
select t.ok((select white_clock_ms between 171000 and 172100 from public.server_move('s3cret', :'g1', 'g1f3', 2)), 'white is charged ~10s and gets the 2s increment (180-10+2)');
reset role;
select t.ok((select black_clock_ms = 180000 from public.games where id = :'g1'), 'the other clock is untouched');

-- flag falls before the move arrives: the move does not count
update public.games set black_clock_ms = 1000, clock_updated_at = clock_timestamp() - interval '5 seconds' where id = :'g1';
set role authenticated; select t.as_user(:'b1');
select t.ok((select status = 'finished' and result = case when :'b1' = white_id::text then '0-1' else '1-0' end and reason = 'timeout' and cardinality(moves) = 3 from public.server_move('s3cret', :'g1', 'b8c6', 3)), 'a late move loses on time and is not recorded');
select t.throws(format($q$select * from public.server_move('s3cret', %L, 'g8f6', 3)$q$, :'g1'), 'finished', 'no moves after the game ends');
reset role;

-- terminal move supplied by the server

-- fixture helper: a fresh started game with chosen players (superuser only)
create function t.new_game(w uuid, b uuid, tm int default 180000, inc int default 0, moves text[] default '{}') returns uuid language sql as $$
  insert into public.games (variant, initial_fen, white_id, black_id, time_initial_ms, time_inc_ms, white_clock_ms, black_clock_ms, moves)
  values ('standard', 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1', w, b, tm, inc, tm, tm, moves) returning id $$;

-- ============================================================ terminal moves
select t.new_game(:'alice', :'bob', moves => '{f2f3,e7e5,g2g4}') as gm \gset
set role authenticated; select t.as_user(:'bob');
select t.ok((select status = 'finished' and result = '0-1' and reason = 'checkmate' and moves[4] = 'd8h4' from public.server_move('s3cret', :'gm', 'd8h4', 3, '0-1', 'checkmate')), 'server-supplied checkmate finishes the game');
select t.throws(format($q$select * from public.server_move('s3cret', %L, 'a2a3', 4)$q$, :'gm'), 'finished', 'finished games are frozen');
select t.throws(format($q$select * from public.server_move('s3cret', %L, 'a7a6', 3, 'bogus', null)$q$, :'gm'), 'finished', 'finished stays finished');
reset role;
select t.ok((select ended_at is not null from public.games where id = :'gm'), 'end time recorded');

-- ============================================================ resign and abort
select t.new_game(:'alice', :'bob') as ga \gset
set role authenticated; select t.as_user(:'alice');
select t.ok((select status = 'aborted' and result is null from public.server_resign('s3cret', :'ga')), 'resigning before two moves aborts instead');
reset role;
select t.new_game(:'alice', :'bob', moves => '{e2e4,e7e5,d2d4}') as gb \gset
set role authenticated; select t.as_user(:'cara');
select t.throws(format($q$select * from public.server_resign('s3cret', %L)$q$, :'gb'), 'not_a_player', 'spectators cannot resign anyone');
select t.as_user(:'bob');
select t.ok((select status = 'finished' and result = '1-0' and reason = 'resignation' from public.server_resign('s3cret', :'gb')), 'black resigns, white wins');
reset role;
select t.new_game(:'alice', :'bob', moves => '{e2e4,e7e5}') as gc \gset
set role authenticated; select t.as_user(:'alice');
select t.throws(format($q$select * from public.server_abort('s3cret', %L)$q$, :'gc'), 'too_late_to_abort', 'cannot abort after both sides moved');
reset role;
select t.new_game(:'alice', :'bob', moves => '{e2e4}') as gd \gset
set role authenticated; select t.as_user(:'bob');
select t.ok((select status = 'aborted' from public.server_abort('s3cret', :'gd')), 'either player can abort early');
reset role;

-- ============================================================ draw offers
select t.new_game(:'alice', :'bob', moves => '{e2e4,e7e5,g1f3,b8c6}') as ge \gset
set role authenticated; select t.as_user(:'alice');
select t.ok((select draw_offer_by = 'white' from public.server_draw('s3cret', :'ge', 'offer')), 'white offers a draw');
select t.throws(format($q$select * from public.server_draw('s3cret', %L, 'accept')$q$, :'ge'), 'no_offer', 'cannot accept your own offer');
select t.ok((select draw_offer_by is null from public.server_draw('s3cret', :'ge', 'cancel')), 'offer can be withdrawn');
select t.ok((select draw_offer_by = 'white' from public.server_draw('s3cret', :'ge', 'offer')), 'offer again');
select t.as_user(:'bob');
select t.ok((select draw_offer_by is null from public.server_draw('s3cret', :'ge', 'decline')), 'black declines');
select t.as_user(:'alice');
select t.ok((select draw_offer_by = 'white' from public.server_draw('s3cret', :'ge', 'offer')), 'white offers once more');
select t.as_user(:'bob');
select t.ok((select status = 'finished' and result = '1/2-1/2' and reason = 'agreement' from public.server_draw('s3cret', :'ge', 'accept')), 'black accepts: draw');
reset role;
select t.new_game(:'alice', :'bob', moves => '{e2e4,e7e5,g1f3,b8c6}') as gf \gset
set role authenticated; select t.as_user(:'alice');
select t.ok((select draw_offer_by = 'white' from public.server_draw('s3cret', :'gf', 'offer')), 'offer made');
select t.as_user(:'bob');
select t.ok((select status = 'finished' and reason = 'agreement' from public.server_draw('s3cret', :'gf', 'offer')), 'crossing offers make a draw');
reset role;
select t.new_game(:'alice', :'bob', moves => '{e2e4,e7e5,g1f3,b8c6}') as gg \gset
set role authenticated; select t.as_user(:'alice');
select t.ok((select draw_offer_by = 'white' from public.server_draw('s3cret', :'gg', 'offer')), 'offer before a move');
select t.ok((select draw_offer_by is null from public.server_move('s3cret', :'gg', 'f1c4', 4)), 'making a move withdraws your own pending offer');
select t.throws(format($q$select * from public.server_draw('s3cret', %L, 'offer')$q$, :'ge'), 'finished', 'no offers in finished games');
reset role;
select t.new_game(:'alice', :'bob', moves => '{e2e4}') as gh \gset
set role authenticated; select t.as_user(:'alice');
select t.throws(format($q$select * from public.server_draw('s3cret', %L, 'offer')$q$, :'gh'), 'too_early', 'no draw offers before both have moved');
reset role;

-- ============================================================ timeouts
select t.new_game(:'alice', :'bob', moves => '{e2e4,e7e5}') as gi \gset
set role authenticated; select t.as_user(:'cara');
select t.ok((select status = 'started' from public.server_claim_timeout('s3cret', :'gi')), 'a healthy clock is left alone');
reset role;
update public.games set white_clock_ms = 2000, clock_updated_at = clock_timestamp() - interval '3 seconds' where id = :'gi';
set role authenticated; select t.as_user(:'cara');
select t.ok((select status = 'finished' and result = '0-1' and reason = 'timeout' and white_clock_ms = 0 from public.server_claim_timeout('s3cret', :'gi')), 'a bystander can claim: white ran out of time');
reset role;
select t.new_game(:'alice', :'bob', moves => '{e2e4,e7e5,g1f3}') as gj \gset
update public.games set black_clock_ms = 100, clock_updated_at = clock_timestamp() - interval '1 second' where id = :'gj';
set role authenticated; select t.as_user(:'alice');
select t.ok((select status = 'finished' and result = '1/2-1/2' and reason = 'timeout' from public.server_claim_timeout('s3cret', :'gj', true)), 'timeout against a side with no mating material is a draw');
reset role;
select t.new_game(:'alice', :'bob') as gk \gset
update public.games set clock_updated_at = clock_timestamp() - interval '30 seconds' where id = :'gk';
set role authenticated; select t.as_user(:'alice');
select t.ok((select status = 'started' from public.server_claim_timeout('s3cret', :'gk')), 'still waiting for the first move after 30s');
reset role;
update public.games set clock_updated_at = clock_timestamp() - interval '61 seconds' where id = :'gk';
set role authenticated; select t.as_user(:'alice');
select t.ok((select status = 'aborted' and reason = 'abandoned' from public.server_claim_timeout('s3cret', :'gk')), 'abandoned before the first move: aborted, no result');
reset role;

-- ============================================================ bans
select t.new_game(:'alice', :'bob', moves => '{e2e4,e7e5}') as gl \gset
update public.profiles set banned_until = now() + interval '1 day' where id = :'alice';
set role authenticated; select t.as_user(:'alice');
select t.throws(format($q$select * from public.server_move('s3cret', %L, 'g1f3', 2)$q$, :'gl'), 'banned', 'banned players cannot move');
select t.throws($q$select public.server_quick_pair('s3cret', 'standard', 60000, 0, 'x')$q$, 'banned', 'banned players cannot queue');
select t.as_user(null);
select t.throws(format($q$select * from public.server_move('s3cret', %L, 'g1f3', 2)$q$, :'gl'), 'not_signed_in', 'anonymous callers are refused');
reset role;

-- ============================================================ readable by everyone
set role anon; select t.as_user(null);
select t.ok((select count(*) from public.games) > 5, 'spectators (even signed out) can read games');
reset role;

\echo ALL LIVE-GAME TESTS PASSED
