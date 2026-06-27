#!/usr/bin/env bash
# check_hurl_coverage.sh — Hurl endpoint coverage report (companion to the
# hurl smoke suite in tests/hurl/).
#
# Computes endpoint coverage of turerp/tests/hurl/*.hurl vs the registered
# endpoints in turerp/openapi.json. Report-only: always exits 0 so it can be
# wired into CI as a non-gating informational step without breaking merges.
#
# Matching model: a REGISTERED endpoint is a pattern where each `{param}`
# segment is a wildcard that matches ANY single hurl segment (a numeric id,
# a UUID, a hurl {{variable}}, or a static literal like "WF999"/"TST"/
# "Asset"/"customer"). A hurl endpoint is COVERED if some registered pattern
# of the same method matches it segment-for-segment (literals must be equal,
# params match anything). This avoids false orphans from hurl variables and
# per-run static fixture literals — the registered route drives the match.
#
# Output:
#   - Overall: covered/registered count + percentage
#   - Per top-level module: covered/registered
#   - UNCOVERED registered endpoints (registered minus covered), listed
#   - HURL-ONLY (orphans): hurl hits that match no registered route
#
# CI wiring (non-gating):
#   - name: hurl coverage report
#     run: bash scripts/check_hurl_coverage.sh
#     continue-on-error: true   # explicit: never gate
#
# Or locally: bash scripts/check_hurl_coverage.sh
#
# Requirements: bash, awk, jq. Run from anywhere; paths resolve relative to
# this script.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OPENAPI="$REPO_ROOT/openapi.json"
HURL_DIR="$REPO_ROOT/tests/hurl"

if [ ! -f "$OPENAPI" ]; then
    echo "ERROR: openapi.json not found at $OPENAPI" >&2
    exit 0
fi
if [ ! -d "$HURL_DIR" ]; then
    echo "ERROR: hurl dir not found at $HURL_DIR" >&2
    exit 0
fi
if ! command -v jq >/dev/null 2>&1; then
    echo "ERROR: jq is required" >&2
    exit 0
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# --- 1. Build REGISTERED list from openapi.json ---
# Emit "METHOD /path" for every (path, verb). Preserve {param} braces verbatim
# (openapi already uses {param} form) — the matcher treats {param} as wildcard.
jq -r '
  .paths
  | to_entries[]
  | .key as $p
  | .value
  | to_entries[]
  | select(.key | test("^(get|post|put|delete|patch)$"))
  | "\(.key | ascii_upcase) \($p)"
' "$OPENAPI" | sort -u > "$WORK/registered.txt"

# --- 2. Build COVERED list from hurl files ---
# Extract request lines: ^METHOD {{base_url}}/api/... Strip query string and
# the {{base_url}} prefix. Keep {{variables}} and static literals verbatim —
# the matcher (step 3) resolves them against registered {param} wildcards.
{
    for f in "$HURL_DIR"/*.hurl; do
        grep -hE '^(GET|POST|PUT|DELETE|PATCH) +\{\{base_url\}\}/api/' "$f" || true
    done
} \
  | sed -E 's/\{\{base_url\}\}//' \
  | awk '{ print $1, $2 }' \
  | sed -E 's/\?.*$//' \
  | sort -u > "$WORK/hurl.txt"

# --- 3. Match hurl endpoints against registered patterns ---
# One awk pass over both files. Registered patterns are loaded first; each
# registered {param} segment is a wildcard matching ANY hurl segment (numeric
# id, UUID, {{variable}}, or static literal); literals must match exactly.
# Writes: covered.txt (registered hit by ≥1 hurl), uncovered.txt (registered
# never hit), orphan.txt (hurl hitting no registered route). Each line keeps
# the original registered/hurl path verbatim.
awk '
# file 1: registered (load original line + verb + path)
NR == FNR {
    R++
    Rline[R] = $0
    split($0, f, " ")
    Rv[R] = f[1]
    Rp[R] = f[2]
    next
}
# file 2: hurl — match against every registered pattern (re-split each)
{
    split($0, hf, " ")
    hv = hf[1]; hn = split(hf[2], hs, "/")
    matched = 0
    for (r = 1; r <= R; r++) {
        if (Rv[r] != hv) continue
        rn = split(Rp[r], rs, "/")
        if (rn != hn) continue
        ok = 1
        for (i = 1; i <= hn; i++) {
            if (rs[i] ~ /^\{.*\}$/) continue      # wildcard matches any segment
            if (rs[i] == hs[i]) continue
            ok = 0; break
        }
        if (ok) { Rcovered[r] = 1; matched = 1 }
    }
    if (!matched) print $0 > "'"$WORK/orphan.txt"'"
    next
}
END {
    for (r = 1; r <= R; r++)
        print Rline[r] > (Rcovered[r] ? "'"$WORK/covered.txt"'" : "'"$WORK/uncovered.txt"'")
}
' "$WORK/registered.txt" "$WORK/hurl.txt"

sort -u "$WORK/covered.txt"   > "$WORK/covered.s"   2>/dev/null || : > "$WORK/covered.s"
sort -u "$WORK/uncovered.txt" > "$WORK/uncovered.s" 2>/dev/null || : > "$WORK/uncovered.s"
sort -u "$WORK/orphan.txt"    > "$WORK/orphan.s"     2>/dev/null || : > "$WORK/orphan.s"

REG_TOTAL=$(wc -l < "$WORK/registered.txt" | tr -d ' ')
MATCHED=$(wc -l < "$WORK/covered.s" | tr -d ' ')
UNCOVERED=$(wc -l < "$WORK/uncovered.s" | tr -d ' ')
ORPHAN=$(wc -l < "$WORK/orphan.s" | tr -d ' ')

PCT="0"
if [ "$REG_TOTAL" -gt 0 ]; then
    PCT=$(awk -v m="$MATCHED" -v r="$REG_TOTAL" 'BEGIN { printf "%.1f", (m/r)*100 }')
fi

# --- 4. Per-module tally (re-derive cleanly from covered/uncovered) ---
declare -A REG_MOD COV_MOD
while IFS= read -r line; do
    verb=${line%% *}; path=${line#* }
    mod=$(echo "$path" | sed -E 's#^/##; s#^api/v1/##; s#^api/##' | awk -F/ '{ if ($1=="") print "(root)"; else print $1 }')
    REG_MOD[$mod]=$(( ${REG_MOD[$mod]:-0} + 1 ))
done < "$WORK/uncovered.s"
while IFS= read -r line; do
    verb=${line%% *}; path=${line#* }
    mod=$(echo "$path" | sed -E 's#^/##; s#^api/v1/##; s#^api/##' | awk -F/ '{ if ($1=="") print "(root)"; else print $1 }')
    REG_MOD[$mod]=$(( ${REG_MOD[$mod]:-0} + 1 ))
    COV_MOD[$mod]=$(( ${COV_MOD[$mod]:-0} + 1 ))
done < "$WORK/covered.s"

# --- 5. Print report ---
echo "=========================================================="
echo "  Hurl endpoint coverage report"
echo "  openapi: $OPENAPI"
echo "  hurl:    $HURL_DIR/*.hurl"
echo "=========================================================="
echo
echo "OVERALL"
echo "  registered endpoints : $REG_TOTAL"
echo "  covered (matched)    : $MATCHED"
echo "  uncovered            : $UNCOVERED"
echo "  hurl-only (orphans)  : $ORPHAN"
echo "  coverage             : ${PCT}%"
echo
echo "PER MODULE (registered -> covered)"
{
    for mod in "${!REG_MOD[@]}"; do
        r=${REG_MOD[$mod]}
        c=${COV_MOD[$mod]:-0}
        mpct="0.0"
        [ "$r" -gt 0 ] && mpct=$(awk -v c="$c" -v r="$r" 'BEGIN { printf "%.0f", (c/r)*100 }')
        printf "%-28s %4d -> %4d  (%s%%)\n" "$mod" "$r" "$c" "$mpct"
    done
} | sort
echo
echo "UNCOVERED REGISTERED ENDPOINTS ($UNCOVERED)"
if [ "$UNCOVERED" -gt 0 ]; then
    column -t < "$WORK/uncovered.s" 2>/dev/null || cat "$WORK/uncovered.s"
else
    echo "  (none — full coverage)"
fi
echo
echo "HURL-ONLY (orphan) ENDPOINTS ($ORPHAN) — informational, not in openapi"
if [ "$ORPHAN" -gt 0 ]; then
    head -50 "$WORK/orphan.s" | column -t 2>/dev/null || head -50 "$WORK/orphan.s"
    [ "$ORPHAN" -gt 50 ] && echo "  ... ($((ORPHAN - 50)) more)"
else
    echo "  (none)"
fi
echo
echo "=========================================================="
echo "  Report-only — exit 0 (never gates CI)."
echo "=========================================================="

exit 0