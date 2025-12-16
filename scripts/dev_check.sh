#!/usr/bin/env bash
set -euo pipefail

echo "[pc] Running cargo fmt..."
cargo fmt --all

echo "[pc] Running cargo clippy..."
cargo clippy --all-targets --all-features -- -D warnings

echo "[pc] Running cargo test..."
cargo test --all-features

