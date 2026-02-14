#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$SCRIPT_DIR/backend"
FRONTEND_DIR="$SCRIPT_DIR/frontend"

PG_CONTAINER="${PG_CONTAINER:-othergirl-postgres}"
PG_IMAGE="${PG_IMAGE:-postgres:16}"
PG_PORT="${PG_PORT:-5432}"
PG_USER="${PG_USER:-postgres}"
PG_PASSWORD="${PG_PASSWORD:-postgres}"
PG_DB="${PG_DB:-othergirl}"

REDIS_CONTAINER="${REDIS_CONTAINER:-othergirl-redis}"
REDIS_IMAGE="${REDIS_IMAGE:-redis:7}"
REDIS_PORT="${REDIS_PORT:-6379}"

FORCE_INSTALL=0
SKIP_INSTALL=0
TMUX_MODE="auto"
TMUX_SESSION="${TMUX_SESSION:-othergirl-dev}"

BACKEND_PID=""
FRONTEND_PID=""

log() {
  printf '[dev] %s\n' "$*"
}

die() {
  printf '[dev] ERROR: %s\n' "$*" >&2
  exit 1
}

usage() {
  cat <<'EOF'
Usage: ./dev.sh [options]

Options:
  --install       Force `bun install` before starting frontend
  --skip-install  Skip `bun install` even if node_modules is missing
  --tmux          Run backend/frontend in separate tmux windows
  --no-tmux       Force single-terminal mode
  --tmux-session  tmux session name (default: othergirl-dev)
  --help          Show this help message

Environment overrides:
  PG_CONTAINER, PG_IMAGE, PG_PORT, PG_USER, PG_PASSWORD, PG_DB
  REDIS_CONTAINER, REDIS_IMAGE, REDIS_PORT
  TMUX_SESSION
EOF
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --install)
        FORCE_INSTALL=1
        shift
        ;;
      --skip-install)
        SKIP_INSTALL=1
        shift
        ;;
      --tmux)
        TMUX_MODE="on"
        shift
        ;;
      --no-tmux)
        TMUX_MODE="off"
        shift
        ;;
      --tmux-session)
        [[ $# -lt 2 ]] && die "--tmux-session requires a value"
        TMUX_SESSION="$2"
        shift 2
        ;;
      --help|-h)
        usage
        exit 0
        ;;
      *)
        die "Unknown option: $1"
        ;;
    esac
  done
}

require_cmd() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    die "Missing command '$command_name'"
  fi
}

ensure_docker_daemon() {
  if docker info >/dev/null 2>&1; then
    return
  fi

  log "Docker daemon is not running; attempting to start it via systemd."
  if command -v sudo >/dev/null 2>&1 && command -v systemctl >/dev/null 2>&1; then
    sudo systemctl enable --now docker >/dev/null 2>&1 || true
  fi

  if ! docker info >/dev/null 2>&1; then
    die "Docker daemon is unavailable. Start Docker and rerun."
  fi
}

ensure_container() {
  local name="$1"
  local run_cmd="$2"

  if docker ps -a --format '{{.Names}}' | grep -Fxq "$name"; then
    if [[ "$(docker inspect -f '{{.State.Running}}' "$name")" != "true" ]]; then
      log "Starting existing container: $name"
      docker start "$name" >/dev/null
    fi
  else
    log "Creating container: $name"
    # shellcheck disable=SC2086
    eval "$run_cmd" >/dev/null
  fi
}

wait_for_postgres() {
  log "Waiting for PostgreSQL to become ready..."
  for _ in $(seq 1 60); do
    if docker exec "$PG_CONTAINER" pg_isready -U "$PG_USER" -d "$PG_DB" >/dev/null 2>&1; then
      log "PostgreSQL is ready."
      return
    fi
    sleep 1
  done
  die "PostgreSQL did not become ready in time."
}

wait_for_redis() {
  log "Waiting for Redis to become ready..."
  for _ in $(seq 1 30); do
    if docker exec "$REDIS_CONTAINER" redis-cli ping 2>/dev/null | grep -q PONG; then
      log "Redis is ready."
      return
    fi
    sleep 1
  done
  die "Redis did not become ready in time."
}

ensure_env_files() {
  if [[ ! -f "$BACKEND_DIR/.env" ]]; then
    log "Creating backend/.env from backend/.env.example"
    cp "$BACKEND_DIR/.env.example" "$BACKEND_DIR/.env"
  fi

  if [[ ! -f "$FRONTEND_DIR/.env" ]]; then
    log "Creating frontend/.env from frontend/.env.example"
    cp "$FRONTEND_DIR/.env.example" "$FRONTEND_DIR/.env"
  fi
}

maybe_install_frontend_deps() {
  if [[ "$SKIP_INSTALL" -eq 1 ]]; then
    log "Skipping Bun install (--skip-install)."
    return
  fi

  if [[ "$FORCE_INSTALL" -eq 1 || ! -d "$FRONTEND_DIR/node_modules" ]]; then
    log "Installing frontend dependencies with Bun..."
    (
      cd "$FRONTEND_DIR"
      bun install
    )
  fi
}

should_use_tmux() {
  if [[ "$TMUX_MODE" == "off" ]]; then
    return 1
  fi

  if [[ "$TMUX_MODE" == "on" ]]; then
    return 0
  fi

  command -v tmux >/dev/null 2>&1
}

start_servers_tmux() {
  require_cmd tmux

  if tmux has-session -t "$TMUX_SESSION" 2>/dev/null; then
    log "Killing existing tmux session: $TMUX_SESSION"
    tmux kill-session -t "$TMUX_SESSION"
  fi

  log "Starting backend/frontend in tmux panes: $TMUX_SESSION"
  tmux new-session -d -s "$TMUX_SESSION" -n dev \
    "cd \"$BACKEND_DIR\" && cargo run; exec bash"
  tmux split-window -h -t "$TMUX_SESSION:dev" \
    "cd \"$FRONTEND_DIR\" && bun run dev; exec bash"
  tmux select-layout -t "$TMUX_SESSION:dev" even-horizontal

  log "Backend:  http://localhost:8080"
  log "Frontend: http://localhost:3000"
  log "Attach with: tmux attach -t $TMUX_SESSION"
  log "Stop with:   tmux kill-session -t $TMUX_SESSION"

  if [[ -n "${TMUX:-}" ]]; then
    tmux switch-client -t "$TMUX_SESSION"
  else
    tmux attach-session -t "$TMUX_SESSION"
  fi
}

start_servers() {
  log "Starting backend (cargo run)..."
  (
    cd "$BACKEND_DIR"
    cargo run
  ) &
  BACKEND_PID=$!

  log "Starting frontend (bun run dev)..."
  (
    cd "$FRONTEND_DIR"
    bun run dev
  ) &
  FRONTEND_PID=$!

  log "Backend:  http://localhost:8080"
  log "Frontend: http://localhost:3000"
  log "Press Ctrl+C to stop both dev servers."
}

cleanup() {
  trap - INT TERM EXIT
  if [[ -n "$FRONTEND_PID" ]] && kill -0 "$FRONTEND_PID" >/dev/null 2>&1; then
    kill "$FRONTEND_PID" >/dev/null 2>&1 || true
  fi
  if [[ -n "$BACKEND_PID" ]] && kill -0 "$BACKEND_PID" >/dev/null 2>&1; then
    kill "$BACKEND_PID" >/dev/null 2>&1 || true
  fi
  wait >/dev/null 2>&1 || true
}

main() {
  parse_args "$@"

  require_cmd docker
  require_cmd cargo
  require_cmd bun

  ensure_docker_daemon

  ensure_container \
    "$PG_CONTAINER" \
    "docker run --name $PG_CONTAINER -e POSTGRES_USER=$PG_USER -e POSTGRES_PASSWORD=$PG_PASSWORD -e POSTGRES_DB=$PG_DB -p $PG_PORT:5432 -d $PG_IMAGE"

  ensure_container \
    "$REDIS_CONTAINER" \
    "docker run --name $REDIS_CONTAINER -p $REDIS_PORT:6379 -d $REDIS_IMAGE"

  wait_for_postgres
  wait_for_redis

  ensure_env_files
  maybe_install_frontend_deps

  if should_use_tmux; then
    start_servers_tmux
    exit 0
  fi

  trap cleanup INT TERM EXIT
  start_servers

  set +e
  wait -n "$BACKEND_PID" "$FRONTEND_PID"
  local status=$?
  set -e

  if [[ $status -ne 0 ]]; then
    log "One dev process exited with status $status. Shutting down both."
  fi

  cleanup
  exit "$status"
}

main "$@"
