#!/bin/bash
# Rust validation for x.uma
# Sources project-paths.sh for canonical path definitions

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/project-paths.sh"

cd "$XUMA_ROOT"

echo "Validating Rust workspace..."

# Format check
echo "Checking formatting..."
cargo fmt --manifest-path "$RUST_MANIFEST" --all -- --check

# Clippy with pedantic
echo "Running clippy..."
cargo clippy --manifest-path "$RUST_MANIFEST" --workspace -- -W clippy::pedantic -D warnings

# Tests
echo "Running tests..."
cargo test --manifest-path "$RUST_MANIFEST" --workspace

# no_std check
echo "Verifying no_std compatibility..."
cargo build --manifest-path "$RUST_MANIFEST" -p "$CRATE_CORE_PACKAGE" --no-default-features --features alloc

echo "Rust validation complete!"
