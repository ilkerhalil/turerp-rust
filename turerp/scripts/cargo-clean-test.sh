#!/usr/bin/env bash
# Clean and test — trades speed for disk space.
# WARNING: This removes incremental compilation artifacts.

set -euo pipefail

echo "[cargo-clean-test] Cleaning target directory..."
cargo clean

echo "[cargo-clean-test] Running tests..."
cargo test "$@"
