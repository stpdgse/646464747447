-- Admin-made bots, and admin edits of the built-in bots. A row whose id matches a built-in bot overrides it
-- (null columns keep the built-in value); any other id is a brand-new bot. Everyone can read, only admins can write.

create table public.custom_bots (
  id            text primary key check (id ~ '^[a-z0-9][a-z0-9-]{1,29}$'),
  name          text not null check (char_length(name) between 1 and 40),
  avatar        text not null default '🤖' check (char_length(avatar) between 1 and 16),
  -- small picture, resized in the browser and stored inline (png, jpeg or webp)
  avatar_image  text check (avatar_image is null or (avatar_image ~ '^data:image/(png|jpeg|webp);base64,[A-Za-z0-9+/=]+$' and char_length(avatar_image) <= 90000)),
  rating        integer not null check (rating between 0 and 5000),
  blurb         text not null default '' check (char_length(blurb) <= 200),
  greeting      text not null default '' check (char_length(greeting) <= 160),
  quip_win      text not null default '' check (char_length(quip_win) <= 160),
  quip_lose     text not null default '' check (char_length(quip_lose) <= 160),
  quip_draw     text not null default '' check (char_length(quip_draw) <= 160),
  -- playing strength: 0 (random mover) to 24 (full engine). Null on a built-in override keeps the original strength.
  smartness     integer check (smartness is null or smartness between 0 and 24),
  style         text check (style is null or style in ('balanced', 'aggressive', 'solid', 'greedy', 'checker', 'chaotic')),
  style_bonus   integer check (style_bonus is null or style_bonus between 0 and 300),
  blunder_pct   integer check (blunder_pct is null or blunder_pct between 0 and 90),
  mate_sight_pct integer check (mate_sight_pct is null or mate_sight_pct between 0 and 100),
  thinking_ms   integer check (thinking_ms is null or thinking_ms between 50 and 5000),
  special       boolean not null default false,
  hidden        boolean not null default false,
  created_at    timestamptz not null default now(),
  updated_at    timestamptz not null default now()
);

alter table public.custom_bots enable row level security;
create policy "anyone can read bots" on public.custom_bots for select using (true);
create policy "admins add bots" on public.custom_bots for insert to authenticated
  with check (exists (select 1 from public.profiles p where p.id = (select auth.uid()) and p.is_admin));
create policy "admins edit bots" on public.custom_bots for update to authenticated
  using (exists (select 1 from public.profiles p where p.id = (select auth.uid()) and p.is_admin))
  with check (exists (select 1 from public.profiles p where p.id = (select auth.uid()) and p.is_admin));
create policy "admins remove bots" on public.custom_bots for delete to authenticated
  using (exists (select 1 from public.profiles p where p.id = (select auth.uid()) and p.is_admin));

revoke all on table public.custom_bots from anon, authenticated;
grant select on table public.custom_bots to anon, authenticated;
grant insert, update, delete on table public.custom_bots to authenticated;

create function private.custom_bot_stamp() returns trigger language plpgsql security definer set search_path = '' as $$
begin
  new.updated_at := now();
  return new;
end $$;
create trigger custom_bots_stamp before insert or update on public.custom_bots for each row execute function private.custom_bot_stamp();

-- Edits are visible in the audit log.
create function private.custom_bot_audit() returns trigger language plpgsql security definer set search_path = '' as $$
begin
  insert into public.mod_actions (actor_id, action, ref_kind, ref_id, detail)
  values ((select auth.uid()), 'bot_' || lower(tg_op), 'bot', coalesce(new.id, old.id), coalesce(new.name, old.name));
  return coalesce(new, old);
end $$;
create trigger custom_bots_audit after insert or update or delete on public.custom_bots for each row execute function private.custom_bot_audit();
