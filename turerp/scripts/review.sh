#!/usr/bin/env bash
#
# review.sh — pre-merge verification matrix for the Turerp ERP project.
#
# Runs the checks required by AGENTS.md "Pre-merge verification matrix":
#   - cargo fmt --check
#   - cargo clippy -- -D warnings
#   - cargo test --lib
#   - cargo test --test '*' (full integration suite)  [unless --quick]
#
# Usage (from the turerp/ crate directory, or the repo root):
#   bash scripts/review.sh            # full matrix (fmt + clippy + lib + integration)
#   bash/scripts/review.sh --quick    # fmt + clippy + lib tests only
#   bash scripts/review.sh --no-clippy # skip clippy (rare; e.g. CI already ran it)
#
# Exit codes:
#   0  all checks passed
#   1  at least one check failed
#
# This script is the helper invoked by the
# `.claude/agents/core/reviewer.md` adversarial review agent. It deliberately
# uses the smallest targeted commands first and only escalates to the full
# integration suite when not run with --quick.

set -u

# Resolve the turerp crate directory whether invoked from repo root or crate.
# The script lives at <crate>/scripts/review.sh, so the crate dir is the
# parent of the scripts dir. When invoked from the repo root as
# `bash turerp/scripts/review.sh`, SCRIPT_DIR is <root>/turerp/scripts and
# the crate is <root>/turerp (one level up).
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
if [ ! -f "$CRATE_DIR/Cargo.toml" ]; then
  echo "FATAL: turerp crate directory not found (expected Cargo.toml in $CRATE_DIR)" >&2
  exit 1
fi
cd "$CRATE_DIR" || { echo "FATAL: cannot cd to $CRATE_DIR" >&2; exit 1; }

QUICK=0
RUN_CLIPPY=1
for arg in "$@"; do
  case "$arg" in
    --quick)     QUICK=1 ;;
    --no-clippy) RUN_CLIPPY=0 ;;
    *) echo "Unknown flag: $arg" >&2; echo "Usage: bash scripts/review.sh [--quick] [--no-clippy]" >&2; exit 1 ;;
  esac
done

FAIL=0
section() { printf '\n\033[1;34m== %s ==\033[0m\n' "$1"; }
ok()      { printf '  \033[1;32mPASS\033[0m  %s\n' "$1"; }
fail()    { printf '  \033[1;31mFAIL\033[0m  %s\n' "$1"; FAIL=1; }

section "cargo fmt --check"
if cargo fmt --check >/tmp/turerp_fmt.log 2>&1; then
  ok "formatting clean"
else
  fail "formatting check failed (run 'cargo fmt')"
  sed 's/^/    /' /tmp/turerp_fmt.log | head -20
fi

if [ "$RUN_CLIPPY" -eq 1 ]; then
  section "cargo clippy -- -D warnings (bin + lib)"
  if cargo clippy --bin turerp --lib -- -D warnings >/tmp/turerp_clippy.log 2>&1; then
    ok "clippy clean"
  else
    fail "clippy reported warnings/errors"
    sed 's/^/    /' /tmp/turerp_clippy.log | tail -30
  fi
fi

section "cargo test --lib"
if cargo test --lib >/tmp/turerp_libtest.log 2>&1; then
  ok "lib tests passed"
  grep -E '^test result:' /tmp/turerp_libtest.log | tail -1 | sed 's/^/    /'
else
  fail "lib tests failed"
  sed 's/^/    /' /tmp/turerp_libtest.log | tail -30
fi

if [ "$QUICK" -eq 0 ]; then
  section "cargo test --test '*' (integration suite)"
  if cargo test --test '*' >/tmp/turerp_inttest.log 2>&1; then
    ok "integration tests passed"
    grep -E '^test result:' /tmp/turerp_inttest.log | tail -1 | sed 's/^/    /'
  else
    fail "integration tests failed"
    sed 's/^/    /' /tmp/turerp_inttest.log | tail -40
  fi
else
  ok "skipped integration suite (--quick)"
fi

section "Verdict"
if [ "$FAIL" -eq 0 ]; then
  printf '  \033[1;32mSAFE\033[0m — all verification-matrix checks passed.\n'
  exit 0
else
  printf '  \033[1;31mNEEDS REVIEW\033[0m — one or more checks failed. Fix before merge.\n'
  exit 1
fi