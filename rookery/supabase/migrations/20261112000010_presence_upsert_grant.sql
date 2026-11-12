-- PostgREST upserts set every supplied column, so user_id must be updatable too (the policy still pins it to the caller).
grant update (user_id) on table public.presence to authenticated;
