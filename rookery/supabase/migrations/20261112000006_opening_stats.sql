-- Rookery M9: what the class plays. For a given opening line (the first moves, in UCI), which moves have been played
-- next in finished standard games, and how did those games end? Games are public, so this reads with the caller's rights.

create function public.opening_stats(p_prefix text[])
returns table (move text, games bigint, white_wins bigint, draws bigint, black_wins bigint)
language sql stable security invoker set search_path = '' as $$
  select g.moves[cardinality(p_prefix) + 1] as move,
         count(*) as games,
         count(*) filter (where g.result = '1-0') as white_wins,
         count(*) filter (where g.result = '1/2-1/2') as draws,
         count(*) filter (where g.result = '0-1') as black_wins
    from public.games g
   where g.variant = 'standard'
     and g.status = 'finished'
     and cardinality(p_prefix) <= 40
     and cardinality(g.moves) > cardinality(p_prefix)
     and g.moves[1:cardinality(p_prefix)] = p_prefix
   group by 1
   order by 2 desc, 1
   limit 25
$$;

revoke execute on function public.opening_stats(text[]) from public;
grant execute on function public.opening_stats(text[]) to anon, authenticated;
