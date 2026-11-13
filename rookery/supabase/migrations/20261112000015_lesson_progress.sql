-- Which lesson exercises and endgame drills a player has completed. Honour system: purely for progress and badges.
create table public.lesson_progress (
  user_id       uuid not null references public.profiles (id) on delete cascade,
  lesson_id     text not null check (lesson_id ~ '^[a-z0-9-]{2,40}$'),
  item_id       text not null check (char_length(item_id) between 1 and 40),
  completed_at  timestamptz not null default now(),
  primary key (user_id, lesson_id, item_id)
);
alter table public.lesson_progress enable row level security;
create policy "see my progress" on public.lesson_progress for select to authenticated using (user_id = (select auth.uid()));
create policy "record my progress" on public.lesson_progress for insert to authenticated with check (user_id = (select auth.uid()));
revoke all on table public.lesson_progress from anon, authenticated;
grant select on table public.lesson_progress to authenticated;
grant insert (user_id, lesson_id, item_id) on table public.lesson_progress to authenticated;
