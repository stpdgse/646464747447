-- Rookery M4 (part 2): pairing functions and the API surface.
-- Split from part 1 so the statements that contain DELETE can be applied on their own.

/** Create the game for an accepted seek. Internal. */
create function private.start_game(s public.seeks, p_joiner uuid, p_fen text)
returns public.games language plpgsql security definer set search_path = '' as $$
declare creator_white boolean; g public.games;
begin
  if exists (
    select 1 from public.games x
    where x.status = 'started' and (x.white_id in (p_joiner, s.user_id) or x.black_id in (p_joiner, s.user_id))
  ) then
    raise exception 'busy';
  end if;
  creator_white := case s.color when 'white' then true when 'black' then false else random() < 0.5 end;
  insert into public.games (variant, initial_fen, white_id, black_id, time_initial_ms, time_inc_ms, white_clock_ms, black_clock_ms)
  values (s.variant, p_fen,
          case when creator_white then s.user_id else p_joiner end,
          case when creator_white then p_joiner else s.user_id end,
          s.time_initial_ms, s.time_inc_ms, s.time_initial_ms, s.time_initial_ms)
  returning * into g;
  delete from public.seeks where id = s.id or (target_id is null and user_id in (p_joiner, s.user_id));
  return g;
end $$;

/** Pair with a compatible open seek, or become one. Atomic per time control, so two people can never miss each other. */
create function public.server_quick_pair(p_secret text, p_variant text, p_time_ms integer, p_inc_ms integer, p_initial_fen text)
returns jsonb language plpgsql security definer set search_path = '' as $$
declare uid uuid; s public.seeks; g public.games; mine public.seeks;
begin
  perform private.check_secret(p_secret);
  uid := private.require_user();
  perform pg_advisory_xact_lock(hashtext(p_variant || ':' || p_time_ms || ':' || p_inc_ms));

  select * into s from public.seeks x
  where x.target_id is null and x.user_id <> uid and x.variant = p_variant
    and x.time_initial_ms = p_time_ms and x.time_inc_ms = p_inc_ms and x.expires_at > now()
    and not exists (select 1 from public.games y where y.status = 'started' and (y.white_id = x.user_id or y.black_id = x.user_id))
  order by x.created_at limit 1 for update skip locked;

  if found then
    g := private.start_game(s, uid, p_initial_fen);
    return jsonb_build_object('game', to_jsonb(g));
  end if;

  if exists (select 1 from public.games y where y.status = 'started' and (y.white_id = uid or y.black_id = uid)) then
    raise exception 'busy';
  end if;
  delete from public.seeks where user_id = uid and target_id is null;
  insert into public.seeks (user_id, variant, time_initial_ms, time_inc_ms)
  values (uid, p_variant, p_time_ms, p_inc_ms) returning * into mine;
  return jsonb_build_object('seek', to_jsonb(mine));
end $$;

-- API surface: the server_* functions are callable through the API but useless without the server secret.
revoke execute on all functions in schema public from public, anon, authenticated;
grant execute on function public.server_time() to anon, authenticated;
grant execute on function public.server_accept_seek(text, uuid, text) to authenticated;
grant execute on function public.server_quick_pair(text, text, integer, integer, text) to authenticated;
grant execute on function public.server_move(text, uuid, text, integer, text, text) to authenticated;
grant execute on function public.server_resign(text, uuid) to authenticated;
grant execute on function public.server_abort(text, uuid) to authenticated;
grant execute on function public.server_draw(text, uuid, text) to authenticated;
grant execute on function public.server_claim_timeout(text, uuid, boolean) to authenticated;
grant execute on function public.is_admin() to authenticated;
