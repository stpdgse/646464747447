-- Rookery M0/M1: profiles, username rules, row level security.
-- Accounts are username + password. Supabase Auth needs an email, so the app maps
-- username -> "<username>@users.rookery.invalid" (never emailed). A real email can be added later.

create extension if not exists citext with schema extensions;

create table public.profiles (
  id            uuid primary key references auth.users (id) on delete cascade,
  username      extensions.citext not null unique,
  display_name  text,
  bio           text not null default '',
  country       text,
  is_admin      boolean not null default false,
  muted_until   timestamptz,
  banned_until  timestamptz,
  settings      jsonb not null default '{}'::jsonb,
  created_at    timestamptz not null default now(),
  constraint username_format check (username::text ~ '^[A-Za-z][A-Za-z0-9_]{2,19}$'),
  constraint display_name_len check (display_name is null or char_length(display_name) <= 40),
  constraint bio_len check (char_length(bio) <= 400),
  constraint country_fmt check (country is null or country ~ '^[A-Z]{2}$'),
  constraint settings_size check (pg_column_size(settings) <= 4096)
);

alter table public.profiles enable row level security;

-- Anyone may view profiles (public profile pages, leaderboards, game headers).
create policy "profiles are public" on public.profiles
  for select using (true);

-- Users may edit only their own row, and only the safe columns (see grants below).
create policy "users edit own profile" on public.profiles
  for update to authenticated
  using (id = (select auth.uid()))
  with check (id = (select auth.uid()));

-- Column-level lockdown: privileged fields (is_admin, mutes, bans, username, id) are never client-writable.
revoke insert, update, delete on public.profiles from anon, authenticated;
grant update (display_name, bio, country, settings) on public.profiles to authenticated;

-- Create the profile row automatically when an auth user is created.
create function public.handle_new_user()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  insert into public.profiles (id, username)
  values (new.id, new.raw_user_meta_data ->> 'username');
  return new;
end;
$$;

revoke execute on function public.handle_new_user() from public, anon, authenticated;

create trigger on_auth_user_created
  after insert on auth.users
  for each row execute function public.handle_new_user();

-- Admin check used by later RLS policies.
create function public.is_admin()
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  select coalesce((select p.is_admin from public.profiles p where p.id = (select auth.uid())), false);
$$;

revoke execute on function public.is_admin() from public, anon;
grant execute on function public.is_admin() to authenticated;
