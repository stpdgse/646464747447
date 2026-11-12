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

## Milestones

Each milestone ends with tests passing, a verified browser run, and a check-in.

- [ ] **M0 Foundation**: scaffold, design system, site shell, Supabase project + migrations, CI, deploy
- [ ] **M1 Accounts**: sign up / in / out, profiles, username rules, sessions, password reset
- [ ] **M2 Board**: Chessground board, chess.js rules, Chess960, PGN/FEN, move list, themes
- [ ] **M3 Vs computer**: Stockfish worker, strength levels, personality bots, takebacks, save games
- [ ] **M4 Live games**: lobby, quick pairing, challenges, server clocks, resign/draw/abort, reconnect, spectate
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
