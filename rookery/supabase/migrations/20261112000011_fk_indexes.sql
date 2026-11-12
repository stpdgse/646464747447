-- Covering indexes for foreign keys flagged by the Supabase performance advisor.
create index if not exists blocks_blocked_idx on public.blocks (blocked_id);
create index if not exists club_messages_user_idx on public.club_messages (user_id);
create index if not exists clubs_owner_idx on public.clubs (owner_id);
create index if not exists fairplay_flags_user_idx on public.fairplay_flags (user_id);
create index if not exists fairplay_flags_created_by_idx on public.fairplay_flags (created_by);
create index if not exists forum_posts_author_idx on public.forum_posts (author_id);
create index if not exists forum_threads_author_idx on public.forum_threads (author_id);
create index if not exists friendships_requester_idx on public.friendships (requester_id);
create index if not exists game_messages_user_idx on public.game_messages (user_id);
create index if not exists mod_actions_actor_idx on public.mod_actions (actor_id);
create index if not exists mod_actions_target_idx on public.mod_actions (target_user);
create index if not exists notifications_actor_idx on public.notifications (actor_id);
create index if not exists notifications_club_idx on public.notifications (club_id);
create index if not exists notifications_game_idx on public.notifications (game_id);
create index if not exists reports_handled_by_idx on public.reports (handled_by);
create index if not exists reports_target_idx on public.reports (target_user);
create index if not exists seeks_target_idx on public.seeks (target_id);
