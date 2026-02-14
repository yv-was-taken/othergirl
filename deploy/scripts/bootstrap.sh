#!/usr/bin/env bash
set -euo pipefail

if [[ ${EUID} -ne 0 ]]; then
  echo "Run as root: sudo bash deploy/scripts/bootstrap.sh" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
APP_ROOT="${APP_ROOT:-/opt/othergirl}"
SETUP_UFW="${SETUP_UFW:-0}"

install_packages() {
  export DEBIAN_FRONTEND=noninteractive
  apt-get update
  apt-get install -y \
    apt-transport-https \
    build-essential \
    ca-certificates \
    curl \
    git \
    gnupg \
    postgresql \
    postgresql-contrib \
    redis-server \
    rsync \
    pkg-config \
    libssl-dev
}

install_cloudflared_if_missing() {
  if command -v cloudflared >/dev/null 2>&1; then
    return
  fi

  local codename
  codename="$(. /etc/os-release && echo "${VERSION_CODENAME:-}")"
  if [[ -z "$codename" ]]; then
    echo "Unable to determine Ubuntu codename for cloudflared repo setup." >&2
    exit 1
  fi

  install -d /usr/share/keyrings
  curl -fsSL https://pkg.cloudflare.com/cloudflare-main.gpg \
    | gpg --dearmor -o /usr/share/keyrings/cloudflare-main.gpg

  cat >/etc/apt/sources.list.d/cloudflared.list <<EOF
deb [signed-by=/usr/share/keyrings/cloudflare-main.gpg] https://pkg.cloudflare.com/cloudflared ${codename} main
EOF

  apt-get update
  apt-get install -y cloudflared
}

install_rust_if_missing() {
  if command -v cargo >/dev/null 2>&1; then
    return
  fi

  if [[ ! -x /root/.cargo/bin/cargo ]]; then
    curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
  fi

  export PATH="/root/.cargo/bin:$PATH"
  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo installation failed" >&2
    exit 1
  fi
}

setup_backup_cron() {
  install -d /etc/othergirl

  if [[ ! -f /etc/othergirl/backup.env ]]; then
    cp "$REPO_ROOT/deploy/env/backup.env.example" /etc/othergirl/backup.env
    chmod 600 /etc/othergirl/backup.env
  fi

  local cron_file="/etc/cron.d/othergirl-backup"
  cat >"$cron_file" <<'CRON'
SHELL=/bin/bash
PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
15 2 * * * root source /etc/othergirl/backup.env && /opt/othergirl/deploy/scripts/backup_postgres.sh >> /var/log/othergirl-backup.log 2>&1
CRON
  chmod 644 "$cron_file"
}

setup_cloudflared_env() {
  install -d /etc/othergirl
  if [[ ! -f /etc/othergirl/cloudflared.env ]]; then
    cp "$REPO_ROOT/deploy/env/cloudflared.env.example" /etc/othergirl/cloudflared.env
    chmod 600 /etc/othergirl/cloudflared.env
  fi
}

setup_dirs() {
  install -d "$APP_ROOT"
  install -d "$APP_ROOT/backend/uploads/emotes"
}

setup_firewall_if_enabled() {
  if [[ "$SETUP_UFW" != "1" ]]; then
    return
  fi

  apt-get install -y ufw
  ufw default deny incoming
  ufw default allow outgoing
  ufw allow OpenSSH
  ufw --force enable
}

run_deploy() {
  export PATH="/root/.cargo/bin:$PATH"
  APP_ROOT="$APP_ROOT" REPO_ROOT="$REPO_ROOT" \
    bash "$REPO_ROOT/deploy/scripts/deploy.sh"
}

echo "Installing base packages"
install_packages

echo "Installing cloudflared"
install_cloudflared_if_missing

echo "Ensuring Rust toolchain"
install_rust_if_missing

echo "Preparing directories"
setup_dirs

echo "Setting up environment scaffolding"
setup_cloudflared_env
setup_backup_cron

echo "Optional firewall setup"
setup_firewall_if_enabled

if [[ ! -f "$APP_ROOT/backend/.env" ]]; then
  echo "Missing backend env file: $APP_ROOT/backend/.env"
  echo "Create it from template before deploy:"
  echo "  cp $REPO_ROOT/deploy/env/backend.production.env.example $APP_ROOT/backend/.env"
  echo "  nano $APP_ROOT/backend/.env"
  exit 1
fi

echo "Running initial deploy"
run_deploy

echo "Bootstrap complete."
echo "Next steps:"
echo "  1) Put tunnel token in /etc/othergirl/cloudflared.env (CLOUDFLARED_TUNNEL_TOKEN=...)"
echo "  2) Configure Cloudflare Tunnel public hostname to http://localhost:8080"
echo "  3) Re-run: sudo bash deploy/scripts/deploy.sh"
