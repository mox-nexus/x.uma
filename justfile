# x.uma Justfile
# Task orchestration for the matcher ecosystem

# Default recipe
default:
    @just --list

# ═══════════════════════════════════════════════════════════════════════════════
# Proto Generation
# ═══════════════════════════════════════════════════════════════════════════════

# Build the PyO3 crust and run its tests.
#
# The crusts are outside default-members (extension-module cannot link
# libpython under plain `cargo test`), so nothing in `just test` touches them.
# 80 tests here had never run in CI — PLAN.md F5 / CI1.
crust-py-check:
    # --features fixtures for the TEST build only. The published wheel is built
    # without it, so no YAML fixture loader ships to PyPI (PLAN.md E3).
    cd rumi/crusts/python && maturin develop --uv --features fixtures
    cd rumi/crusts/python && uv run pytest tests/ -q \
        --ignore=tests/test_bench_config.py --ignore=tests/test_bench_crusty.py

# Build the wasm-bindgen crust and run its tests. 80 more, same story.
crust-wasm-check:
    # --features fixtures for the TEST build only; the npm package ships without.
    cd rumi/crusts/wasm && wasm-pack build --target web -- --features fixtures
    cd rumi/crusts/wasm && bun install --frozen-lockfile && bun test

# Both crusts. Not in `just ci`: wasm-pack builds take ~1 minute and the two
# have separate toolchains, so CI runs them as their own jobs.
crust-check: crust-py-check crust-wasm-check

# Do the crusts still compile? ~3s each, and no maturin, wasm-pack or wasm32
# target needed — `cargo check` neither links nor builds a cdylib.
#
# This IS in `just ci`, unlike the recipes above. Three times in one day a
# change that `just ci` called green broke both crusts, and each was found only
# after pushing: deleting a fixture dialect, adding an HTTP domain to the
# fixture schema, and turning `EvalTrace.steps` into an enum. Every one would
# have failed here in seconds. The full suites still run as their own CI jobs;
# what was missing was anything at all locally.
crust-compiles:
    cargo check --manifest-path rumi/crusts/python/Cargo.toml --features fixtures
    cargo check --manifest-path rumi/crusts/wasm/Cargo.toml --features fixtures

# Check every tool this repo needs is installed. First command in CONTRIBUTING.
doctor:
    ./scripts/doctor.sh

# Build from `git archive HEAD` — what a clone actually gets — rather than the
# working tree. `just ci` cannot see a file that exists locally but is untracked
# or ignored, which is a class that has bitten this repo three times.
verify-clean-clone:
    ./scripts/verify-clean-clone.sh

# Build and test each rumi-http feature in isolation.
#
# --all-features samples one corner of the feature lattice, and additivity is a
# property of the whole lattice — which is why a broken `gateway`-only build was
# invisible to it.
features:
    ./scripts/check-features.sh

# No publishable crate may depend on a publish = false crate.
publishable:
    node scripts/check-publishable.mjs

# Every xuma proto field must round-trip through binary as the identity.
#
# The config path encodes JSON to protobuf to fill an Any and decodes it back.
# That is lossless only for a specific set of field types; the script says which
# and why.
proto-field-types:
    node scripts/check-proto-field-types.mjs

# Generate proto code (all three languages) and the xDS dependency types.
#
# Two passes are required. `buf generate` only walks the local module graph, and
# no xuma proto imports xDS, so the xds.* types convert.rs depends on are absent
# from it. The second pass fetches them explicitly.
#
# Order matters: buf.gen.yaml carries `clean: true` and wipes the tree, so it
# must run first. buf.gen.rust.yaml deliberately has no `clean` — it appends.
gen:
    ./scripts/gen.sh

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

# Run tests with all features.
#
# This was red for months and nothing noticed, because `just ci` ran `test`
# (default features, default-members) instead. What it was reporting was real:
# two config vocabularies fighting over one associated type. It is in `ci` now
# so it cannot go quiet again.
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
ci: fmt-check lint-strict test test-full test-protojson test-fixture-coverage crust-compiles bench-smoke features publishable proto-field-types docs-commands docs-links docs-samples readme-agreement puma-check bumi-check docs-check docs-build audit

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
# Assert every `rumi ...` command in docs/content and README matches `rumi --help`.
# Four how-to pages taught subcommands that never existed; prose cannot fail a build.
docs-commands:
    node scripts/check-doc-commands.mjs

# Assert every internal Markdown link resolves to a page the site serves.
# Every one of them was dead until 2026-08-17; the build reported one, as a 404.
# Run the README's literal routes.yaml through every runtime and assert they
# agree. "One config, all runtimes" is the project's central claim; nothing
# checked it. PLAN.md CI3.
readme-agreement:
    node scripts/check-readme-agreement.mjs

docs-links:
    node scripts/check-doc-links.mjs

# Execute every Python/TypeScript code block on the getting-started pages and in
# the package READMEs, in the runtime that owns it.
#
# `rumi-docs-tests` has compiled the *Rust* blocks since PR #26; nothing did the
# same for the other two languages, and the drift tracked that exactly.
# `--require-all` turns an environment skip into a failure, so CI cannot pass by
# quietly not checking the wasm crust.
docs-samples:
    node scripts/check-doc-samples.mjs --require-all

docs-check:
    cd docs/experience && bun run check

# Generate Rust API docs (assembled into the site at /api/rust by CI)
docs-rust:
    cargo doc --manifest-path rumi/Cargo.toml --no-deps

# ═══════════════════════════════════════════════════════════════════════════════
# Benchmarks
# ═══════════════════════════════════════════════════════════════════════════════

# Run every benchmark exactly once, to prove it still runs. ~37s.
#
# This is NOT regression detection. There is no baseline and no threshold, so a
# benchmark that gets 10x slower passes here. What it catches is a benchmark
# that has quietly stopped measuring anything: one that panics, or calls an API
# that is gone, or carries a config that no longer parses.
#
# That last one is why it exists. Until 2026-08-18 `benches/config.rs` measured
# the parse cost of the terse config dialect — a format retired months earlier
# by D-026. It was found only because deleting the dialect broke compilation.
# Had the format merely *changed*, the benchmark would have gone on producing
# numbers for something nobody ran. Falsified: a typo'd field in that config
# panics this recipe.
bench-smoke:
    cargo bench --manifest-path rumi/Cargo.toml --all-features -- --test

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

# Conformance over the protojson fixtures — the format that replaces the four
# transitional dialects. Each fixture names the implementations expected to run
# it, and a runner not on that list must FAIL to run it, so an exception that
# quietly starts working is caught as well as one that quietly starts failing.
test-protojson:
    cargo test --manifest-path rumi/Cargo.toml -p rumi-test --test proto_conformance --features rumi-test/registry,rumi-test/fixtures

# Does the fixture corpus span the schema?
#
# The price of not generating types for puma and bumi: their dependency on
# proto/xuma is a human's memory rather than an arrow the build can see, so the
# fixture corpus has to carry it. Messages with no fixture are listed with a
# reason, and that list is checked for staleness in both directions.
test-fixture-coverage:
    cargo test --manifest-path rumi/Cargo.toml -p rumi-test --test fixture_coverage --features rumi-test/registry,rumi-test/fixtures

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

# Security audit against the committed lockfile. Mirrors the CI audit job.
#
# No generate-lockfile step: Cargo.lock is committed (D-031), so auditing a
# freshly resolved graph would audit something other than what ships.
audit:
    cargo audit --file rumi/Cargo.lock
