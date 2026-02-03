# x.uma Justfile
# Task orchestration for the matcher ecosystem

# Default recipe
default:
    @just --list

# ═══════════════════════════════════════════════════════════════════════════════
# Proto Generation
# ═══════════════════════════════════════════════════════════════════════════════

# Generate proto code
gen:
    buf generate

# Lint proto files
lint-proto:
    buf lint

# Check proto breaking changes
breaking:
    buf breaking --against '.git#branch=main'

# ═══════════════════════════════════════════════════════════════════════════════
# Rust (rumi)
# ═══════════════════════════════════════════════════════════════════════════════

# Build all crates
build:
    cargo build --manifest-path rumi/Cargo.toml --workspace

# Build with all features
build-full:
    cargo build --manifest-path rumi/Cargo.toml --workspace --all-features

# Run tests
test:
    cargo test --manifest-path rumi/Cargo.toml --workspace

# Run tests with all features
test-full:
    cargo test --manifest-path rumi/Cargo.toml --workspace --all-features

# Run clippy lints
lint:
    cargo clippy --manifest-path rumi/Cargo.toml --workspace -- -W clippy::pedantic

# Format code
fmt:
    cargo fmt --manifest-path rumi/Cargo.toml --all

# Check formatting
fmt-check:
    cargo fmt --manifest-path rumi/Cargo.toml --all -- --check

# Run all checks (lint + fmt-check + test)
check: lint fmt-check test

# Build Rust documentation
doc:
    cargo doc --manifest-path rumi/Cargo.toml --workspace --no-deps --open

# ═══════════════════════════════════════════════════════════════════════════════
# Documentation Site
# ═══════════════════════════════════════════════════════════════════════════════

# Build full docs site (mdbook + rustdoc + proto)
docs-build:
    mkdir -p docs/src/generated/rust docs/src/generated/proto
    cargo doc --manifest-path rumi/Cargo.toml --workspace --no-deps
    cp -r rumi/target/doc/* docs/src/generated/rust/
    mdbook build docs

# Serve docs locally with hot reload
docs-serve:
    mdbook serve docs --open

# Clean generated docs
docs-clean:
    rm -rf docs/book docs/src/generated

# Run benchmarks
bench:
    cargo bench --manifest-path rumi/Cargo.toml

# Verify no_std compatibility (core only)
check-no-std:
    cargo build --manifest-path rumi/Cargo.toml -p rumi --no-default-features --features alloc

# ═══════════════════════════════════════════════════════════════════════════════
# Conformance Testing
# ═══════════════════════════════════════════════════════════════════════════════

# Run conformance fixtures
test-fixtures:
    cargo test --manifest-path rumi/Cargo.toml -p rumi-test --test conformance --features rumi-test/fixtures

# ═══════════════════════════════════════════════════════════════════════════════
# Development
# ═══════════════════════════════════════════════════════════════════════════════

# Watch and rebuild on changes
watch:
    cargo watch --manifest-path rumi/Cargo.toml -x build

# Clean build artifacts
clean:
    cargo clean --manifest-path rumi/Cargo.toml

# ═══════════════════════════════════════════════════════════════════════════════
# Release
# ═══════════════════════════════════════════════════════════════════════════════

# Dry-run publish
publish-dry:
    cargo publish --manifest-path rumi/core/Cargo.toml --dry-run
    cargo publish --manifest-path rumi/ext/test/Cargo.toml --dry-run
    cargo publish --manifest-path rumi/ext/http/Cargo.toml --dry-run
    cargo publish --manifest-path rumi/ext/claude/Cargo.toml --dry-run

# Security audit
audit:
    cargo audit
