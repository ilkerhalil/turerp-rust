#!/usr/bin/env bash
# turerp/scripts/backup_pg.sh
# Daily pg_dump to /var/backups/turerp/, 7-day retention.
#
# Flags:
#   --no-acquire              no table locks
#   --serializable-deferrable wait for concurrent txns to finish
# These are the safe flags per the production-release design spec
# § 6 to avoid locking tables on a busy DB.
#
# Cron: see backup_pg.cron. Manual runs are supported and verify-safe
# (the dump goes to $BACKUP_DIR; the live DB is read-only via
# pg_dump's MVCC snapshot).
#
# Exit codes:
#   0  backup completed and verified (file size > 0)
#   1  pre-flight check failed (DB container not running, etc.)
#   2  pg_dump failed (see /var/log/turerp-backup.log)
#   3  post-write verify failed (file is empty or corrupt)
set -euo pipefail

BACKUP_DIR="${BACKUP_DIR:-/var/backups/turerp}"
RETENTION_DAYS="${RETENTION_DAYS:-7}"
DB_CONTAINER="${DB_CONTAINER:-turerp_db_1}"
COMPRESS_LEVEL="${COMPRESS_LEVEL:-9}"

mkdir -p "$BACKUP_DIR"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="$BACKUP_DIR/turerp-$TIMESTAMP.sql.gz"

log() {
    echo "[$(date -Iseconds)] $*"
}

# --- pre-flight ---
if ! docker inspect "$DB_CONTAINER" >/dev/null 2>&1; then
    log "ERROR: container '$DB_CONTAINER' is not running. Aborting."
    exit 1
fi

if ! docker exec "$DB_CONTAINER" pg_isready -U turerp >/dev/null 2>&1; then
    log "ERROR: Postgres is not ready in '$DB_CONTAINER'. Aborting."
    exit 1
fi

log "starting backup → $OUT"

# --- dump ---
if ! docker exec "$DB_CONTAINER" \
    pg_dump -U turerp -d turerp \
        --no-acquire --serializable-deferrable \
        --format=custom \
    | gzip -"$COMPRESS_LEVEL" \
    > "$OUT"; then
    log "ERROR: pg_dump failed; removing partial file $OUT"
    rm -f "$OUT"
    exit 2
fi

# --- post-write verify ---
if [ ! -s "$OUT" ]; then
    log "ERROR: backup file $OUT is empty. Aborting."
    rm -f "$OUT"
    exit 3
fi

# gzip integrity check
if ! gzip -t "$OUT" 2>/dev/null; then
    log "ERROR: backup file $OUT failed gzip integrity check. Aborting."
    rm -f "$OUT"
    exit 3
fi

log "backup complete: $(du -h "$OUT" | cut -f1)"

# --- prune ---
find "$BACKUP_DIR" -name "turerp-*.sql.gz" -mtime "+${RETENTION_DAYS}" -delete
log "pruned backups older than ${RETENTION_DAYS} days"
log "remaining backups:"
ls -lh "$BACKUP_DIR"/turerp-*.sql.gz 2>/dev/null | awk '{print "  " $9 " (" $5 ")"}' || true
