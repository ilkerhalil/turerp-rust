# Operational scripts

This directory contains shell scripts that operators run on the host
that hosts the `turerp` docker-compose stack. They are not part of
the application runtime; they live next to the code for review
visibility.

## backup_pg.sh

Daily `pg_dump` of the turerp PostgreSQL DB to `/var/backups/turerp/`,
7-day retention. Run from `/etc/cron.d/turerp-backup` on the host.

**Safe defaults:** uses `--no-acquire --serializable-deferrable` so
the dump does not lock tables on a busy DB (see the design spec
§ 6 for the rationale). Compresses with `gzip -9`; the output is
`pg_dump --format=custom` wrapped in gzip so the file can be
restored with `pg_restore` after a `gunzip`.

**Exit codes:**

| Code | Meaning |
|------|---------|
| 0 | Backup completed and verified (file size > 0, gzip integrity OK) |
| 1 | Pre-flight failed (DB container not running, Postgres not ready) |
| 2 | `pg_dump` failed (see the log) |
| 3 | Post-write verify failed (empty or corrupt file) |

The cron `MAILTO` (if set) will receive the exit code in the cron
log; pair with monitoring that alerts on exit != 0.

### Manual run

```bash
BACKUP_DIR=/var/backups/turerp ./backup_pg.sh
```

Useful env overrides:

- `BACKUP_DIR` — where to write the dump (default `/var/backups/turerp`)
- `RETENTION_DAYS` — how long to keep old dumps (default 7)
- `DB_CONTAINER` — name of the docker container running Postgres
  (default `turerp_db_1`)
- `COMPRESS_LEVEL` — gzip level 1-9 (default 9)

### Restore

```bash
# Pick a backup file
LATEST=$(ls -t /var/backups/turerp/turerp-*.sql.gz | head -1)

# Restore into a fresh DB to verify the dump is good
docker exec -i turerp_db_1 createdb -U turerp turerp_verify
gunzip -c "$LATEST" | docker exec -i turerp_db_1 pg_restore -U turerp -d turerp_verify
docker compose exec -T db psql -U turerp -d turerp_verify -c "\dt" | head
docker exec -i turerp_db_1 dropdb -U turerp turerp_verify

# Cutover: stop app, drop+recreate prod DB, restore, start app
docker compose stop turerp
docker exec -i turerp_db_1 dropdb -U turerp turerp
docker exec -i turerp_db_1 createdb -U turerp turerp
gunzip -c "$LATEST" | docker exec -i turerp_db_1 pg_restore -U turerp -d turerp
docker compose start turerp
```

### Failure modes

- **`pg_dump` blocks on long-running transaction**: re-run with
  `--no-acquire --serializable-deferrable` (already set). If still
  blocked, kill the offending txn in `pg_stat_activity` (see
  `RUNBOOK.md` § 5 "DB connection pool exhausted").
- **Backup file is empty (0 bytes)**: the script catches this and
  exits 3. Common cause: the Postgres container exited mid-dump.
  Check `docker ps -a` and `pg_dump` exit code in
  `/var/log/turerp-backup.log`.
- **`gunzip` fails on the dump file**: gzip integrity check failed
  inside the script. The dump is corrupt — likely a disk-full
  condition. Check `df -h $BACKUP_DIR` and the kernel log for I/O
  errors.
- **Restore fails with "role does not exist"**: the dump was made
  before the `turerp` role was created (e.g. restoring an old
  dump into a fresh container). Use `--no-owner` on restore:

  ```bash
  gunzip -c "$LATEST" | docker exec -i turerp_db_1 \
      pg_restore -U turerp -d turerp --no-owner
  ```

- **Restore succeeds but the app 500s on startup**: the migration
  runner found a newer schema than the dump expected. Set
  `MIGRATIONS_DOWN=1` and restart the app once to replay the
  down-migrations, then re-run with the default env. See the
  rollback PR for the exact flag.

### Backup verification schedule

The pilot gate (`RUNBOOK.md` § 8) requires:

- A backup file in `/var/backups/turerp/` newer than 25h at all
  times.
- A weekly restore-into-fresh-DB verification, logged in
  `/var/log/turerp-restore-verify.log`.

The restore-verify cron entry is **out of scope for this script**
— it is an operator-side discipline, not a code change. Add it
to your runbook rotation.
