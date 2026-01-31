#!/bin/bash
# Rust validation for x.uma

set -e

XUMA_ROOT="${1:-$(pwd)}"
cd "$XUMA_ROOT"

echo "Validating Rust workspace..."

# Format check
echo "Checking formatting..."
cargo fmt --manifest-path rumi/Cargo.toml --all -- --check

# Clippy with pedantic
echo "Running clippy..."
cargo clippy --manifest-path rumi/Cargo.toml --workspace -- -W clippy::pedantic -D warnings

# Tests
echo "Running tests..."
cargo test --manifest-path rumi/Cargo.toml --workspace

# no_std check
echo "Verifying no_std compatibility..."
cargo build --manifest-path rumi/Cargo.toml -p rumi-core --no-default-features --features alloc

echo "Rust validation complete!"
