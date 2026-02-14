#!/usr/bin/env bash
set -euo pipefail

APP_ROOT="/opt/othergirl"
BACKEND_DIR="$APP_ROOT/backend"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required" >&2
  exit 1
fi

echo "Building backend"
cd "$BACKEND_DIR"
cargo build --release

echo "Restarting services"
sudo systemctl daemon-reload
sudo systemctl restart othergirl-backend.service

echo "Backend deploy complete"
echo "Frontend is deployed separately via Cloudflare Pages."
