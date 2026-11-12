# Rookery: roadmap

A full chess site for a school class, because chess.com is blocked on school wifi.
Real server-side app: **Next.js (App Router) + TypeScript** on **Vercel**, with **Supabase** (Postgres, Auth, Realtime).
"Rookery" is a working name and lives in one constant (`src/lib/site.ts`), so renaming is one edit.

## Decisions (from the planning Q&A)

| Area | Decision |
|---|---|
| Audience | Class of ~30, architecture should not need a rewrite to grow |
| Accounts | Full accounts: username + password, profile, rating, history, friends |
| Modes | Live games, vs computer, daily/correspondence, Chess960 |
| Time controls | Bullet / Blitz / Rapid presets plus custom clocks |
| Bots | Stockfish strength levels (~400-3200) plus named personality bots |
| Learning | Rated puzzles, game review / analysis board, opening explorer |
| Social | Friends, in-game chat, spectating, Glicko-2 leaderboards, clubs, forum |
| Admin | Manual moderation panel, engine-accuracy fair-play flags for admins |
| Look | Dark and green, clean and modern; board/piece themes; mobile-first; installable PWA |
| Realtime | Supabase Realtime, server-authoritative state; long-poll fallback if a school filter blocks WebSockets |
| Hosting | Vercel (site) and Supabase (data) |

## Architecture rules

1. **The server owns the truth.** Clients never write game state. Moves go through a route handler that validates
   with chess.js, stamps the server clock, and writes the move; clients only *render* what Realtime sends.
2. **Clocks are timestamps, not timers.** A game stores `clock_ms` per side and `last_move_at`. Time left is derived,
   so reconnects and refreshes are always correct. Flag-fall is checked lazily on every read and by a scheduled sweep.
3. **One host.** The browser talks to our own origin wherever possible, because school filters allow-list by domain.
4. **Row Level Security on every table.** Nothing is readable or writable by default.
5. **Stockfish runs in the browser** (WASM in a Web Worker); the server never runs an engine on the request path.

## Hosting

- Vercel project `rookery` (team `tforzas-projects`), root directory `rookery`, linked to this GitHub repo.
- Production branch is `main`: merging the PR is what updates the public site. Other branches get preview URLs that sit behind Vercel login.
- Supabase project `rookery` (`ultpyrvfzdqamprglnxd`, eu-central-1). Email confirmation is off by design (usernames, no real emails).
- Board uses the maintained `@lichess-org/chessground` and `chessops` (rules for all variants).

## Milestones

Each milestone ends with tests passing, a verified browser run, and a check-in.

- [x] **M0 Foundation**: scaffold, design system, site shell, Supabase project + migrations, CI, Vercel deploy (live: https://rookery-sable.vercel.app)
- [x] **M1 Accounts**: sign up / in / out, profiles, settings, RLS (password reset will be admin-assisted, since there are no real emails)
- [x] **M2 Board**: Chessground board, chessops rules (replaces chess.js, which lacks Chess960), Chess960, King of the Hill, Three-check, PGN/FEN, move list, 6 board themes, 8 piece sets
- [x] **M3 Vs computer**: Stockfish 19 (lite, single-thread WASM, no special headers) in a Web Worker, 8 personality bots, clocks, takebacks, resign/draw, Chess960, hand-off to analysis. Saving games to the database arrives with M4's `games` table
- [x] **M4 Live games**: quick pairing, open games, challenges, rematch, DB-clock timing, flag-fall, resign/abort/draw, realtime with polling fallback, spectating, Watch page. Rated games arrive with M5
- [ ] **M5 Ratings**: Glicko-2 per time control, leaderboards, stats profile, rating graph, PGN download
- [ ] **M6 Daily games**: correspondence with days-per-move, notifications, timeout sweep
- [ ] **M7 Puzzles**: Lichess puzzle DB import, rated puzzles, puzzle rush, daily puzzle
- [ ] **M8 Analysis**: engine eval bar, best-move arrows, accuracy score, blunder labels, game review
- [ ] **M9 Openings**: ECO names, move-tree explorer from site games
- [ ] **M10 Social**: friends, in-game chat, presence, spectator view, notifications
- [ ] **M11 Clubs and forum**: clubs/teams, club chat, forum categories/threads/posts
- [ ] **M12 Admin and fair play**: moderation panel, bans/mutes/reports, accuracy flags
- [ ] **M13 Polish**: PWA, sounds, themes, accessibility, performance, load test

## Notes

- Chessground is GPL-3.0. A hosted site is not distribution, but keep the source available if the repo is ever shared.
- Lichess puzzle and opening data are CC0, so they can be imported freely.
- Stockfish is GPL-3.0 too, same note as above.

## Testing

- `npm test`: unit tests (validation, chess rules, Chess960, PGN).
- `node e2e/board.cjs <screenshot-dir>`: browser test of the board against a running server (see the file header).
