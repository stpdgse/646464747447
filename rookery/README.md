# Rookery

A chess website for a school class (chess.com is blocked on school wifi). Next.js on Vercel, Supabase for data and accounts.
Live: https://rookery-sable.vercel.app. See [ROADMAP.md](ROADMAP.md) for what is built and what is next.

## Run it

```bash
cp .env.example .env.local     # public Supabase URL + publishable key
npm install                    # also copies the Stockfish engine into public/engine
npm run dev                    # http://localhost:3000
```

## Check it

```bash
npm run lint && npm run typecheck
npm test                       # unit + real-Stockfish integration tests
npm run build && npm start -- -p 3111
node e2e/board.cjs /tmp        # browser tests (need Playwright and a running server)
node e2e/computer.cjs /tmp
node e2e/live.cjs /tmp         # two players, real Supabase, ~2.5 min (creates e2ea*/e2eb* accounts)
supabase/tests/run.sh          # database tests on a scratch local Postgres
```

## Layout

| Path | What |
|---|---|
| `src/lib/chess/` | Rules core: `GameRecord` (start FEN + UCI moves) replayed into views; variants, Chess960, PGN/FEN |
| `src/lib/engine/` | UCI client, bot personalities and move selection |
| `src/components/chess/` | Board, move list, appearance settings, analysis workspace |
| `src/components/computer/` | Play-the-computer game, clocks |
| `src/app/` | Pages and server actions (auth, profiles) |
| `supabase/migrations/` | Database schema, applied to the Supabase project |
| `DEPLOY.md` | Deploying to Vercel |

Rules for contributors: read the bundled Next.js docs in `node_modules/next/dist/docs/` before using Next APIs (this is Next 16);
accounts use a username mapped to a non-routable email (`@rookery.example`), so email confirmation must stay off in Supabase.

## Live games: how they are secured

- Clients can **read** `games` and `seeks`; they can never write game state (RLS plus revoked privileges).
- The Next.js server validates each move with the shared chess rules, then calls `server_*` database functions with
  `GAME_SERVER_SECRET`. The database stores only the SHA-256 of each accepted secret (`private.server_secrets`, one row per
  environment). Supabase's security advisor lists these functions as "callable by signed-in users": that is intended, they
  are useless without the secret.
- Clocks use the database clock; the server decides flag-fall, checkmate and draws.
- Realtime has a polling fallback for networks that block WebSockets.

Register a secret for an environment (SQL editor):

```sql
insert into private.server_secrets (name, hash)
values ('production', encode(extensions.digest('THE_SECRET', 'sha256'), 'hex'));
```

Remove the accounts the browser tests create (SQL editor; this also removes their games):

```sql
delete from auth.users where email like 'e2e%@rookery.example';
```
