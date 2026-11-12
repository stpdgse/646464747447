-- Rookery M10/M11: notifications gain social kinds and an actor ("who did it").
alter table public.notifications drop constraint notifications_kind_check;
alter table public.notifications add constraint notifications_kind_check
  check (kind in ('your_turn', 'game_over', 'friend_request', 'friend_accepted', 'club_invite'));
alter table public.notifications add column actor_id uuid references public.profiles (id) on delete cascade;
