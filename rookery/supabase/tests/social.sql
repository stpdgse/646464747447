\set ON_ERROR_STOP on
\set QUIET on
-- Runs after openings.sql in the same scratch database (reuses the t.* helpers).

insert into auth.users (id, raw_user_meta_data) values
  ('00000000-0000-0000-0000-0000000000e1', '{"username":"mia"}'),    -- admin
  ('00000000-0000-0000-0000-0000000000e2', '{"username":"noah"}'),   -- moderator
  ('00000000-0000-0000-0000-0000000000e3', '{"username":"olga"}'),
  ('00000000-0000-0000-0000-0000000000e4', '{"username":"pete"}'),
  ('00000000-0000-0000-0000-0000000000e5', '{"username":"quinn"}'),
  ('00000000-0000-0000-0000-0000000000e6', '{"username":"rex"}');
\set mia   '00000000-0000-0000-0000-0000000000e1'
\set noah  '00000000-0000-0000-0000-0000000000e2'
\set olga  '00000000-0000-0000-0000-0000000000e3'
\set pete  '00000000-0000-0000-0000-0000000000e4'
\set quinn '00000000-0000-0000-0000-0000000000e5'
\set rex   '00000000-0000-0000-0000-0000000000e6'
update public.profiles set is_admin = true where id = :'mia';
update public.profiles set is_moderator = true where id = :'noah';

-- ============================================================ roles and helpers
set role authenticated; select t.as_user(:'olga');
select t.ok(not public.is_mod(), 'players are not moderators');
select t.ok(public.can_post(), 'players can post by default');
select t.as_user(:'noah'); select t.ok(public.is_mod(), 'moderators are moderators');
select t.as_user(:'mia'); select t.ok(public.is_mod(), 'admins are moderators too');
reset role; set role anon; select t.as_user(null);
select t.ok(not public.is_mod() and not public.can_post(), 'signed-out visitors are neither');
reset role;
set role authenticated; select t.as_user(:'olga');
select t.throws($q$update public.profiles set is_moderator = true where id = '00000000-0000-0000-0000-0000000000e3'$q$, 'permission denied', 'players cannot promote themselves');
select t.throws($q$update public.profiles set muted_until = null where id = '00000000-0000-0000-0000-0000000000e3'$q$, 'permission denied', 'players cannot edit their own mute');
reset role;

-- ============================================================ muting, banning, roles
set role authenticated; select t.as_user(:'olga');
select t.throws($q$select public.mod_mute('00000000-0000-0000-0000-0000000000e4', 1, 'x')$q$, 'forbidden', 'players cannot mute');
select t.throws($q$select public.mod_ban('00000000-0000-0000-0000-0000000000e4', 1, 'x')$q$, 'forbidden', 'players cannot ban');
select t.as_user(:'noah');
select public.mod_mute(:'olga', 24, 'spam');
reset role;
select t.ok((select muted_until between now() + interval '23 hours' and now() + interval '25 hours' from public.profiles where id = :'olga'), 'a moderator can mute for a set time');
select t.ok((select count(*) = 1 from public.mod_actions where action = 'mute' and target_user = :'olga' and actor_id = :'noah'), 'the mute is in the audit log');
set role authenticated; select t.as_user(:'olga');
select t.ok(not public.can_post(), 'a muted player cannot post');
select t.ok((select count(*) = 0 from public.mod_actions), 'players cannot read the audit log');
select t.as_user(:'noah');
select t.throws($q$select public.mod_mute('00000000-0000-0000-0000-0000000000e1', 1, 'x')$q$, 'forbidden_target', 'moderators cannot mute admins');
select t.throws($q$select public.mod_mute('00000000-0000-0000-0000-0000000000e2', 1, 'x')$q$, 'forbidden_target', 'moderators cannot mute moderators');
select t.throws($q$select public.mod_ban('00000000-0000-0000-0000-0000000000e3', 1, 'x')$q$, 'forbidden', 'only admins can ban');
select public.mod_mute(:'olga', 0, 'sorry');
select t.ok((select count(*) > 0 from public.mod_actions), 'moderators can read the audit log');
select t.throws($q$insert into public.mod_actions (actor_id, action) values ('00000000-0000-0000-0000-0000000000e2', 'x')$q$, 'permission denied', 'nobody can write the audit log directly');
reset role;
select t.ok((select muted_until is null from public.profiles where id = :'olga'), 'unmuting clears it');
set role authenticated; select t.as_user(:'mia');
select public.mod_mute(:'noah', 1, 'test'); select public.mod_mute(:'noah', 0, 'done');
select public.mod_ban(:'pete', 7, 'cheating');
reset role;
select t.ok((select banned_until > now() + interval '6 days' from public.profiles where id = :'pete'), 'an admin can ban');
set role authenticated; select t.as_user(:'pete');
select t.ok(not public.can_post(), 'a banned player cannot post');
select t.throws($q$select public.server_quick_pair('s3cret', 'standard', 180000, 0, 'x', false)$q$, 'banned', 'a banned player cannot start games');
select t.as_user(:'mia'); select public.mod_ban(:'pete', 0, 'appeal accepted');
select t.throws($q$select public.mod_ban('00000000-0000-0000-0000-0000000000e1', 1, 'x')$q$, 'forbidden_target', 'admins cannot be banned');
select t.as_user(:'noah');
select t.throws($q$select public.admin_set_moderator('00000000-0000-0000-0000-0000000000e4', true)$q$, 'forbidden', 'only admins grant moderator');
select t.as_user(:'mia'); select public.admin_set_moderator(:'quinn', true);
reset role;
select t.ok((select is_moderator from public.profiles where id = :'quinn'), 'an admin can make a moderator');
set role authenticated; select t.as_user(:'mia'); select public.admin_set_moderator(:'quinn', false); reset role;
select t.ok(not (select is_moderator from public.profiles where id = :'quinn'), 'and remove them again');

-- ============================================================ friends
set role authenticated; select t.as_user(:'olga');
insert into public.friendships (requester_id, addressee_id) values (:'olga', :'pete');
select t.as_user(:'pete');
select t.throws($q$insert into public.friendships (requester_id, addressee_id) values ('00000000-0000-0000-0000-0000000000e4', '00000000-0000-0000-0000-0000000000e3')$q$, 'duplicate key', 'a pair can only have one friendship, in either direction');
select t.as_user(:'olga');
select t.throws($q$insert into public.friendships (requester_id, addressee_id) values ('00000000-0000-0000-0000-0000000000e3', '00000000-0000-0000-0000-0000000000e3')$q$, 'violates check', 'no friending yourself');
select t.throws($q$insert into public.friendships (requester_id, addressee_id) values ('00000000-0000-0000-0000-0000000000e4', '00000000-0000-0000-0000-0000000000e5')$q$, 'row-level security', 'you cannot send requests as someone else');
select t.throws($q$insert into public.friendships (requester_id, addressee_id, status) values ('00000000-0000-0000-0000-0000000000e3', '00000000-0000-0000-0000-0000000000e5', 'accepted')$q$, 'permission denied', 'you cannot create an already-accepted friendship');
with u as (update public.friendships set status = 'accepted' where requester_id = :'olga' returning 1) select t.ok((select count(*) = 0 from u), 'the requester cannot accept their own request');
select t.as_user(:'quinn');
select t.ok((select count(*) = 0 from public.friendships), 'strangers cannot see a request');
reset role;
select t.ok((select count(*) = 1 from public.notifications where user_id = :'pete' and kind = 'friend_request' and actor_id = :'olga'), 'the addressee is notified of the request');
set role authenticated; select t.as_user(:'pete');
with u as (update public.friendships set status = 'accepted' where requester_id = :'olga' returning 1) select t.ok((select count(*) = 1 from u), 'the addressee can accept');
select t.ok((select status = 'accepted' and responded_at is not null from public.friendships where requester_id = :'olga'), 'accepted, with a response time');
select t.throws($q$update public.friendships set requester_id = '00000000-0000-0000-0000-0000000000e5'$q$, 'permission denied', 'nothing but the status can be edited');
reset role;
select t.ok((select count(*) = 1 from public.notifications where user_id = :'olga' and kind = 'friend_accepted' and actor_id = :'pete'), 'the requester is told it was accepted');

-- presence is for friends only
set role authenticated; select t.as_user(:'olga');
insert into public.presence (user_id) values (:'olga') on conflict (user_id) do update set last_seen_at = now();
select t.ok((select count(*) = 1 from public.presence), 'you can record your own presence');
select t.throws($q$insert into public.presence (user_id) values ('00000000-0000-0000-0000-0000000000e4')$q$, 'row-level security', 'but not someone elses');
select t.as_user(:'pete'); select t.ok((select count(*) = 1 from public.presence where user_id = :'olga'), 'friends can see each others presence');
select t.as_user(:'quinn'); select t.ok((select count(*) = 0 from public.presence), 'strangers cannot');
select t.as_user(:'quinn');
insert into public.friendships (requester_id, addressee_id) values (:'quinn', :'olga');
select t.as_user(:'olga'); select t.ok((select count(*) = 1 from public.presence where user_id = :'olga'), 'a pending request does not reveal presence');
select t.as_user(:'quinn'); select t.ok((select count(*) = 0 from public.presence where user_id = :'olga'), 'pending is not friendship');
select t.as_user(:'olga');
delete from public.friendships where requester_id = :'quinn';
reset role;
select t.ok((select count(*) = 0 from public.friendships where requester_id = :'quinn'), 'the addressee can decline');
set role authenticated; select t.as_user(:'quinn');
delete from public.friendships where requester_id = :'olga';
reset role;
select t.ok((select count(*) = 1 from public.friendships where requester_id = :'olga'), 'strangers cannot remove other peoples friendships');
set role authenticated; select t.as_user(:'pete');
delete from public.friendships where requester_id = :'olga';
reset role;
select t.ok((select count(*) = 0 from public.friendships where requester_id = :'olga'), 'either friend can unfriend');

-- blocks
set role authenticated; select t.as_user(:'quinn');
insert into public.blocks (blocker_id, blocked_id) values (:'quinn', :'olga');
select t.throws($q$insert into public.blocks (blocker_id, blocked_id) values ('00000000-0000-0000-0000-0000000000e3', '00000000-0000-0000-0000-0000000000e5')$q$, 'row-level security', 'you cannot block on behalf of others');
select t.as_user(:'olga');
select t.ok((select count(*) = 0 from public.blocks), 'blocks are private');
select t.throws($q$insert into public.friendships (requester_id, addressee_id) values ('00000000-0000-0000-0000-0000000000e3', '00000000-0000-0000-0000-0000000000e5')$q$, 'row-level security', 'a blocked player cannot send a friend request');
select t.throws($q$insert into public.seeks (user_id, target_id, variant, time_initial_ms) values ('00000000-0000-0000-0000-0000000000e3', '00000000-0000-0000-0000-0000000000e5', 'standard', 180000)$q$, 'row-level security', 'or a challenge');
select t.as_user(:'quinn');
select t.throws($q$insert into public.friendships (requester_id, addressee_id) values ('00000000-0000-0000-0000-0000000000e5', '00000000-0000-0000-0000-0000000000e3')$q$, 'row-level security', 'the blocker cannot request them either (mutual)');
delete from public.blocks where blocked_id = :'olga';
select t.as_user(:'olga');
insert into public.friendships (requester_id, addressee_id) values (:'olga', :'quinn');
delete from public.friendships where requester_id = :'olga' and addressee_id = :'quinn';
reset role;
select t.ok(true, 'after unblocking, requests work again');

-- request rate limit (fresh user so earlier rows do not count)
insert into auth.users (id, raw_user_meta_data) select ('00000000-0000-0000-0001-' || lpad(g::text, 12, '0'))::uuid, jsonb_build_object('username', 'bot' || g) from generate_series(1, 22) g;
set role authenticated; select t.as_user(:'rex');
do $$ declare i int; begin for i in 1..20 loop insert into public.friendships (requester_id, addressee_id) values ('00000000-0000-0000-0000-0000000000e6', ('00000000-0000-0000-0001-' || lpad(i::text, 12, '0'))::uuid); end loop; end $$;
select t.throws($q$insert into public.friendships (requester_id, addressee_id) values ('00000000-0000-0000-0000-0000000000e6', '00000000-0000-0000-0001-000000000021')$q$, 'slow_down', 'at most 20 friend requests an hour');
reset role;

-- ============================================================ game chat
select t.new_game(:'olga', :'pete', moves => '{e2e4,e7e5}') as cg \gset
set role authenticated; select t.as_user(:'olga');
insert into public.game_messages (game_id, user_id, room, body) values (:'cg', :'olga', 'players', 'good luck');
select t.throws(format($q$insert into public.game_messages (game_id, user_id, room, body) values (%L, %L, 'spectators', 'hi')$q$, :'cg', :'olga'), 'row-level security', 'players cannot post in the spectator room');
select t.throws(format($q$insert into public.game_messages (game_id, user_id, room, body) values (%L, %L, 'players', 'imposter')$q$, :'cg', :'pete'), 'row-level security', 'you cannot post as someone else');
select t.throws(format($q$insert into public.game_messages (game_id, user_id, room, body) values (%L, %L, 'players', '')$q$, :'cg', :'olga'), 'violates check', 'empty messages are refused');
select t.throws(format($q$insert into public.game_messages (game_id, user_id, room, body) values (%L, %L, 'players', %L)$q$, :'cg', :'olga', repeat('x', 201)), 'violates check', 'messages are capped at 200 characters');
select t.as_user(:'quinn');
select t.throws(format($q$insert into public.game_messages (game_id, user_id, room, body) values (%L, %L, 'players', 'psst')$q$, :'cg', :'quinn'), 'row-level security', 'spectators cannot post in the players room');
insert into public.game_messages (game_id, user_id, room, body) values (:'cg', :'quinn', 'spectators', 'nice game');
select t.ok((select count(*) = 1 and bool_and(room = 'spectators') from public.game_messages where game_id = :'cg'), 'spectators see only the spectator room');
select t.as_user(:'pete');
select t.ok((select count(*) = 1 and bool_and(room = 'players') from public.game_messages where game_id = :'cg'), 'players see only the players room');
select t.as_user(:'noah');
select t.ok((select count(*) = 2 from public.game_messages where game_id = :'cg'), 'moderators see both rooms');
select id as bad_msg from public.game_messages where user_id = :'quinn' and game_id = :'cg' \gset
select public.mod_hide_message('game', :'bad_msg', true);
select t.as_user(:'mia'); select t.ok((select count(*) = 2 from public.game_messages where game_id = :'cg'), 'moderators still see hidden messages');
select t.as_user(:'quinn'); select t.ok((select count(*) = 0 from public.game_messages where game_id = :'cg'), 'hidden messages disappear for everyone else');
select t.throws(format($q$select public.mod_hide_message('game', %L, true)$q$, :'bad_msg'), 'forbidden', 'players cannot hide messages');
select t.throws($q$update public.game_messages set hidden = false$q$, 'permission denied', 'or un-hide them');
reset role;
select t.ok((select count(*) = 1 from public.mod_actions where action = 'hide_message' and target_user = :'quinn'), 'hiding is audited');
-- blocking hides someones messages from you
set role authenticated; select t.as_user(:'olga');
insert into public.blocks (blocker_id, blocked_id) values (:'olga', :'pete');
select t.as_user(:'pete'); insert into public.game_messages (game_id, user_id, room, body) values (:'cg', :'pete', 'players', 'hello');
select t.as_user(:'olga'); select t.ok((select count(*) = 1 and bool_and(user_id = :'olga') from public.game_messages where game_id = :'cg'), 'a blocker no longer sees the blocked players messages');
select t.as_user(:'pete'); select t.ok((select count(*) = 2 from public.game_messages where game_id = :'cg'), 'the blocked player still sees the room');
select t.as_user(:'olga'); delete from public.blocks where blocker_id = :'olga';
reset role;
-- muted players cannot chat, and chat is rate limited
set role authenticated; select t.as_user(:'noah'); select public.mod_mute(:'pete', 1, 'test'); select t.as_user(:'pete');
select t.throws(format($q$insert into public.game_messages (game_id, user_id, room, body) values (%L, %L, 'players', 'still here')$q$, :'cg', :'pete'), 'row-level security', 'muted players cannot chat');
select t.as_user(:'noah'); select public.mod_mute(:'pete', 0, 'done'); reset role;
delete from public.game_messages where user_id = :'olga';
set role authenticated; select t.as_user(:'olga');
do $$ declare i int; begin for i in 1..8 loop insert into public.game_messages (game_id, user_id, room, body) values ((select id from public.games where white_id = '00000000-0000-0000-0000-0000000000e3' and black_id = '00000000-0000-0000-0000-0000000000e4' order by created_at desc limit 1), '00000000-0000-0000-0000-0000000000e3', 'players', 'm' || i); end loop; end $$;
select t.throws(format($q$insert into public.game_messages (game_id, user_id, room, body) values (%L, %L, 'players', 'one too many')$q$, :'cg', :'olga'), 'slow_down', 'at most 8 messages per 10 seconds');
reset role;
update public.profiles set muted_until = null;
select t.ok(true, 'rate limits reset');

-- ============================================================ clubs
set role authenticated; select t.as_user(:'olga');
insert into public.clubs (slug, name, description, owner_id) values ('chess-club', 'The Chess Club', 'Weekly games', :'olga');
select t.ok((select count(*) = 1 and bool_and(role = 'owner' and status = 'active') from public.club_members where user_id = :'olga'), 'creating a club makes you its owner member');
select t.throws($q$insert into public.clubs (slug, name, owner_id) values ('Bad Slug', 'Nope', '00000000-0000-0000-0000-0000000000e3')$q$, 'violates check', 'slugs are lowercase words');
select t.throws($q$insert into public.clubs (slug, name, owner_id) values ('short-name', 'ab', '00000000-0000-0000-0000-0000000000e3')$q$, 'violates check', 'names have a minimum length');
select t.throws($q$insert into public.clubs (slug, name, owner_id) values ('someone-elses', 'Someone elses', '00000000-0000-0000-0000-0000000000e4')$q$, 'row-level security', 'you cannot create a club for someone else');
insert into public.clubs (slug, name, owner_id, open) values ('private-club', 'Private Club', :'olga', false);
insert into public.clubs (slug, name, owner_id) values ('third-club', 'Third Club', :'olga');
select t.throws($q$insert into public.clubs (slug, name, owner_id) values ('fourth-club', 'Fourth Club', '00000000-0000-0000-0000-0000000000e3')$q$, 'too_many_clubs', 'at most three clubs per owner');
reset role;
select id as club_open from public.clubs where slug = 'chess-club' \gset
select id as club_closed from public.clubs where slug = 'private-club' \gset

set role authenticated; select t.as_user(:'pete');
insert into public.club_members (club_id, user_id) values (:'club_open', :'pete');
select t.ok((select status = 'active' and role = 'member' from public.club_members where club_id = :'club_open' and user_id = :'pete'), 'anyone can join an open club');
select t.throws(format($q$insert into public.club_members (club_id, user_id, role) values (%L, %L, 'owner')$q$, :'club_closed', :'pete'), 'row-level security', 'you cannot make yourself owner');
select t.throws(format($q$insert into public.club_members (club_id, user_id) values (%L, %L)$q$, :'club_closed', :'pete'), 'row-level security', 'you cannot walk into a closed club');
insert into public.club_members (club_id, user_id, status) values (:'club_closed', :'pete', 'requested');
select t.as_user(:'quinn');
with u as (update public.club_members set status = 'active' where club_id = :'club_closed' and user_id = :'pete' returning 1) select t.ok((select count(*) = 0 from u), 'outsiders cannot approve a request');
select t.ok((select count(*) = 0 from public.club_members where club_id = :'club_closed' and status = 'requested'), 'requests are private to the club team and the requester');
select t.as_user(:'olga');
with u as (update public.club_members set status = 'active' where club_id = :'club_closed' and user_id = :'pete' returning 1) select t.ok((select count(*) = 1 from u), 'the owner approves the request');
-- invites
insert into public.club_members (club_id, user_id, status) values (:'club_closed', :'quinn', 'invited');
reset role;
select t.ok((select count(*) = 1 from public.notifications where user_id = :'quinn' and kind = 'club_invite' and club_id = :'club_closed' and actor_id = :'olga'), 'an invite notifies the player');
set role authenticated; select t.as_user(:'quinn');
select t.throws(format($q$insert into public.club_members (club_id, user_id, status) values (%L, %L, 'invited')$q$, :'club_open', :'rex'), 'row-level security', 'ordinary members cannot invite');
update public.club_members set status = 'active' where club_id = :'club_closed' and user_id = :'quinn';
select t.ok((select count(*) = 1 from public.club_members where club_id = :'club_closed' and user_id = :'quinn' and status = 'active'), 'an invited player can accept');
-- roles
select t.throws(format($q$select public.club_set_role(%L, %L, 'moderator')$q$, :'club_closed', :'pete'), 'forbidden', 'only the owner changes roles');
select t.as_user(:'olga'); select public.club_set_role(:'club_closed', :'pete', 'moderator');
select t.ok((select role = 'moderator' from public.club_members where club_id = :'club_closed' and user_id = :'pete'), 'the owner can appoint a moderator');
select t.as_user(:'pete');
delete from public.club_members where club_id = :'club_closed' and user_id = :'quinn';
reset role;
select t.ok((select count(*) = 0 from public.club_members where club_id = :'club_closed' and user_id = :'quinn'), 'a club moderator can remove a member');
set role authenticated; select t.as_user(:'pete');
delete from public.club_members where club_id = :'club_closed' and user_id = :'olga';
reset role;
select t.ok((select count(*) = 1 from public.club_members where club_id = :'club_closed' and user_id = :'olga'), 'nobody can remove the owner');
-- club chat
set role authenticated; select t.as_user(:'pete');
insert into public.club_messages (club_id, user_id, body) values (:'club_closed', :'pete', 'hello club');
select t.as_user(:'quinn');
select t.ok((select count(*) = 0 from public.club_messages), 'non-members cannot read club chat');
select t.throws(format($q$insert into public.club_messages (club_id, user_id, body) values (%L, %L, 'let me in')$q$, :'club_closed', :'quinn'), 'row-level security', 'or write in it');
select t.as_user(:'olga');
select t.ok((select count(*) = 1 from public.club_messages), 'members read club chat');
select t.as_user(:'noah'); select t.ok((select count(*) = 1 from public.club_messages), 'moderators can read it too');
select id as club_msg from public.club_messages limit 1 \gset
select public.mod_hide_message('club', :'club_msg', true);
select t.as_user(:'olga'); select t.ok((select count(*) = 0 from public.club_messages), 'hidden club messages vanish for members');
-- leaving, editing, archiving
select t.as_user(:'pete'); delete from public.club_members where club_id = :'club_open' and user_id = :'pete'; reset role;
select t.ok((select count(*) = 0 from public.club_members where club_id = :'club_open' and user_id = :'pete'), 'members can leave');
set role authenticated; select t.as_user(:'pete');
with u as (update public.clubs set name = 'Hijacked' where id = :'club_open' returning 1) select t.ok((select count(*) = 0 from u), 'only the owner can edit a club');
select t.as_user(:'olga');
update public.clubs set description = 'Updated description' where id = :'club_open';
select t.throws(format($q$update public.clubs set owner_id = %L where id = %L$q$, :'pete', :'club_open'), 'permission denied', 'ownership cannot be reassigned by an edit');
update public.clubs set archived = true where id = :'club_open';
select t.as_user(:'pete'); select t.ok((select count(*) = 0 from public.clubs where slug = 'chess-club'), 'archived clubs disappear for others');
select t.as_user(:'olga'); select t.ok((select count(*) = 1 from public.clubs where slug = 'chess-club'), 'but not for their owner');
reset role; set role anon; select t.as_user(null);
select t.ok((select count(*) >= 1 from public.clubs), 'signed-out visitors can browse clubs');
select t.throws($q$select count(*) from public.club_messages$q$, 'permission denied', 'but club chat is closed to them entirely');
reset role;

-- ============================================================ forum
select id as cat_general from public.forum_categories where slug = 'general' \gset
select id as cat_news from public.forum_categories where slug = 'announcements' \gset
select t.ok((select count(*) = 5 from public.forum_categories), 'the forum starts with five categories');
set role authenticated; select t.as_user(:'olga');
select public.create_forum_thread(:'cat_general', '  First thread  ', 'Hello forum!') as th \gset
select t.ok((select title = 'First thread' and post_count = 1 from public.forum_threads where id = :'th'), 'a thread and its first post are created together (and tidied)');
select t.throws(format($q$select public.create_forum_thread(%L, 'Official news', 'by a player')$q$, :'cat_news'), 'row-level security', 'players cannot post announcements');
select t.throws(format($q$select public.create_forum_thread(%L, 'ab', 'short title')$q$, :'cat_general'), 'violates check', 'titles need 3+ characters');
select t.as_user(:'pete');
insert into public.forum_posts (thread_id, author_id, body) values (:'th', :'pete', 'Welcome!');
select t.ok((select post_count = 2 from public.forum_threads where id = :'th'), 'replies bump the post count');
select t.throws(format($q$insert into public.forum_posts (thread_id, author_id, body) values (%L, %L, 'as olga')$q$, :'th', :'olga'), 'row-level security', 'you cannot post as someone else');
select t.as_user(:'noah');
select public.create_forum_thread(:'cat_news', 'Welcome to Rookery', 'Be kind.') as news \gset
select t.ok((select count(*) = 1 from public.forum_threads where id = :'news'), 'moderators can post announcements');
select public.mod_forum('thread', :'th', 'lock');
select t.as_user(:'pete');
select t.throws(format($q$insert into public.forum_posts (thread_id, author_id, body) values (%L, %L, 'late reply')$q$, :'th', :'pete'), 'row-level security', 'locked threads refuse replies');
select t.as_user(:'noah'); select public.mod_forum('thread', :'th', 'unlock'); select public.mod_forum('thread', :'th', 'pin');
select t.ok((select pinned and not locked from public.forum_threads where id = :'th'), 'moderators lock, unlock and pin');
select t.as_user(:'pete');
select t.throws(format($q$select public.mod_forum('thread', %L, 'lock')$q$, :'th'), 'forbidden', 'players cannot moderate');
-- editing and deleting
select t.as_user(:'olga');
select id as op from public.forum_posts where thread_id = :'th' and author_id = :'olga' \gset
update public.forum_posts set body = 'Hello forum! (edited)' where id = :'op';
select t.ok((select edited_at is not null from public.forum_posts where id = :'op'), 'editing a post marks it edited');
select t.as_user(:'pete');
with u as (update public.forum_posts set body = 'vandalism' where id = :'op' returning 1) select t.ok((select count(*) = 0 from u), 'you cannot edit someone elses post');
select t.throws(format($q$update public.forum_posts set author_id = %L where id = %L$q$, :'pete', :'op'), 'permission denied', 'or change its author');
insert into public.forum_posts (thread_id, author_id, body) values (:'th', :'pete', 'second reply');
select id as pp from public.forum_posts where thread_id = :'th' and author_id = :'pete' order by created_at desc limit 1 \gset
update public.forum_posts set deleted_at = now() where id = :'pp';
select t.ok((select deleted_at is not null from public.forum_posts where id = :'pp'), 'deleting your own post marks it deleted');
select t.as_user(:'quinn');
select t.ok((select count(*) = 0 from public.forum_posts where id = :'pp'), 'and hides it from everyone else');
select t.as_user(:'noah');
select t.ok((select count(*) = 1 from public.forum_posts where id = :'pp'), 'moderators can still see it');
select public.mod_forum('post', :'op', 'remove');
select t.as_user(:'olga');
select t.ok((select count(*) = 0 from public.forum_posts where id = :'op'), 'a moderator-removed post is hidden');
with u as (update public.forum_posts set deleted_at = null where id = :'op' returning 1) select t.ok((select count(*) = 0 from u), 'and its author cannot bring it back');
select t.as_user(:'noah'); select public.mod_forum('post', :'op', 'restore');
select t.as_user(:'olga'); select t.ok((select count(*) = 1 from public.forum_posts where id = :'op'), 'a moderator can restore it');
select t.as_user(:'noah'); select public.mod_forum('thread', :'th', 'remove');
reset role; set role anon; select t.as_user(null);
select t.ok((select count(*) = 0 from public.forum_threads where id = :'th'), 'a removed thread is gone for visitors');
select t.ok((select count(*) >= 1 from public.forum_threads), 'visitors can read the forum');
reset role;
select t.ok((select count(*) >= 6 from public.mod_actions where action like 'forum_%'), 'forum moderation is audited');
-- muted players cannot post; rate limits
set role authenticated; select t.as_user(:'noah'); select public.mod_mute(:'rex', 1, 'test'); select t.as_user(:'rex');
select t.throws(format($q$select public.create_forum_thread(%L, 'Muted thread', 'should fail')$q$, :'cat_general'), 'row-level security', 'muted players cannot start threads');
select t.as_user(:'noah'); select public.mod_mute(:'rex', 0, 'done'); select t.as_user(:'rex');
do $$ declare i int; c uuid := (select id from public.forum_categories where slug = 'help'); begin for i in 1..6 loop insert into public.forum_threads (category_id, author_id, title) values (c, '00000000-0000-0000-0000-0000000000e6', 'Thread number ' || i); end loop; end $$;
select t.throws(format($q$insert into public.forum_threads (category_id, author_id, title) values (%L, %L, 'Thread number 7')$q$, :'cat_general', :'rex'), 'slow_down', 'at most 6 threads an hour');
select t.as_user(:'quinn');
do $$ declare i int; tid uuid := (select id from public.forum_threads where title = 'Welcome to Rookery'); begin for i in 1..4 loop insert into public.forum_posts (thread_id, author_id, body) values (tid, '00000000-0000-0000-0000-0000000000e5', 'post ' || i); end loop; end $$;
select t.throws(format($q$insert into public.forum_posts (thread_id, author_id, body) values (%L, %L, 'post 5')$q$, (select id from public.forum_threads where title = 'Welcome to Rookery'), :'quinn'), 'slow_down', 'at most 4 posts a minute');
reset role;

-- ============================================================ reports
set role authenticated; select t.as_user(:'olga');
insert into public.reports (reporter_id, target_user, kind, ref_id, reason, details) values (:'olga', :'pete', 'game_message', 'abc', 'abuse', 'rude');
select t.throws($q$insert into public.reports (reporter_id, target_user, kind, ref_id, reason) values ('00000000-0000-0000-0000-0000000000e3', '00000000-0000-0000-0000-0000000000e4', 'game_message', 'abc', 'spam')$q$, 'duplicate key', 'you cannot report the same thing twice while it is open');
select t.throws($q$insert into public.reports (reporter_id, kind, ref_id, reason, status) values ('00000000-0000-0000-0000-0000000000e3', 'user', 'x', 'spam', 'actioned')$q$, 'permission denied', 'players cannot set a report status');
select t.throws($q$insert into public.reports (reporter_id, kind, ref_id, reason) values ('00000000-0000-0000-0000-0000000000e4', 'user', 'y', 'spam')$q$, 'row-level security', 'you cannot report as someone else');
select t.throws($q$insert into public.reports (reporter_id, kind, ref_id, reason) values ('00000000-0000-0000-0000-0000000000e3', 'user', 'z', 'because')$q$, 'violates check', 'reasons come from a fixed list');
select t.ok((select count(*) = 1 from public.reports), 'you see your own reports');
select t.as_user(:'pete'); select t.ok((select count(*) = 0 from public.reports), 'the reported player does not');
select t.throws($q$select public.mod_resolve_report(1, 'actioned', 'x')$q$, 'forbidden', 'players cannot resolve reports');
select t.as_user(:'noah'); select t.ok((select count(*) = 1 from public.reports), 'moderators see every report');
select id as rid from public.reports limit 1 \gset
select public.mod_resolve_report(:'rid', 'actioned', 'muted the player');
select t.ok((select status = 'actioned' and handled_by = :'noah' and note = 'muted the player' from public.reports where id = :'rid'), 'a moderator resolves a report with a note');
select t.throws(format($q$select public.mod_resolve_report(%L, 'dismissed', 'again')$q$, :'rid'), 'not_found', 'a report is resolved once');
select t.as_user(:'olga');
insert into public.reports (reporter_id, target_user, kind, ref_id, reason) values (:'olga', :'pete', 'game_message', 'abc', 'abuse');
select t.ok(true, 'once resolved, the same thing can be reported again');
reset role;
select t.ok((select count(*) >= 1 from public.mod_actions where action = 'report_actioned'), 'resolutions are audited');

-- ============================================================ fair play
create function t.rated_win(winner uuid, loser uuid, i int) returns void language plpgsql as $$
declare gid uuid := gen_random_uuid();
begin
  insert into public.games (id, variant, initial_fen, white_id, black_id, status, result, reason, time_initial_ms, white_clock_ms, black_clock_ms, moves, rated)
  values (gid, 'standard', 'x', winner, loser, 'finished', '1-0', 'resignation', 180000, 0, 0, '{e2e4,e7e5}', true);
  insert into public.rating_history (user_id, perf, game_id, rating_before, rating_after, rd_after) values (winner, 'blitz', gid, 1500 + 40 * i, 1540 + 40 * i, 100);
  insert into public.rating_history (user_id, perf, game_id, rating_before, rating_after, rd_after) values (loser, 'blitz', gid, 1500, 1495, 100);
end $$;
select t.rated_win(:'rex', gen_random_uuid_user.id, g) from generate_series(1, 9) g cross join lateral (select ('00000000-0000-0000-0001-' || lpad(g::text, 12, '0'))::uuid as id) gen_random_uuid_user;
set role authenticated; select t.as_user(:'olga');
select t.throws($q$select * from public.fairplay_watchlist()$q$, 'forbidden', 'players cannot see the watchlist');
select t.as_user(:'noah');
select t.ok((select count(*) = 1 and bool_and(username = 'rex' and wins = 9 and net_gain >= 300) from public.fairplay_watchlist()), 'a player who wins everything and climbs fast is on the watchlist');
select t.ok((select reason like '%wins 100%' and reason like '%gained%' from public.fairplay_watchlist()), 'with the reasons spelled out');
select id as fg from public.games where white_id = :'rex' and rated limit 1 \gset
select public.mod_record_fairplay(:'fg', :'rex', 97.5, 0.88, 30, 'suspiciously clean') as fid \gset
select t.ok((select count(*) = 1 from public.fairplay_flags where user_id = :'rex' and status = 'open' and accuracy > 97), 'a moderator records an engine check');
select public.mod_record_fairplay(:'fg', :'rex', 90.0, 0.7, 30, 'rerun');
select t.ok((select count(*) = 1 and bool_and(accuracy = 90) from public.fairplay_flags where user_id = :'rex'), 'rechecking updates instead of duplicating');
select public.mod_fairplay_status(:'fid', 'cleared', 'played the opening from theory');
select t.ok((select status = 'cleared' from public.fairplay_flags where id = :'fid'), 'a flag can be cleared');
select t.throws(format($q$select public.mod_record_fairplay(%L, %L, 50, 0.5, 20, '')$q$, :'fg', :'olga'), 'not_found', 'you can only check someone who played that game');
select t.as_user(:'olga');
select t.ok((select count(*) = 0 from public.fairplay_flags), 'players cannot read fair-play flags');
select t.throws(format($q$select public.mod_record_fairplay(%L, %L, 50, 0.5, 20, '')$q$, :'fg', :'rex'), 'forbidden', 'or create them');
reset role;

\echo ALL SOCIAL AND MODERATION TESTS PASSED
