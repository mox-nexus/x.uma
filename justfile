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

# Build and open Rust documentation
doc:
    cargo doc --manifest-path rumi/Cargo.toml --workspace --exclude rumi-proto --no-deps --open

# ═══════════════════════════════════════════════════════════════════════════════
# Documentation
# ═══════════════════════════════════════════════════════════════════════════════

# Generate Rust API docs
docs-rust:
    cargo doc --manifest-path rumi/Cargo.toml --workspace --exclude rumi-proto --no-deps

# Generate Python API docs
docs-python:
    cd puma && uv run pdoc xuma -o ../docs/api/python --html

# Generate TypeScript API docs
docs-typescript:
    cd bumi && bunx typedoc src/index.ts --out ../docs/api/typescript

# Generate all API docs
docs-api: docs-rust docs-python docs-typescript
    mkdir -p docs/api/rust
    cp -r rumi/target/doc/* docs/api/rust/

# Build mdBook site
docs-build:
    mdbook build docs

# Build everything (mdBook + API docs)
docs: docs-api docs-build

# Serve docs locally with hot reload
docs-serve:
    mdbook serve docs --open

# Clean generated docs
docs-clean:
    rm -rf docs/book docs/api

# ═══════════════════════════════════════════════════════════════════════════════
# Benchmarks
# ═══════════════════════════════════════════════════════════════════════════════

# Run Rust benchmarks (divan)
bench-rust:
    cargo bench --manifest-path rumi/Cargo.toml
    cargo bench --manifest-path rumi/Cargo.toml --bench config --features registry

# Run Python benchmarks (pytest-benchmark)
bench-puma:
    cd puma && uv run pytest tests/bench/ --benchmark-only --benchmark-disable-gc

# Run TypeScript benchmarks (mitata)
bench-bumi:
    cd bumi && bun run bench
    cd bumi && bun run bench/config.bench.ts

# Run xuma-crust (Python) vs puma comparison benchmarks
bench-xuma-crust-py:
    cd rumi/crusts/python && maturin develop && uv run pytest tests/test_bench_crusty.py tests/test_bench_config.py --benchmark-only --benchmark-disable-gc

# Run xuma-crust (WASM) vs bumi comparison benchmarks
bench-xuma-crust-wasm:
    cd rumi/crusts/wasm && wasm-pack build --target web && bun run bench/crusty.bench.ts && bun run bench/config.bench.ts

# Run all benchmarks
bench-all: bench-rust bench-puma bench-bumi bench-xuma-crust-py bench-xuma-crust-wasm

# Alias for bench-all
bench: bench-all

# Verify no_std compatibility (core only)
check-no-std:
    cargo build --manifest-path rumi/Cargo.toml -p rumi --no-default-features --features alloc

# ═══════════════════════════════════════════════════════════════════════════════
# Python (puma)
# ═══════════════════════════════════════════════════════════════════════════════

# Run puma tests
puma-test:
    cd puma && uv run pytest

# ═══════════════════════════════════════════════════════════════════════════════
# TypeScript (bumi)
# ═══════════════════════════════════════════════════════════════════════════════

# Install bumi dependencies
bumi-install:
    cd bumi && bun install

# Run bumi tests
bumi-test:
    cd bumi && bun test

# Type-check bumi
bumi-typecheck:
    cd bumi && bun run typecheck

# Lint bumi
bumi-lint:
    cd bumi && bun run lint

# Format bumi
bumi-fmt:
    cd bumi && bun run fmt

# Check bumi formatting
bumi-fmt-check:
    cd bumi && bun run fmt:check

# Run all bumi checks
bumi-check: bumi-lint bumi-fmt-check bumi-typecheck bumi-test

# ═══════════════════════════════════════════════════════════════════════════════
# Playground
# ═══════════════════════════════════════════════════════════════════════════════

# Install playground dependencies
playground-install:
    cd playground && bun install

# Run playground dev server
playground-dev:
    cd playground && bun run dev

# Build playground for production
playground-build:
    cd playground && bun run build

# Preview production playground build
playground-preview:
    cd playground && bun run preview

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
    cargo publish --manifest-path rumi/ext/http/Cargo.toml --dry-run

# Security audit
audit:
    cargo audit
