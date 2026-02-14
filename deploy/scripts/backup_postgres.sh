#!/usr/bin/env bash
set -euo pipefail

BACKUP_DIR="/var/backups/othergirl"
DATE_STAMP="$(date +%Y%m%d_%H%M%S)"
DB_NAME="${PGDATABASE:-othergirl}"
DB_USER="${PGUSER:-postgres}"
STORAGE_TARGET="${STORAGE_TARGET:-}"

mkdir -p "$BACKUP_DIR"

FILE="$BACKUP_DIR/${DB_NAME}_${DATE_STAMP}.sql.gz"
pg_dump -U "$DB_USER" "$DB_NAME" | gzip > "$FILE"

# keep 14 days locally
find "$BACKUP_DIR" -type f -name '*.sql.gz' -mtime +14 -delete

if [[ -n "$STORAGE_TARGET" ]]; then
  rsync -av "$FILE" "$STORAGE_TARGET"
fi

echo "backup written: $FILE"
