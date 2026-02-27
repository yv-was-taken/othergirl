#!/usr/bin/env bash
set -euo pipefail

BACKUP_DIR="/var/backups/othergirl"
DATE_STAMP="$(date +%Y%m%d_%H%M%S)"
DB_NAME="${PGDATABASE:-othergirl}"
DB_USER="${PGUSER:-postgres}"
DB_HOST="${PGHOST:-127.0.0.1}"
DB_PORT="${PGPORT:-5432}"
STORAGE_TARGET="${STORAGE_TARGET:-}"
LOCK_FILE="/tmp/othergirl-backup.lock"

mkdir -p "$BACKUP_DIR"

exec 9>"$LOCK_FILE"
if ! flock -n 9; then
  echo "another backup process is running; exiting" >&2
  exit 75  # EX_TEMPFAIL — lets cron/monitoring detect the skip
fi

FILE="$BACKUP_DIR/${DB_NAME}_${DATE_STAMP}.sql.gz"
pg_dump -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" "$DB_NAME" | gzip > "$FILE"

# keep 14 days locally
find "$BACKUP_DIR" -type f -name '*.sql.gz' -mtime +14 -delete

if [[ -n "$STORAGE_TARGET" ]]; then
  rsync -av "$FILE" "$STORAGE_TARGET"
fi

echo "backup written: $FILE"
