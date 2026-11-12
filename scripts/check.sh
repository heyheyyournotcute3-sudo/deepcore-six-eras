#!/usr/bin/env bash
# Format, lint and test the whole workspace. Used by every milestone.
set -euo pipefail
cd "$(dirname "$0")/.."
echo "== fmt =="
cargo fmt --all
echo "== clippy =="
cargo clippy --workspace --all-targets -- -D warnings
echo "== test =="
cargo test --workspace
echo "== ok =="
