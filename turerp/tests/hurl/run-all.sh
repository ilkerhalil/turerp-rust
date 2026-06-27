#!/usr/bin/env bash
# Runs the hurl smoke suite against a running turerp instance.
#
# Architecture:
#   1. Login once via curl, capture access_token, write to a temp .env file
#   2. Pass the .env file to every hurl invocation via --variables-file
#   3. Each NN_*.hurl is a standalone file that imports {{access_token}},
#      {{refresh_token}}, {{user_id}}, {{tenant_id}} from the env file
#
# Why not hurl's own multi-file feature? Hurl 8.x does not support
# cross-file capture sharing. We use an external login + variables-file
# pattern instead.
#
# Usage:
#   BASE_URL=http://127.0.0.1:8080 \
#   TURERP_TEST_PASSWORD='TestUser123!Pass' \
#   ./run-all.sh
#
# Exit code: 0 = all green, non-zero = at least one assertion failed.

set -uo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
TEST_PASSWORD="${TURERP_TEST_PASSWORD:-TestUser123!Pass}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

if ! command -v hurl >/dev/null 2>&1; then
    echo "ERROR: hurl not found. Install with: cargo install hurl --locked" >&2
    exit 2
fi
if ! command -v jq >/dev/null 2>&1; then
    echo "ERROR: jq not found. Install with: sudo apt install jq" >&2
    exit 2
fi

# --- 1. Login once, capture tokens, write to a temp variables file ---
VARS_FILE="$(mktemp /tmp/turerp-hurl-vars.XXXXXX.env)"
# Ensure token file is removed on normal exit AND when interrupted
# (Ctrl-C / SIGTERM) — otherwise it sits in /tmp until the OS cleans it.
trap 'rm -f "$VARS_FILE"' EXIT INT TERM
chmod 600 "$VARS_FILE"

echo
echo "=== login (one-time) ==="
LOGIN_JSON=$(curl -sf -X POST "${BASE_URL}/api/v1/auth/login?tenant_id=1" \
    -H 'Content-Type: application/json' \
    -d "{\"username\":\"testuser\",\"password\":\"${TEST_PASSWORD}\"}" \
    2>&1) || {
    echo "FATAL: login failed. Ensure the testuser exists in tenant 1." >&2
    echo "Register it first:" >&2
    echo "  curl -X POST ${BASE_URL}/api/v1/auth/register \\" >&2
    echo "    -H 'Content-Type: application/json' \\" >&2
    echo "    -d '{\"username\":\"testuser\",\"email\":\"testuser@turerp.local\",\"password\":\"<set TURERP_TEST_PASSWORD>\",\"tenant_id\":1,\"full_name\":\"Test User\"}'" >&2
    rm -f "$VARS_FILE"
    exit 1
}

ACCESS=$(echo "$LOGIN_JSON" | jq -r '.tokens.access_token')
REFRESH=$(echo "$LOGIN_JSON" | jq -r '.tokens.refresh_token')
USER_ID=$(echo "$LOGIN_JSON" | jq -r '.user.id')
TENANT_ID=$(echo "$LOGIN_JSON" | jq -r '.user.tenant_id')

# --- 1b. Ensure the admin test user exists, then log in (write-fence scenarios) ---
# Self-registration forces role=user even when Admin is requested
# (src/domain/auth/service.rs), so an admin can only be provisioned at the DB
# level. seed_test_admin.sql clones testuser's password hash into an admin
# account that shares the SAME password. Best-effort: try via docker compose
# (the stack already runs under docker compose); if unavailable, the admin
# login below simply fails and write-fence scenarios will 401/403 — the
# standard read-only scenarios still run. Fix by running seed_test_admin.sql
# against the DB manually.
SEED_SCRIPT="${SCRIPT_DIR}/../../scripts/seed_test_admin.sql"
if [ -f "$SEED_SCRIPT" ] && docker compose version >/dev/null 2>&1; then
    # Run from the turerp dir so `docker compose` finds docker-compose.yml.
    ( cd "$SCRIPT_DIR/.." && docker compose exec -T db psql -U turerp -d turerp ) \
        < "$SEED_SCRIPT" >/dev/null 2>&1 || true
fi

ADMIN_ACCESS=""
ADMIN_LOGIN_JSON=$(curl -sf -X POST "${BASE_URL}/api/v1/auth/login?tenant_id=1" \
    -H 'Content-Type: application/json' \
    -d "{\"username\":\"testadmin\",\"password\":\"${TEST_PASSWORD}\"}" 2>&1) || true
if [ -n "$ADMIN_LOGIN_JSON" ]; then
    ADMIN_ACCESS=$(echo "$ADMIN_LOGIN_JSON" | jq -r '.tokens.access_token // empty')
fi
if [ -n "$ADMIN_ACCESS" ]; then
    echo "  admin_token: ${ADMIN_ACCESS:0:30}... (write-fence scenarios enabled)"
else
    echo "  admin_token: UNAVAILABLE — write-fence scenarios will 401/403." >&2
    echo "    Seed the admin: docker compose exec -T db psql -U turerp -d turerp -f scripts/seed_test_admin.sql" >&2
fi

# Hurl --variables-file uses simple key=value env-style format.
cat > "$VARS_FILE" <<EOF
base_url=${BASE_URL}
test_password=${TEST_PASSWORD}
access_token=${ACCESS}
admin_token=${ADMIN_ACCESS}
refresh_token=${REFRESH}
user_id=${USER_ID}
tenant_id=${TENANT_ID}
EOF

echo "  access_token: ${ACCESS:0:30}..."
echo "  user_id: $USER_ID, tenant_id: $TENANT_ID"

HURL_OPTS=(
    --test
    --variables-file "$VARS_FILE"
    --color
)

# --- 2. Run all numbered scenarios in order ---
FAILED=0
PASSED=0
TOTAL=0
FAILED_FILES=()

for scenario in [0-9][0-9]*.hurl; do
    # Admin-gated scenarios — *_write.hurl (create/update/delete) and
    # *_admin.hurl (admin happy-path reads) — require the admin token. Skip
    # them when no admin could be seeded, so the read-only RBAC regression
    # fence still runs green in admin-less environments.
    if [[ "$scenario" == *_write.hurl || "$scenario" == *_admin.hurl ]] && [ -z "$ADMIN_ACCESS" ]; then
        echo
        echo "=== $scenario === SKIPPED (no admin_token; seed testadmin to enable)"
        continue
    fi
    TOTAL=$((TOTAL+1))
    echo
    echo "=== $scenario ==="
    if hurl "${HURL_OPTS[@]}" "$scenario"; then
        PASSED=$((PASSED+1))
    else
        FAILED=$((FAILED+1))
        FAILED_FILES+=("$scenario")
        echo "  >>> FAILED: $scenario" >&2
    fi
    # Brief pause to keep total request rate under the per-IP rate limit
    # (default: 60 req/min, burst 60). The suite makes ~250 requests across
    # 56 scenarios (~4-5 per file, fired in a burst); sleeping 5s between
    # scenarios keeps the average under 60/min so no scenario hits a 429.
    sleep 5
done

echo
echo "========================================"
echo "  Hurl smoke suite: $PASSED/$TOTAL passed, $FAILED failed"
if [ "${#FAILED_FILES[@]}" -gt 0 ]; then
    echo "  Failed scenarios:"
    for f in "${FAILED_FILES[@]}"; do
        echo "    - $f"
    done
fi
echo "========================================"

if [ "$FAILED" -gt 0 ]; then
    exit 1
fi
exit 0
