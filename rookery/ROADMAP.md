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
- [x] **M5 Ratings**: Glicko-2 in SQL (cross-checked against the glicko2 library), rated/casual pairing, rating shown in games, leaderboard per rating type, stats profile with rating graph, PGN export
- [x] **M6 Daily games**: days per move, offers and challenges, deadlines, in-app notification bell, timeout claims plus a pg_cron sweep every 10 minutes, a separate Daily rating
- [x] **M7 Puzzles**: 11,258 puzzles sampled from the Lichess database (CC0), rated Glicko-2 puzzle rating, theme filter, daily puzzle, Puzzle Rush with server-verified runs
- [x] **M8 Analysis**: live Stockfish lines and best-move arrows, eval bar, full-game review with accuracy, inaccuracy/mistake/blunder marks and an evaluation graph; "Review with engine" on finished games
- [x] **M9 Openings**: 3,865 named openings (ECO), opening names on games and the board, opening explorer with an offline book plus what the class played (`opening_stats`)
- [x] **M10 Social**: friends and requests, blocking, online dots (heartbeat table, works behind school filters), in-game chat (players and spectators rooms), friends playing now, bell notifications
- [x] **M11 Clubs and forum**: clubs (open or by request, invites, roles, club chat), forum with categories, threads, replies, edit/delete, lock and pin
- [x] **M12 Admin and fair play**: reports queue, mute/ban, moderator role, audit log, fair-play watchlist plus in-browser Stockfish accuracy check with recorded flags
- [x] **M13 Polish**: installable PWA (manifest, icons, offline page, service worker), synthesised move/capture/check/end/notification sounds, dark/light/system theme with no flash, skip link, focus rings, reduced motion, axe-core audit on 17 pages in both themes, 30-user load test, foreign-key indexes

## Notes

- Chessground is GPL-3.0. A hosted site is not distribution, but keep the source available if the repo is ever shared.
- Lichess puzzle and opening data are CC0, so they can be imported freely.
- Stockfish is GPL-3.0 too, same note as above.

## Testing

- `node e2e/social.cjs setup|run`: friends, chat, clubs, forum, reports and moderation (needs one account made moderator, see file header).
- `node e2e/a11y.cjs`: axe-core accessibility audit (both themes), theme/sound toggles, skip link, PWA manifest and offline page.
- `SUPABASE_URL=... SUPABASE_KEY=... node e2e/load.cjs 30`: page throughput plus 30 users chatting over Realtime.
- `npm test`: unit tests (validation, chess rules, Chess960, PGN).
- `node e2e/board.cjs <screenshot-dir>`: browser test of the board against a running server (see the file header).
