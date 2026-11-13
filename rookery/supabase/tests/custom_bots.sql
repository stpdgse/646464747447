\set ON_ERROR_STOP on
\set QUIET on
-- Runs after bot_games.sql in the same scratch database (users: mia = admin, noah = moderator, olga = player).
\set mia  '00000000-0000-0000-0000-0000000000e1'
\set noah '00000000-0000-0000-0000-0000000000e2'
\set olga '00000000-0000-0000-0000-0000000000e3'

set role authenticated; select t.as_user(:'mia');
insert into public.custom_bots (id, name, rating, smartness, style, blurb) values ('testbot', 'Test Bot', 1234, 12, 'chaotic', 'made by an admin');
select t.ok(true, 'an admin can add a bot');
update public.custom_bots set rating = 1300, hidden = true where id = 'testbot';
select t.ok((select rating from public.custom_bots where id = 'testbot') = 1300, 'and edit it');
insert into public.custom_bots (id, name, rating, greeting) values ('alf', 'Pong Alf Femboy', 700, 'edited greeting');
select t.ok(true, 'an admin can override a built-in bot (same id)');
select t.throws($q$insert into public.custom_bots (id, name, rating, smartness) values ('bad id!', 'x', 1, 1)$q$, 'check constraint', 'ids must be simple slugs');
select t.throws($q$insert into public.custom_bots (id, name, rating, smartness) values ('toosmart', 'x', 1, 25)$q$, 'check constraint', 'smartness stops at 24');
select t.throws($q$insert into public.custom_bots (id, name, rating, avatar_image) values ('evil', 'x', 1, 'data:text/html;base64,PHNjcmlwdD4=')$q$, 'check constraint', 'only real image data urls are accepted');
insert into public.custom_bots (id, name, rating, avatar_image) values ('pic', 'Pic', 1, 'data:image/png;base64,iVBORw0KGgo=');
select t.ok(true, 'a small png picture is accepted');
select t.ok((select count(*) from public.mod_actions where action like 'bot_%') >= 3, 'bot changes are in the audit log');

select t.as_user(:'noah');
select t.throws($q$insert into public.custom_bots (id, name, rating) values ('modbot', 'x', 1)$q$, 'row-level security', 'moderators cannot add bots');
select t.as_user(:'olga');
select t.throws($q$insert into public.custom_bots (id, name, rating) values ('playerbot', 'x', 1)$q$, 'row-level security', 'players cannot add bots');
update public.custom_bots set name = 'hacked' where id = 'testbot';
select t.ok((select name from public.custom_bots where id = 'testbot') = 'Test Bot', 'players cannot edit bots');
select t.ok((select count(*) from public.custom_bots) = 3, 'but everyone can read them');
reset role; set role anon; select t.as_user(null);
select t.ok((select count(*) from public.custom_bots) = 3, 'including signed-out visitors');
reset role;

set role authenticated; select t.as_user(:'olga');
delete from public.custom_bots where id = 'testbot';
select t.ok((select count(*) from public.custom_bots where id = 'testbot') = 1, 'players cannot delete bots');
select t.as_user(:'mia');
delete from public.custom_bots where id = 'testbot';
select t.ok((select count(*) from public.custom_bots where id = 'testbot') = 0, 'an admin can delete a bot');
reset role;
do $$ begin raise notice 'ALL CUSTOM BOT TESTS PASSED'; end $$;
