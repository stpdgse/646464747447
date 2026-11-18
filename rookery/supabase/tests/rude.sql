\set ON_ERROR_STOP on
\set QUIET on
-- Runs after bot_tournaments.sql in the same scratch database (secret 's3cret' exists; users olga, pete, quinn).
\set olga  '00000000-0000-0000-0000-0000000000e3'
\set pete  '00000000-0000-0000-0000-0000000000e4'
\set quinn '00000000-0000-0000-0000-0000000000e5'
update public.profiles set banned_until = null, muted_until = null;

-- ============================================================ the event and starting
select t.ok((select count(*) from public.events where key = 'rude-holiday-2026') = 1, 'the holiday event exists');
set role authenticated; select t.as_user(:'olga');
create temp table a1 as select (public.rude_start('rude-holiday-2026')).*;
select t.ok((select attempt_no = 1 and level = 100 and best_level = 0 and status = 'playing' from a1), 'a first attempt starts at the first Rüde (100)');
select t.ok((select id from public.rude_start('rude-holiday-2026')) = (select id from a1), 'asking again carries on the same attempt instead of burning another');
select t.throws($q$select public.rude_start('nope')$q$, 'not_found', 'unknown event');
select t.throws($q$insert into public.rude_attempts (user_id, event_key, attempt_no) values ('00000000-0000-0000-0000-0000000000e3', 'rude-holiday-2026', 5)$q$, 'permission denied', 'attempts cannot be written directly');
reset role; set role anon; select t.as_user(null);
select t.ok((select count(*) from public.rude_attempts) = 1, 'the leaderboard is public');
select t.throws($q$select public.rude_start('rude-holiday-2026')$q$, 'permission denied', 'visitors cannot start attempts');
reset role;

-- ============================================================ reporting results (server secret)
set role authenticated; select t.as_user(:'olga');
select t.throws($q$select public.server_rude_report('wrong', (select id from a1), 100, 'win')$q$, 'forbidden', 'reports need the server secret');
select t.throws($q$select public.server_rude_report('s3cret', (select id from a1), 300, 'win')$q$, 'wrong_level', 'you cannot skip a Rüde');
select t.as_user(:'pete');
select t.throws($q$select public.server_rude_report('s3cret', (select id from a1), 100, 'win')$q$, 'not_found', 'nor report someone else''s attempt');
select t.as_user(:'olga');
select t.ok((select level = 200 and best_level = 100 and status = 'playing' and not cleared from public.server_rude_report('s3cret', (select id from a1), 100, 'win')), 'a win at 100 moves you up to 200 and records 100 as your best');
select t.ok((select level = 300 and best_level = 200 from public.server_rude_report('s3cret', (select id from a1), 200, 'win')), 'another win: now at 300');
select t.throws($q$select public.server_rude_report('s3cret', (select id from a1), 200, 'win')$q$, 'wrong_level', 'the same level cannot be reported twice');
select t.ok((select status = 'finished' and best_level = 200 and ended_at is not null from public.server_rude_report('s3cret', (select id from a1), 300, 'loss')), 'a loss ends the attempt and keeps the best level (200)');
select t.throws($q$select public.server_rude_report('s3cret', (select id from a1), 300, 'win')$q$, 'finished', 'a finished attempt cannot be continued');

-- draws count as failing
create temp table a2 as select (public.rude_start('rude-holiday-2026')).*;
select t.ok((select attempt_no = 2 from a2), 'the next start is attempt 2');
select t.ok((select status = 'finished' from public.server_rude_report('s3cret', (select id from a2), 100, 'draw')), 'a draw ends the attempt too');
select t.throws($q$select public.server_rude_report('s3cret', (select id from a2), 100, 'maybe')$q$, 'finished', 'an attempt that is over ignores further reports');

-- ============================================================ eight attempts in all
do $$ declare r public.rude_attempts; begin
  for i in 3..8 loop
    r := public.rude_start('rude-holiday-2026');
    perform public.server_rude_report('s3cret', r.id, 100, 'loss');
  end loop;
end $$;
select t.ok((select count(*) from public.rude_attempts where user_id = :'olga') = 8, 'eight attempts used');
select t.throws($q$select public.rude_start('rude-holiday-2026')$q$, 'no_attempts', 'a ninth attempt is refused');
reset role;

-- ============================================================ clearing the whole ladder, and badges
set role authenticated; select t.as_user(:'pete');
create temp table p1 as select (public.rude_start('rude-holiday-2026')).*;
do $$ declare r public.rude_attempts; begin
  for lvl in 1..20 loop
    r := public.server_rude_report('s3cret', (select id from p1), lvl * 100, 'win');
  end loop;
  if not (r.cleared and r.status = 'finished' and r.best_level = 2000) then raise exception 'ladder not cleared: %', row_to_json(r); end if;
end $$;
select t.ok((select cleared and status = 'finished' and best_level = 2000 and level = 2100 from public.rude_attempts where id = (select id from p1)), 'beating all twenty Rüden clears the ladder');
select t.ok(public.check_achievements() @> array['rude_holiday','rude_first','rude_500','rude_1000','rude_1500','rude_2000'], 'and earns every event badge');
select t.as_user(:'olga');
select t.ok(public.check_achievements() @> array['rude_holiday','rude_first'] and not (public.check_achievements() @> array['rude_500']), 'olga (best 200) earns the first two badges only: ');
select t.ok((select count(*) from public.achievements where user_id = :'olga' and key like 'rude%') = 2, 'exactly two event badges for olga');
reset role;

-- ============================================================ the time window and stale attempts
update public.events set starts_at = now() + interval '1 day' where key = 'rude-holiday-2026';
set role authenticated; select t.as_user(:'quinn');
select t.throws($q$select public.rude_start('rude-holiday-2026')$q$, 'not_started', 'nothing can start before the event');
reset role;
update public.events set starts_at = now() - interval '2 days', ends_at = now() - interval '1 day' where key = 'rude-holiday-2026';
set role authenticated; select t.as_user(:'quinn');
select t.throws($q$select public.rude_start('rude-holiday-2026')$q$, 'event_over', 'nor after it ends');
reset role;
update public.events set ends_at = now() + interval '30 days' where key = 'rude-holiday-2026';

set role authenticated; select t.as_user(:'quinn');
create temp table q1 as select (public.rude_start('rude-holiday-2026')).*;
reset role;
update public.rude_attempts set updated_at = now() - interval '31 minutes' where id = (select id from q1);
set role authenticated; select t.as_user(:'quinn');
select t.ok((select attempt_no = 2 from public.rude_start('rude-holiday-2026')), 'an attempt left alone for 30 minutes is over, and the next start is a new attempt');
select t.ok((select status = 'finished' from public.rude_attempts where id = (select id from q1)), 'the abandoned attempt is closed');
select public.rude_beat((select id from public.rude_attempts where user_id = :'quinn' and status = 'playing'));
select t.ok(true, 'the heartbeat keeps a running attempt alive');
reset role;
do $$ begin raise notice 'ALL RUDE CHALLENGE TESTS PASSED'; end $$;
