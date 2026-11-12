-- Rookery M10 + M11 + M12: moderation, friends and blocks, presence, game chat, clubs, forum, reports, fair play.
--
-- Rules of the house:
--  * Moderation is enforced in the database. Muted and banned players cannot post anywhere (public.can_post()).
--  * Clients never edit roles, bans, mutes, hidden flags or audit data directly: only the mod_* / admin_* functions
--    below do, and each checks the caller's role itself and writes an audit row.
--  * Every table lists exactly which privileges clients get; row level security does the rest.

-- =============================================================================================================
-- Roles and helper functions (callable by policies, so they live in public and only ever look at the caller)

alter table public.profiles add column is_moderator boolean not null default false;

create function public.is_mod() returns boolean language sql stable security definer set search_path = '' as $$
  select coalesce((select p.is_admin or p.is_moderator from public.profiles p where p.id = (select auth.uid())), false)
$$;

create function public.can_post() returns boolean language sql stable security definer set search_path = '' as $$
  select (select auth.uid()) is not null and not exists (
    select 1 from public.profiles p
    where p.id = (select auth.uid())
      and ((p.banned_until is not null and p.banned_until > now()) or (p.muted_until is not null and p.muted_until > now()))
  )
$$;

create table public.blocks (
  blocker_id  uuid not null references public.profiles (id) on delete cascade,
  blocked_id  uuid not null references public.profiles (id) on delete cascade,
  created_at  timestamptz not null default now(),
  primary key (blocker_id, blocked_id),
  constraint blocks_not_self check (blocker_id <> blocked_id)
);
alter table public.blocks enable row level security;
create policy "see own blocks" on public.blocks for select to authenticated using (blocker_id = (select auth.uid()));
create policy "block someone" on public.blocks for insert to authenticated with check (blocker_id = (select auth.uid()));
create policy "unblock" on public.blocks for delete to authenticated using (blocker_id = (select auth.uid()));
revoke all on table public.blocks from anon, authenticated;
grant select, insert, delete on table public.blocks to authenticated;

create function public.blocked_with(p_other uuid) returns boolean language sql stable security definer set search_path = '' as $$
  select exists (
    select 1 from public.blocks b
    where (b.blocker_id = (select auth.uid()) and b.blocked_id = p_other) or (b.blocker_id = p_other and b.blocked_id = (select auth.uid()))
  )
$$;
create function public.i_blocked(p_other uuid) returns boolean language sql stable security definer set search_path = '' as $$
  select exists (select 1 from public.blocks b where b.blocker_id = (select auth.uid()) and b.blocked_id = p_other)
$$;
create function public.is_game_player(p_game uuid) returns boolean language sql stable security definer set search_path = '' as $$
  select exists (select 1 from public.games g where g.id = p_game and (select auth.uid()) in (g.white_id, g.black_id))
$$;

-- Shared guards used by the mod_* functions.
create function private.require_mod() returns uuid language plpgsql security definer set search_path = '' as $$
declare uid uuid := auth.uid();
begin
  if uid is null or not exists (select 1 from public.profiles p where p.id = uid and (p.is_admin or p.is_moderator)) then
    raise exception 'forbidden' using errcode = '42501';
  end if;
  return uid;
end $$;
create function private.require_admin() returns uuid language plpgsql security definer set search_path = '' as $$
declare uid uuid := auth.uid();
begin
  if uid is null or not exists (select 1 from public.profiles p where p.id = uid and p.is_admin) then
    raise exception 'forbidden' using errcode = '42501';
  end if;
  return uid;
end $$;

-- Generic "no more than N rows per window per user" guard for posting tables.
create function private.rate_limit() returns trigger language plpgsql security definer set search_path = '' as $$
declare col text := tg_argv[0]; secs integer := tg_argv[1]::integer; maxn integer := tg_argv[2]::integer; uid uuid; n integer;
begin
  execute format('select ($1).%I', col) into uid using new;
  execute format('select count(*) from %I.%I where %I = $1 and created_at > now() - make_interval(secs => $2)', tg_table_schema, tg_table_name, col)
    into n using uid, secs;
  if n >= maxn then raise exception 'slow_down'; end if;
  return new;
end $$;

-- =============================================================================================================
-- Audit log, reports, fair play

create table public.mod_actions (
  id           bigint generated always as identity primary key,
  actor_id     uuid not null references public.profiles (id) on delete cascade,
  action       text not null,
  target_user  uuid references public.profiles (id) on delete set null,
  ref_kind     text,
  ref_id       text,
  detail       text,
  created_at   timestamptz not null default now()
);
create index mod_actions_created_idx on public.mod_actions (created_at desc);
alter table public.mod_actions enable row level security;
create policy "moderators read the audit log" on public.mod_actions for select to authenticated using (public.is_mod());
revoke all on table public.mod_actions from anon, authenticated;
grant select on table public.mod_actions to authenticated;

create function private.log_mod(p_actor uuid, p_action text, p_target uuid, p_kind text, p_ref text, p_detail text) returns void
language sql security definer set search_path = '' as $$
  insert into public.mod_actions (actor_id, action, target_user, ref_kind, ref_id, detail) values (p_actor, p_action, p_target, p_kind, p_ref, left(p_detail, 500))
$$;

create table public.reports (
  id           bigint generated always as identity primary key,
  reporter_id  uuid not null references public.profiles (id) on delete cascade,
  target_user  uuid references public.profiles (id) on delete set null,
  kind         text not null check (kind in ('user', 'game', 'game_message', 'club_message', 'forum_post', 'forum_thread')),
  ref_id       text not null check (char_length(ref_id) <= 64),
  reason       text not null check (reason in ('cheating', 'abuse', 'spam', 'inappropriate', 'other')),
  details      text not null default '' check (char_length(details) <= 500),
  status       text not null default 'open' check (status in ('open', 'actioned', 'dismissed')),
  handled_by   uuid references public.profiles (id) on delete set null,
  handled_at   timestamptz,
  note         text check (note is null or char_length(note) <= 500),
  created_at   timestamptz not null default now()
);
create unique index reports_one_open_per_reporter on public.reports (reporter_id, kind, ref_id) where status = 'open';
create index reports_status_idx on public.reports (status, created_at desc);
alter table public.reports enable row level security;
create policy "file a report" on public.reports for insert to authenticated
  with check (reporter_id = (select auth.uid()) and status = 'open' and handled_by is null and public.can_post());
create policy "read own reports, moderators read all" on public.reports for select to authenticated
  using (reporter_id = (select auth.uid()) or public.is_mod());
revoke all on table public.reports from anon, authenticated;
grant select on table public.reports to authenticated;
grant insert (reporter_id, target_user, kind, ref_id, reason, details) on table public.reports to authenticated;
create trigger reports_rate_limit before insert on public.reports for each row execute function private.rate_limit('reporter_id', '86400', '12');

create table public.fairplay_flags (
  id          bigint generated always as identity primary key,
  game_id     uuid not null references public.games (id) on delete cascade,
  user_id     uuid not null references public.profiles (id) on delete cascade,
  accuracy    real not null,
  best_match  real not null,
  moves       integer not null,
  status      text not null default 'open' check (status in ('open', 'cleared', 'confirmed')),
  note        text check (note is null or char_length(note) <= 500),
  created_by  uuid not null references public.profiles (id) on delete cascade,
  created_at  timestamptz not null default now(),
  unique (game_id, user_id)
);
alter table public.fairplay_flags enable row level security;
create policy "moderators read fair-play flags" on public.fairplay_flags for select to authenticated using (public.is_mod());
revoke all on table public.fairplay_flags from anon, authenticated;
grant select on table public.fairplay_flags to authenticated;

-- ---- moderator actions ---------------------------------------------------------------------------------------

create function public.mod_mute(p_user uuid, p_hours integer, p_reason text) returns void
language plpgsql security definer set search_path = '' as $$
declare actor uuid := private.require_mod(); actor_admin boolean;
begin
  select p.is_admin into actor_admin from public.profiles p where p.id = actor;
  if exists (select 1 from public.profiles p where p.id = p_user and (p.is_admin or (p.is_moderator and not actor_admin))) then
    raise exception 'forbidden_target' using errcode = '42501';
  end if;
  update public.profiles set muted_until = case when coalesce(p_hours, 0) <= 0 then null else now() + make_interval(hours => least(p_hours, 87600)) end where id = p_user;
  perform private.log_mod(actor, case when coalesce(p_hours, 0) <= 0 then 'unmute' else 'mute' end, p_user, 'user', p_user::text,
                          coalesce(p_reason, '') || case when coalesce(p_hours, 0) > 0 then ' (' || p_hours || 'h)' else '' end);
end $$;

create function public.mod_ban(p_user uuid, p_days integer, p_reason text) returns void
language plpgsql security definer set search_path = '' as $$
declare actor uuid := private.require_admin();
begin
  if exists (select 1 from public.profiles p where p.id = p_user and p.is_admin) then raise exception 'forbidden_target' using errcode = '42501'; end if;
  update public.profiles set banned_until = case when coalesce(p_days, 0) <= 0 then null else now() + make_interval(days => least(p_days, 36500)) end where id = p_user;
  perform private.log_mod(actor, case when coalesce(p_days, 0) <= 0 then 'unban' else 'ban' end, p_user, 'user', p_user::text,
                          coalesce(p_reason, '') || case when coalesce(p_days, 0) > 0 then ' (' || p_days || 'd)' else '' end);
end $$;

create function public.admin_set_moderator(p_user uuid, p_on boolean) returns void
language plpgsql security definer set search_path = '' as $$
declare actor uuid := private.require_admin();
begin
  update public.profiles set is_moderator = coalesce(p_on, false) where id = p_user and not is_admin;
  perform private.log_mod(actor, case when p_on then 'make_moderator' else 'remove_moderator' end, p_user, 'user', p_user::text, '');
end $$;

create function public.mod_resolve_report(p_report bigint, p_status text, p_note text) returns void
language plpgsql security definer set search_path = '' as $$
declare actor uuid := private.require_mod(); r public.reports;
begin
  if p_status not in ('actioned', 'dismissed') then raise exception 'bad_status'; end if;
  update public.reports set status = p_status, handled_by = actor, handled_at = now(), note = left(p_note, 500)
   where id = p_report and status = 'open' returning * into r;
  if not found then raise exception 'not_found'; end if;
  perform private.log_mod(actor, 'report_' || p_status, r.target_user, r.kind, r.ref_id, p_note);
end $$;

create function public.mod_record_fairplay(p_game uuid, p_user uuid, p_accuracy real, p_match real, p_moves integer, p_note text) returns bigint
language plpgsql security definer set search_path = '' as $$
declare actor uuid := private.require_mod(); fid bigint;
begin
  if not exists (select 1 from public.games g where g.id = p_game and p_user in (g.white_id, g.black_id)) then raise exception 'not_found'; end if;
  insert into public.fairplay_flags (game_id, user_id, accuracy, best_match, moves, note, created_by)
  values (p_game, p_user, p_accuracy, p_match, p_moves, left(p_note, 500), actor)
  on conflict (game_id, user_id) do update set accuracy = excluded.accuracy, best_match = excluded.best_match, moves = excluded.moves,
                                              note = excluded.note, status = 'open', created_by = excluded.created_by, created_at = now()
  returning id into fid;
  perform private.log_mod(actor, 'fairplay_check', p_user, 'game', p_game::text, 'accuracy ' || round(p_accuracy::numeric, 1) || ', best-move match ' || round((p_match * 100)::numeric) || '%');
  return fid;
end $$;

create function public.mod_fairplay_status(p_flag bigint, p_status text, p_note text) returns void
language plpgsql security definer set search_path = '' as $$
declare actor uuid := private.require_mod(); f public.fairplay_flags;
begin
  if p_status not in ('open', 'cleared', 'confirmed') then raise exception 'bad_status'; end if;
  update public.fairplay_flags set status = p_status, note = coalesce(left(p_note, 500), note) where id = p_flag returning * into f;
  if not found then raise exception 'not_found'; end if;
  perform private.log_mod(actor, 'fairplay_' || p_status, f.user_id, 'game', f.game_id::text, p_note);
end $$;

/** Players worth a second look: lots of recent rated wins, or a very fast rating climb. Moderators only. */
create function public.fairplay_watchlist() returns table (user_id uuid, username text, games bigint, wins bigint, net_gain bigint, reason text)
language plpgsql stable security definer set search_path = '' as $$
begin
  perform private.require_mod();
  return query
  select s.user_id, p.username::text, s.games, s.wins, s.net_gain,
         concat_ws(' and ', case when s.wins::numeric / s.games >= 0.85 then 'wins ' || round(100.0 * s.wins / s.games) || '% of rated games' end,
                            case when s.net_gain >= 300 then 'gained ' || s.net_gain || ' rating in 14 days' end)
    from (
      select h.user_id, count(*) as games,
             count(*) filter (where (g.result = '1-0' and g.white_id = h.user_id) or (g.result = '0-1' and g.black_id = h.user_id)) as wins,
             sum(h.rating_after - h.rating_before) as net_gain
        from public.rating_history h join public.games g on g.id = h.game_id
       where h.created_at > now() - interval '14 days' and g.rated
       group by h.user_id
    ) s join public.profiles p on p.id = s.user_id
   where s.games >= 8 and (s.wins::numeric / s.games >= 0.85 or s.net_gain >= 300)
   order by s.net_gain desc;
end $$;

-- =============================================================================================================
-- Friends, presence

create table public.friendships (
  id            bigint generated always as identity primary key,
  requester_id  uuid not null references public.profiles (id) on delete cascade,
  addressee_id  uuid not null references public.profiles (id) on delete cascade,
  status        text not null default 'pending' check (status in ('pending', 'accepted')),
  created_at    timestamptz not null default now(),
  responded_at  timestamptz,
  constraint friendships_not_self check (requester_id <> addressee_id)
);
create unique index friendships_one_per_pair on public.friendships (least(requester_id, addressee_id), greatest(requester_id, addressee_id));
create index friendships_addressee_idx on public.friendships (addressee_id, status);
alter table public.friendships enable row level security;
create policy "see my friendships" on public.friendships for select to authenticated
  using (requester_id = (select auth.uid()) or addressee_id = (select auth.uid()));
create policy "send a friend request" on public.friendships for insert to authenticated
  with check (requester_id = (select auth.uid()) and status = 'pending' and public.can_post() and not public.blocked_with(addressee_id));
create policy "accept a friend request" on public.friendships for update to authenticated
  using (addressee_id = (select auth.uid()) and status = 'pending')
  with check (addressee_id = (select auth.uid()) and status = 'accepted');
create policy "decline, cancel or unfriend" on public.friendships for delete to authenticated
  using (requester_id = (select auth.uid()) or addressee_id = (select auth.uid()));
revoke all on table public.friendships from anon, authenticated;
grant select, delete on table public.friendships to authenticated;
grant insert (requester_id, addressee_id) on table public.friendships to authenticated;
grant update (status) on table public.friendships to authenticated;
create trigger friendships_rate_limit before insert on public.friendships for each row execute function private.rate_limit('requester_id', '3600', '20');

create function private.friendship_changed() returns trigger language plpgsql security definer set search_path = '' as $$
begin
  if tg_op = 'INSERT' then
    insert into public.notifications (user_id, kind, actor_id) values (new.addressee_id, 'friend_request', new.requester_id);
  elsif new.status = 'accepted' and old.status = 'pending' then
    new.responded_at := now();
    insert into public.notifications (user_id, kind, actor_id) values (new.requester_id, 'friend_accepted', new.addressee_id);
  end if;
  return new;
end $$;
create trigger friendships_notify_insert after insert on public.friendships for each row execute function private.friendship_changed();
create trigger friendships_notify_update before update on public.friendships for each row execute function private.friendship_changed();

create function public.is_friend(p_other uuid) returns boolean language sql stable security definer set search_path = '' as $$
  select exists (
    select 1 from public.friendships f
    where f.status = 'accepted'
      and ((f.requester_id = (select auth.uid()) and f.addressee_id = p_other) or (f.addressee_id = (select auth.uid()) and f.requester_id = p_other))
  )
$$;

-- Presence: a heartbeat row per player, readable only by that player and their friends.
create table public.presence (
  user_id       uuid primary key references public.profiles (id) on delete cascade,
  last_seen_at  timestamptz not null default now()
);
alter table public.presence enable row level security;
create policy "see my own and my friends presence" on public.presence for select to authenticated
  using (user_id = (select auth.uid()) or public.is_friend(user_id));
create policy "record my own presence" on public.presence for insert to authenticated with check (user_id = (select auth.uid()));
create policy "refresh my own presence" on public.presence for update to authenticated
  using (user_id = (select auth.uid())) with check (user_id = (select auth.uid()));
revoke all on table public.presence from anon, authenticated;
grant select on table public.presence to authenticated;
grant insert (user_id, last_seen_at) on table public.presence to authenticated;
grant update (last_seen_at) on table public.presence to authenticated;

-- Challenges never cross a block.
create policy "no challenges across a block" on public.seeks as restrictive for insert to authenticated
  with check (target_id is null or not public.blocked_with(target_id));

-- =============================================================================================================
-- Game chat: one room for the two players, one for people watching.

create table public.game_messages (
  id          uuid primary key default gen_random_uuid(),
  game_id     uuid not null references public.games (id) on delete cascade,
  user_id     uuid not null references public.profiles (id) on delete cascade,
  room        text not null check (room in ('players', 'spectators')),
  body        text not null check (char_length(body) between 1 and 200),
  hidden      boolean not null default false,
  created_at  timestamptz not null default now()
);
create index game_messages_game_idx on public.game_messages (game_id, created_at);
alter table public.game_messages enable row level security;
create policy "read game chat" on public.game_messages for select to authenticated using (
  public.is_mod() or (
    not hidden and not public.i_blocked(user_id) and (
      (room = 'players' and public.is_game_player(game_id)) or (room = 'spectators' and not public.is_game_player(game_id))
    )
  )
);
create policy "write game chat" on public.game_messages for insert to authenticated with check (
  user_id = (select auth.uid()) and public.can_post() and
  ((room = 'players' and public.is_game_player(game_id)) or (room = 'spectators' and not public.is_game_player(game_id)))
);
revoke all on table public.game_messages from anon, authenticated;
grant select on table public.game_messages to authenticated;
grant insert (game_id, user_id, room, body) on table public.game_messages to authenticated;
create trigger game_messages_rate_limit before insert on public.game_messages for each row execute function private.rate_limit('user_id', '10', '8');

-- =============================================================================================================
-- Clubs

create table public.clubs (
  id           uuid primary key default gen_random_uuid(),
  slug         text not null unique check (slug ~ '^[a-z0-9][a-z0-9-]{2,29}$'),
  name         text not null check (char_length(name) between 3 and 40),
  description  text not null default '' check (char_length(description) <= 400),
  owner_id     uuid not null references public.profiles (id) on delete cascade,
  open         boolean not null default true,
  archived     boolean not null default false,
  created_at   timestamptz not null default now()
);
create table public.club_members (
  club_id    uuid not null references public.clubs (id) on delete cascade,
  user_id    uuid not null references public.profiles (id) on delete cascade,
  role       text not null default 'member' check (role in ('owner', 'moderator', 'member')),
  status     text not null default 'active' check (status in ('active', 'requested', 'invited')),
  joined_at  timestamptz not null default now(),
  primary key (club_id, user_id)
);
create index club_members_user_idx on public.club_members (user_id, status);

create function public.club_role(p_club uuid) returns text language sql stable security definer set search_path = '' as $$
  select m.role from public.club_members m where m.club_id = p_club and m.user_id = (select auth.uid()) and m.status = 'active'
$$;
create function public.club_is_open(p_club uuid) returns boolean language sql stable security definer set search_path = '' as $$
  select coalesce((select c.open and not c.archived from public.clubs c where c.id = p_club), false)
$$;

alter table public.clubs enable row level security;
alter table public.club_members enable row level security;
create policy "see clubs" on public.clubs for select using (not archived or public.club_role(id) = 'owner' or public.is_mod());
create policy "create a club" on public.clubs for insert to authenticated with check (owner_id = (select auth.uid()) and public.can_post());
create policy "owner edits the club" on public.clubs for update to authenticated
  using (public.club_role(id) = 'owner') with check (public.club_role(id) = 'owner' and owner_id = (select auth.uid()));
revoke all on table public.clubs from anon, authenticated;
grant select on table public.clubs to anon, authenticated;
grant insert (slug, name, description, owner_id, open) on table public.clubs to authenticated;
grant update (name, description, open, archived) on table public.clubs to authenticated;

create policy "see members" on public.club_members for select using (
  status = 'active' or user_id = (select auth.uid()) or public.club_role(club_id) in ('owner', 'moderator') or public.is_mod()
);
create policy "join, request or invite" on public.club_members for insert to authenticated with check (
  (user_id = (select auth.uid()) and role = 'member' and public.can_post() and
     ((status = 'active' and public.club_is_open(club_id)) or (status = 'requested' and not public.club_is_open(club_id))))
  or (public.club_role(club_id) in ('owner', 'moderator') and role = 'member' and status = 'invited' and user_id <> (select auth.uid()))
);
create policy "accept an invite or approve a request" on public.club_members for update to authenticated
  using ((user_id = (select auth.uid()) and status = 'invited') or (public.club_role(club_id) in ('owner', 'moderator') and status = 'requested'))
  with check (status = 'active' and role = 'member');
create policy "leave or remove a member" on public.club_members for delete to authenticated using (
  role <> 'owner' and (user_id = (select auth.uid()) or public.club_role(club_id) = 'owner'
                       or (public.club_role(club_id) = 'moderator' and role = 'member'))
);
revoke all on table public.club_members from anon, authenticated;
grant select on table public.club_members to anon, authenticated;
grant insert (club_id, user_id, role, status) on table public.club_members to authenticated;
grant update (status) on table public.club_members to authenticated;
grant delete on table public.club_members to authenticated;

create function private.club_created() returns trigger language plpgsql security definer set search_path = '' as $$
begin
  if (select count(*) from public.clubs c where c.owner_id = new.owner_id and not c.archived) >= 3 then raise exception 'too_many_clubs'; end if;
  return new;
end $$;
create trigger clubs_limit before insert on public.clubs for each row execute function private.club_created();
create function private.club_owner_member() returns trigger language plpgsql security definer set search_path = '' as $$
begin
  insert into public.club_members (club_id, user_id, role, status) values (new.id, new.owner_id, 'owner', 'active');
  return new;
end $$;
create trigger clubs_owner_member after insert on public.clubs for each row execute function private.club_owner_member();

alter table public.notifications add column club_id uuid references public.clubs (id) on delete cascade;
create function private.club_member_invited() returns trigger language plpgsql security definer set search_path = '' as $$
begin
  if new.status = 'invited' then
    insert into public.notifications (user_id, kind, actor_id, club_id) values (new.user_id, 'club_invite', auth.uid(), new.club_id);
  end if;
  return new;
end $$;
create trigger club_members_invited after insert on public.club_members for each row execute function private.club_member_invited();

create function public.club_set_role(p_club uuid, p_user uuid, p_role text) returns void
language plpgsql security definer set search_path = '' as $$
begin
  if public.club_role(p_club) is distinct from 'owner' then raise exception 'forbidden' using errcode = '42501'; end if;
  if p_role not in ('moderator', 'member') then raise exception 'bad_role'; end if;
  update public.club_members set role = p_role where club_id = p_club and user_id = p_user and role <> 'owner' and status = 'active';
end $$;

create table public.club_messages (
  id          uuid primary key default gen_random_uuid(),
  club_id     uuid not null references public.clubs (id) on delete cascade,
  user_id     uuid not null references public.profiles (id) on delete cascade,
  body        text not null check (char_length(body) between 1 and 300),
  hidden      boolean not null default false,
  created_at  timestamptz not null default now()
);
create index club_messages_club_idx on public.club_messages (club_id, created_at);
alter table public.club_messages enable row level security;
create policy "members read club chat" on public.club_messages for select to authenticated
  using (public.is_mod() or (not hidden and not public.i_blocked(user_id) and public.club_role(club_id) is not null));
create policy "members write club chat" on public.club_messages for insert to authenticated
  with check (user_id = (select auth.uid()) and public.can_post() and public.club_role(club_id) is not null);
revoke all on table public.club_messages from anon, authenticated;
grant select on table public.club_messages to authenticated;
grant insert (club_id, user_id, body) on table public.club_messages to authenticated;
create trigger club_messages_rate_limit before insert on public.club_messages for each row execute function private.rate_limit('user_id', '10', '8');

-- =============================================================================================================
-- Forum

create table public.forum_categories (
  id          uuid primary key default gen_random_uuid(),
  slug        text not null unique,
  name        text not null,
  description text not null default '',
  sort        integer not null default 0,
  mod_only    boolean not null default false
);
alter table public.forum_categories enable row level security;
create policy "categories are public" on public.forum_categories for select using (true);
revoke all on table public.forum_categories from anon, authenticated;
grant select on table public.forum_categories to anon, authenticated;
insert into public.forum_categories (slug, name, description, sort, mod_only) values
  ('announcements', 'Announcements', 'News about the site. Only moderators post here.', 0, true),
  ('general', 'General chess talk', 'Anything about chess.', 1, false),
  ('help', 'Help and questions', 'New to chess or the site? Ask here.', 2, false),
  ('games', 'Game analysis', 'Share a game and talk it through.', 3, false),
  ('feedback', 'Site feedback', 'Bugs, ideas and requests.', 4, false)
on conflict (slug) do nothing;

create table public.forum_threads (
  id              uuid primary key default gen_random_uuid(),
  category_id     uuid not null references public.forum_categories (id),
  author_id       uuid not null references public.profiles (id) on delete cascade,
  title           text not null check (char_length(title) between 3 and 120),
  created_at      timestamptz not null default now(),
  last_post_at    timestamptz not null default now(),
  post_count      integer not null default 0,
  pinned          boolean not null default false,
  locked          boolean not null default false,
  deleted_at      timestamptz
);
create index forum_threads_category_idx on public.forum_threads (category_id, pinned desc, last_post_at desc);
create table public.forum_posts (
  id               uuid primary key default gen_random_uuid(),
  thread_id        uuid not null references public.forum_threads (id) on delete cascade,
  author_id        uuid not null references public.profiles (id) on delete cascade,
  body             text not null check (char_length(body) between 1 and 4000),
  created_at       timestamptz not null default now(),
  edited_at        timestamptz,
  deleted_at       timestamptz,
  deleted_by_mod   boolean not null default false
);
create index forum_posts_thread_idx on public.forum_posts (thread_id, created_at);

create function public.category_open_for_me(p_cat uuid) returns boolean language sql stable security definer set search_path = '' as $$
  select exists (select 1 from public.forum_categories c where c.id = p_cat and (not c.mod_only or public.is_mod()))
$$;
create function public.thread_open(p_thread uuid) returns boolean language sql stable security definer set search_path = '' as $$
  select exists (select 1 from public.forum_threads t where t.id = p_thread and not t.locked and t.deleted_at is null)
$$;

alter table public.forum_threads enable row level security;
alter table public.forum_posts enable row level security;
create policy "read threads" on public.forum_threads for select using (deleted_at is null or public.is_mod());
create policy "start a thread" on public.forum_threads for insert to authenticated
  with check (author_id = (select auth.uid()) and public.can_post() and public.category_open_for_me(category_id));
revoke all on table public.forum_threads from anon, authenticated;
grant select on table public.forum_threads to anon, authenticated;
grant insert (category_id, author_id, title) on table public.forum_threads to authenticated;

-- An author can still read their own self-deleted post (Postgres requires the updated row to stay readable for the
-- author to delete it); posts removed by a moderator are hidden from everyone but moderators. The app hides both.
create policy "read posts" on public.forum_posts for select using (
  deleted_at is null or public.is_mod() or (author_id = (select auth.uid()) and not deleted_by_mod)
);
create policy "write a post" on public.forum_posts for insert to authenticated
  with check (author_id = (select auth.uid()) and public.can_post() and public.thread_open(thread_id));
create policy "edit or delete my own post" on public.forum_posts for update to authenticated
  using (author_id = (select auth.uid()) and not deleted_by_mod and public.can_post())
  with check (author_id = (select auth.uid()) and not deleted_by_mod);
revoke all on table public.forum_posts from anon, authenticated;
grant select on table public.forum_posts to anon, authenticated;
grant insert (thread_id, author_id, body) on table public.forum_posts to authenticated;
grant update (body, deleted_at) on table public.forum_posts to authenticated;
create trigger forum_posts_rate_limit before insert on public.forum_posts for each row execute function private.rate_limit('author_id', '60', '4');
create trigger forum_threads_rate_limit before insert on public.forum_threads for each row execute function private.rate_limit('author_id', '3600', '6');

create function private.forum_post_added() returns trigger language plpgsql security definer set search_path = '' as $$
begin
  update public.forum_threads set last_post_at = new.created_at, post_count = post_count + 1 where id = new.thread_id;
  return new;
end $$;
create trigger forum_posts_added after insert on public.forum_posts for each row execute function private.forum_post_added();
create function private.forum_post_edited() returns trigger language plpgsql security definer set search_path = '' as $$
begin
  if new.body is distinct from old.body then new.edited_at := now(); end if;
  return new;
end $$;
create trigger forum_posts_edited before update on public.forum_posts for each row execute function private.forum_post_edited();

/** Start a thread and its first post in one step. */
create function public.create_forum_thread(p_category uuid, p_title text, p_body text) returns uuid
language plpgsql security invoker set search_path = '' as $$
declare tid uuid;
begin
  insert into public.forum_threads (category_id, author_id, title) values (p_category, (select auth.uid()), btrim(p_title)) returning id into tid;
  insert into public.forum_posts (thread_id, author_id, body) values (tid, (select auth.uid()), btrim(p_body));
  return tid;
end $$;

create function public.mod_forum(p_kind text, p_id uuid, p_action text) returns void
language plpgsql security definer set search_path = '' as $$
declare actor uuid := private.require_mod(); author uuid;
begin
  if p_kind = 'post' then
    select p.author_id into author from public.forum_posts p where p.id = p_id;
    if not found then raise exception 'not_found'; end if;
    if p_action = 'remove' then update public.forum_posts set deleted_at = now(), deleted_by_mod = true where id = p_id;
    elsif p_action = 'restore' then update public.forum_posts set deleted_at = null, deleted_by_mod = false where id = p_id;
    else raise exception 'bad_action'; end if;
  elsif p_kind = 'thread' then
    select t.author_id into author from public.forum_threads t where t.id = p_id;
    if not found then raise exception 'not_found'; end if;
    if p_action = 'remove' then update public.forum_threads set deleted_at = now() where id = p_id;
    elsif p_action = 'restore' then update public.forum_threads set deleted_at = null where id = p_id;
    elsif p_action = 'lock' then update public.forum_threads set locked = true where id = p_id;
    elsif p_action = 'unlock' then update public.forum_threads set locked = false where id = p_id;
    elsif p_action = 'pin' then update public.forum_threads set pinned = true where id = p_id;
    elsif p_action = 'unpin' then update public.forum_threads set pinned = false where id = p_id;
    else raise exception 'bad_action'; end if;
  else
    raise exception 'bad_kind';
  end if;
  perform private.log_mod(actor, 'forum_' || p_kind || '_' || p_action, author, 'forum_' || p_kind, p_id::text, '');
end $$;

create function public.mod_hide_message(p_kind text, p_id uuid, p_hidden boolean) returns void
language plpgsql security definer set search_path = '' as $$
declare actor uuid := private.require_mod(); author uuid;
begin
  if p_kind = 'game' then
    update public.game_messages set hidden = coalesce(p_hidden, true) where id = p_id returning user_id into author;
  elsif p_kind = 'club' then
    update public.club_messages set hidden = coalesce(p_hidden, true) where id = p_id returning user_id into author;
  else
    raise exception 'bad_kind';
  end if;
  if author is null then raise exception 'not_found'; end if;
  perform private.log_mod(actor, case when coalesce(p_hidden, true) then 'hide_message' else 'show_message' end, author, p_kind || '_message', p_id::text, '');
end $$;

-- =============================================================================================================
-- Realtime and API surface

do $$
begin
  if exists (select 1 from pg_publication where pubname = 'supabase_realtime') then
    alter publication supabase_realtime add table public.game_messages;
    alter publication supabase_realtime add table public.club_messages;
    alter publication supabase_realtime add table public.friendships;
  end if;
end $$;

-- New functions start with default execute rights: lock them down, then grant exactly what each role needs.
revoke execute on function public.is_mod(), public.can_post(), public.blocked_with(uuid), public.i_blocked(uuid), public.is_game_player(uuid),
  public.is_friend(uuid), public.club_role(uuid), public.club_is_open(uuid), public.category_open_for_me(uuid), public.thread_open(uuid),
  public.create_forum_thread(uuid, text, text), public.club_set_role(uuid, uuid, text),
  public.mod_mute(uuid, integer, text), public.mod_ban(uuid, integer, text), public.admin_set_moderator(uuid, boolean),
  public.mod_resolve_report(bigint, text, text), public.mod_record_fairplay(uuid, uuid, real, real, integer, text),
  public.mod_fairplay_status(bigint, text, text), public.fairplay_watchlist(), public.mod_forum(text, uuid, text),
  public.mod_hide_message(text, uuid, boolean) from public, anon, authenticated;
-- Helpers used inside policies must also run for signed-out visitors (they simply answer "no").
grant execute on function public.is_mod(), public.can_post(), public.club_role(uuid), public.club_is_open(uuid),
  public.category_open_for_me(uuid), public.thread_open(uuid) to anon, authenticated;
grant execute on function public.blocked_with(uuid), public.i_blocked(uuid), public.is_game_player(uuid), public.is_friend(uuid),
  public.create_forum_thread(uuid, text, text), public.club_set_role(uuid, uuid, text),
  public.mod_mute(uuid, integer, text), public.mod_ban(uuid, integer, text), public.admin_set_moderator(uuid, boolean),
  public.mod_resolve_report(bigint, text, text), public.mod_record_fairplay(uuid, uuid, real, real, integer, text),
  public.mod_fairplay_status(bigint, text, text), public.fairplay_watchlist(), public.mod_forum(text, uuid, text),
  public.mod_hide_message(text, uuid, boolean) to authenticated;
