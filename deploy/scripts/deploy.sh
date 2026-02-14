#!/usr/bin/env bash
set -euo pipefail

if [[ ${EUID} -ne 0 ]]; then
  echo "Run as root: sudo bash deploy/scripts/deploy.sh" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
APP_ROOT="${APP_ROOT:-/opt/othergirl}"
BACKEND_UNIT_NAME="othergirl-backend.service"
TUNNEL_UNIT_NAME="othergirl-cloudflared.service"
BACKEND_ENV_PATH="$APP_ROOT/backend/.env"
TUNNEL_ENV_PATH="/etc/othergirl/cloudflared.env"

if [[ ! -d "$REPO_ROOT/backend" || ! -d "$REPO_ROOT/deploy" ]]; then
  echo "REPO_ROOT does not look like the othergirl repository: $REPO_ROOT" >&2
  exit 1
fi

ensure_command() {
  local command_name="$1"
  local package_name="${2:-$1}"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "Missing command '$command_name'. Install package '$package_name' first." >&2
    exit 1
  fi
}

ensure_command rsync rsync
ensure_command cargo cargo
ensure_command systemctl systemd
ensure_command cloudflared cloudflared
ensure_command curl curl

echo "Syncing repository to $APP_ROOT"
mkdir -p "$APP_ROOT"
rsync -a --delete \
  --exclude ".git/" \
  --exclude "backend/target/" \
  --exclude "frontend/node_modules/" \
  --exclude "frontend/.svelte-kit/" \
  "$REPO_ROOT/" "$APP_ROOT/"

if [[ ! -f "$BACKEND_ENV_PATH" ]]; then
  echo "Missing backend env file at $BACKEND_ENV_PATH" >&2
  echo "Copy $APP_ROOT/deploy/env/backend.production.env.example to $BACKEND_ENV_PATH and fill secrets, then rerun." >&2
  exit 1
fi

if [[ ! -f "$TUNNEL_ENV_PATH" ]]; then
  echo "Missing Cloudflare tunnel env at $TUNNEL_ENV_PATH" >&2
  echo "Copy $APP_ROOT/deploy/env/cloudflared.env.example to $TUNNEL_ENV_PATH and set CLOUDFLARED_TUNNEL_TOKEN, then rerun." >&2
  exit 1
fi

chown -R root:root "$APP_ROOT"
chmod 640 "$BACKEND_ENV_PATH"
if getent group www-data >/dev/null 2>&1; then
  chgrp www-data "$BACKEND_ENV_PATH"
fi

chmod 600 "$TUNNEL_ENV_PATH"

echo "Building backend release binary"
cd "$APP_ROOT/backend"
cargo build --release

echo "Installing systemd units"
install -m 0644 \
  "$APP_ROOT/deploy/systemd/$BACKEND_UNIT_NAME" \
  "/etc/systemd/system/$BACKEND_UNIT_NAME"
install -m 0644 \
  "$APP_ROOT/deploy/systemd/$TUNNEL_UNIT_NAME" \
  "/etc/systemd/system/$TUNNEL_UNIT_NAME"

echo "Reloading services"
systemctl daemon-reload
systemctl enable --now "$BACKEND_UNIT_NAME"
systemctl restart "$BACKEND_UNIT_NAME"
systemctl enable --now redis-server || true
systemctl enable --now postgresql || true

if grep -Eq '^CLOUDFLARED_TUNNEL_TOKEN=.+$' "$TUNNEL_ENV_PATH"; then
  systemctl enable --now "$TUNNEL_UNIT_NAME"
  systemctl restart "$TUNNEL_UNIT_NAME"
else
  echo "CLOUDFLARED_TUNNEL_TOKEN is empty in $TUNNEL_ENV_PATH; tunnel service not started."
fi

echo "Checking backend health"
if curl -fsS "http://127.0.0.1:8080/health" >/dev/null; then
  echo "Backend health check passed."
else
  echo "Warning: backend health check failed. Check logs with: journalctl -u $BACKEND_UNIT_NAME -n 200 --no-pager" >&2
fi

if systemctl is-active --quiet "$TUNNEL_UNIT_NAME"; then
  echo "Cloudflare tunnel service is active."
else
  echo "Cloudflare tunnel service is not active."
  echo "Check token and logs: journalctl -u $TUNNEL_UNIT_NAME -n 200 --no-pager"
fi

echo "Deploy complete."
echo "Frontend (Cloudflare Pages) should point PUBLIC_API_BASE_URL to your Cloudflare-routed API domain."
