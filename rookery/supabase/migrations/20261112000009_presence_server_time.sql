-- Presence timestamps come from the database clock, never from the browser (school laptops have wrong clocks).
create function private.presence_stamp() returns trigger language plpgsql security definer set search_path = '' as $$
begin
  new.last_seen_at := now();
  return new;
end $$;
create trigger presence_stamp before insert or update on public.presence for each row execute function private.presence_stamp();
