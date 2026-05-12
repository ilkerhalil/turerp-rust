#!/usr/bin/env bash
# Clean and build — trades speed for disk space.
# WARNING: This removes incremental compilation artifacts,
# so every build will be from scratch (much slower).

set -euo pipefail

echo "[cargo-clean-build] Cleaning target directory..."
cargo clean

echo "[cargo-clean-build] Building..."
cargo build "$@"
