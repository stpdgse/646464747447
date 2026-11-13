#!/usr/bin/env bash
# Tests the database migrations on a throwaway local Postgres (needs the postgresql-16 server binaries).
# Usage: supabase/tests/run.sh
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PGBIN="$(ls -d /usr/lib/postgresql/*/bin | sort -V | tail -1)"
DIR="$(mktemp -d /var/tmp/rookery-pg.XXXXXX)"
PORT=54331
chown postgres:postgres "$DIR" 2>/dev/null || true
cleanup() { su postgres -c "$PGBIN/pg_ctl -D $DIR/data stop -m immediate" >/dev/null 2>&1 || true; rm -rf "$DIR"; }
trap cleanup EXIT
su postgres -c "$PGBIN/initdb -D $DIR/data -A trust >/dev/null && $PGBIN/pg_ctl -D $DIR/data -o '-p $PORT -k $DIR' -l $DIR/log -w start >/dev/null"
PSQL=(psql -h "$DIR" -p "$PORT" -U postgres -X -q -v ON_ERROR_STOP=1)
"${PSQL[@]}" -c "create database app" >/dev/null
PSQL+=(-d app)
"${PSQL[@]}" -f "$HERE/stubs.sql"
for f in "$HERE"/../migrations/*.sql; do echo "-- applying $(basename "$f")"; "${PSQL[@]}" -f "$f"; done
"${PSQL[@]}" -f "$HERE/live_games.sql" 2>&1 | grep -E "^(psql:|NOTICE:  ok|NOTICE:  FAILED|ERROR|ALL )|FAILED" | sed -e "s/^NOTICE:  //" -e "s/^psql:[^ ]* //"
"${PSQL[@]}" -f "$HERE/ratings.sql" 2>&1 | grep -E "^(psql:|NOTICE:  ok|NOTICE:  FAILED|ERROR|ALL )|FAILED" | sed -e "s/^NOTICE:  //" -e "s/^psql:[^ ]* //"
"${PSQL[@]}" -f "$HERE/daily_puzzles.sql" 2>&1 | grep -E "^(psql:|NOTICE:  ok|NOTICE:  FAILED|ERROR|ALL )|FAILED" | sed -e "s/^NOTICE:  //" -e "s/^psql:[^ ]* //"
"${PSQL[@]}" -f "$HERE/openings.sql" 2>&1 | grep -E "^(psql:|NOTICE:  ok|NOTICE:  FAILED|ERROR|ALL )|FAILED" | sed -e "s/^NOTICE:  //" -e "s/^psql:[^ ]* //"
"${PSQL[@]}" -f "$HERE/social.sql" 2>&1 | grep -E "^(psql:|NOTICE:  ok|NOTICE:  FAILED|ERROR|ALL )|FAILED" | sed -e "s/^NOTICE:  //" -e "s/^psql:[^ ]* //"
"${PSQL[@]}" -f "$HERE/bot_games.sql" 2>&1 | grep -E "^(psql:|NOTICE:  ok|NOTICE:  FAILED|ERROR|ALL )|FAILED" | sed -e "s/^NOTICE:  //" -e "s/^psql:[^ ]* //"
"${PSQL[@]}" -f "$HERE/custom_bots.sql" 2>&1 | grep -E "^(psql:|NOTICE:  ok|NOTICE:  FAILED|ERROR|ALL )|FAILED" | sed -e "s/^NOTICE:  //" -e "s/^psql:[^ ]* //"
"${PSQL[@]}" -f "$HERE/tournaments.sql" 2>&1 | grep -E "^(psql:|NOTICE:  ok|NOTICE:  FAILED|ERROR|ALL )|FAILED" | sed -e "s/^NOTICE:  //" -e "s/^psql:[^ ]* //"
"${PSQL[@]}" -f "$HERE/achievements.sql" 2>&1 | grep -E "^(psql:|NOTICE:  ok|NOTICE:  FAILED|ERROR|ALL )|FAILED" | sed -e "s/^NOTICE:  //" -e "s/^psql:[^ ]* //"
