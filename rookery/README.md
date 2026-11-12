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
