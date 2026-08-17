# x.uma Justfile
# Task orchestration for the matcher ecosystem

# Default recipe
default:
    @just --list

# ═══════════════════════════════════════════════════════════════════════════════
# Proto Generation
# ═══════════════════════════════════════════════════════════════════════════════

# Generate proto code (all three languages) and the xDS dependency types.
#
# Two passes are required. `buf generate` only walks the local module graph, and
# no xuma proto imports xDS, so the xds.* types convert.rs depends on are absent
# from it. The second pass fetches them explicitly.
#
# Order matters: buf.gen.yaml carries `clean: true` and wipes the tree, so it
# must run first. buf.gen.rust.yaml deliberately has no `clean` — it appends.
gen:
    #!/usr/bin/env bash
    set -euo pipefail
    # Generate BOTH passes into a staging tree, then swap. buf.gen.yaml carries
    # `clean: true`, so an in-place generate that fails part-way leaves the
    # committed tree destroyed — and the second pass reaches the network, which
    # rate-limits. Observed on 2026-08-17.
    #
    # Nothing here deletes: the outgoing trees are MOVED into the staging
    # directory and left for the OS to reap, so a bad swap is recoverable.
    STAGE=$(mktemp -d)
    echo "gen: staging in $STAGE"
    buf generate -o "$STAGE"
    # Scoped with --path. `buf generate buf.build/cncf/xds` without it pulls the
    # WHOLE module — ORCA load-reporting services, annotation metadata, the legacy
    # udpa namespace — 14 extra files and ~4,500 lines that lib.rs never includes
    # and nothing compiles. Only these three packages are used.
    buf generate buf.build/cncf/xds --template buf.gen.rust.yaml -o "$STAGE" \
        --path xds/core/v3 --path xds/type/v3 --path xds/type/matcher/v3
    for d in rumi/proto/src/gen puma/proto/src/gen bumi/proto/src/gen; do
        test -d "$STAGE/$d" || { echo "gen: $d missing from staging, refusing to swap"; exit 1; }
    done
    mkdir -p "$STAGE/.outgoing"
    for d in rumi/proto/src/gen puma/proto/src/gen bumi/proto/src/gen; do
        if [ -d "$d" ]; then mv "$d" "$STAGE/.outgoing/$(echo "$d" | tr / _)"; fi
        mkdir -p "$(dirname "$d")"
        mv "$STAGE/$d" "$d"
    done
    echo "gen: ok — previous trees kept in $STAGE/.outgoing"

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
    cargo build --manifest-path rumi/Cargo.toml

# Build with all features
build-full:
    cargo build --manifest-path rumi/Cargo.toml --all-features

# Run tests
test:
    cargo test --manifest-path rumi/Cargo.toml
    # rumi-proto is outside default-members, so the line above never sees it.
    # It went its whole life uncompiled because nothing ran this (PLAN.md F1).
    cargo test --manifest-path rumi/Cargo.toml -p rumi-proto

# Run tests with all features
test-full:
    cargo test --manifest-path rumi/Cargo.toml --all-features

# Run clippy lints
lint:
    cargo clippy --manifest-path rumi/Cargo.toml -- -W clippy::pedantic

# Format code
fmt:
    cargo fmt --manifest-path rumi/Cargo.toml --all

# Check formatting
fmt-check:
    cargo fmt --manifest-path rumi/Cargo.toml --all -- --check

# Run all checks (lint + fmt-check + test)
check: lint fmt-check test

# Everything CI runs, in the same order. Green here means green there.
ci: fmt-check lint-strict test test-fixtures puma-check bumi-check docs-check docs-build audit

# Clippy as CI enforces it: all targets, warnings denied
lint-strict:
    cargo clippy --manifest-path rumi/Cargo.toml --all-targets -- -D warnings

# Build and open Rust documentation
doc:
    cargo doc --manifest-path rumi/Cargo.toml --workspace --exclude rumi-proto --no-deps --open

# ═══════════════════════════════════════════════════════════════════════════════
# Documentation
# ═══════════════════════════════════════════════════════════════════════════════

# Run the docs site with hot reload (localhost:6200)
docs-dev: bumi-build
    cd docs/experience && bun run dev

# Build the docs site (includes Pagefind search index)
docs-build: bumi-build
    cd docs/experience && bun run build

# Preview the production docs build
docs-preview:
    cd docs/experience && bun run preview

# Type-check the docs site
docs-check:
    cd docs/experience && bun run check

# Generate Rust API docs (assembled into the site at /api/rust by CI)
docs-rust:
    cargo doc --manifest-path rumi/Cargo.toml --no-deps

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

# ═══════════════════════════════════════════════════════════════════════════════
# Python (puma)
# ═══════════════════════════════════════════════════════════════════════════════

# Run puma tests
puma-test:
    cd puma && uv run pytest

# Lint puma
puma-lint:
    cd puma && uv run ruff check .

# Type-check puma
puma-typecheck:
    cd puma && uv run mypy src/xuma

# Run all puma checks
puma-check: puma-lint puma-typecheck puma-test

# ═══════════════════════════════════════════════════════════════════════════════
# TypeScript (bumi)
# ═══════════════════════════════════════════════════════════════════════════════

# Install bumi dependencies
bumi-install:
    cd bumi && bun install

# Build bumi to dist/ (published artifact — .js + .d.ts)
bumi-build:
    cd bumi && bun run build

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

# Security audit. The workspace is rumi/, and Cargo.lock is gitignored, so the
# lock is generated first. Mirrors the CI audit job exactly.
audit:
    cargo generate-lockfile --manifest-path rumi/Cargo.toml
    cargo audit --file rumi/Cargo.lock
