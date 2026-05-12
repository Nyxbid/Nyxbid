#!/usr/bin/env bash
#
# Bounce the local nyxbid-server on :8080 and run it again from
# source. Usage: ./scripts/restart-server.sh
#
# What it does:
#   1. kills whatever is listening on PORT (default 8080),
#   2. rebuilds the server (release by default),
#   3. starts a fresh process attached to your terminal so logs are
#      visible and Ctrl+C cleanly stops it.
#
# After `git pull`, run this and the new code is what the browser
# talks to. The previous binary keeps running on the port until it is
# killed — `git pull` alone changes nothing.

set -euo pipefail

PORT="${PORT:-8080}"
PROFILE="${PROFILE:-release}"

cd "$(dirname "$0")/.."

echo "[restart-server] freeing port $PORT"
PIDS="$(lsof -nP -iTCP:"$PORT" -sTCP:LISTEN -t 2>/dev/null || true)"
if [ -n "$PIDS" ]; then
  # shellcheck disable=SC2086
  kill $PIDS 2>/dev/null || true
  sleep 1
  PIDS="$(lsof -nP -iTCP:"$PORT" -sTCP:LISTEN -t 2>/dev/null || true)"
  if [ -n "$PIDS" ]; then
    # shellcheck disable=SC2086
    kill -9 $PIDS 2>/dev/null || true
  fi
fi

if [ "$PROFILE" = "release" ]; then
  echo "[restart-server] cargo run --release -p nyxbid-server"
  exec cargo run --release -p nyxbid-server
else
  echo "[restart-server] cargo run -p nyxbid-server"
  exec cargo run -p nyxbid-server
fi
