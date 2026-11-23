# x.uma task runner. `just --list` shows one line per recipe; a comment above
# a `[doc]` line says why the recipe is the way it is.

# List the recipes
default:
    @just --list

# CI differs only in how it installs tools, and in building the docs under the
# deployed base path.
[doc('Run every check CI runs')]
ci: fmt-check lint-strict test test-full test-protojson test-http-conformance test-fixture-coverage lint-proto gen-check crust-compiles crust-check crust-pack bench-smoke features publishable proto-field-types harness docs-commands docs-links docs-check docs-test docs-build docs-samples readme-agreement audit

# Check every tool this repository needs is installed
doctor:
    ./scripts/doctor.sh

# ═══════════════════════════════════════════════════════════════════════════════
# Proto
# ═══════════════════════════════════════════════════════════════════════════════

# Two buf passes, in order: the first wipes the generated tree and writes the
# xuma types, the second appends the xDS types that no xuma proto imports.
[doc('Regenerate the proto code')]
gen:
    ./scripts/gen.sh

# Generated code is committed, which is honest only if regenerating reproduces it.
[doc('Regenerate the proto code and fail if the committed tree differs')]
gen-check: gen
    git diff --exit-code -- rumi/proto/src/gen

# Lint the proto files
lint-proto:
    buf lint

# Check the proto files for breaking changes against main
breaking:
    buf breaking --against '.git#branch=main'

# The config path encodes JSON to protobuf to fill an Any and decodes it back,
# which is lossless only for some field types. The script says which and why.
[doc('Check every xuma proto field round-trips through binary unchanged')]
proto-field-types:
    node scripts/check-proto-field-types.mjs

# ═══════════════════════════════════════════════════════════════════════════════
# Rust (rumi)
# ═══════════════════════════════════════════════════════════════════════════════

# Build the default workspace members
build:
    cargo build --manifest-path rumi/Cargo.toml

# Build with all features
build-full:
    cargo build --manifest-path rumi/Cargo.toml --all-features

# rumi-proto is outside default-members, so it needs its own line.
[doc('Run the Rust tests')]
test:
    cargo test --manifest-path rumi/Cargo.toml
    cargo test --manifest-path rumi/Cargo.toml -p rumi-proto

# Run the Rust tests with all features
test-full:
    cargo test --manifest-path rumi/Cargo.toml --all-features

# Run clippy with pedantic warnings
lint:
    cargo clippy --manifest-path rumi/Cargo.toml -- -W clippy::pedantic

# Run clippy as CI enforces it: all targets, warnings denied
lint-strict:
    cargo clippy --manifest-path rumi/Cargo.toml --all-targets -- -D warnings

# Format the Rust code
fmt:
    cargo fmt --manifest-path rumi/Cargo.toml --all

# Check the Rust formatting
fmt-check:
    cargo fmt --manifest-path rumi/Cargo.toml --all -- --check

# Run lint, fmt-check and test
check: lint fmt-check test

# --all-features samples one corner of the feature lattice; a feature that
# removes an item instead of adding one only shows up when built alone.
[doc('Build and test each rumi-http feature in isolation')]
features:
    ./scripts/check-features.sh

# Named crates, not `--workspace`: both crusts produce a lib called `xuma_crust`
# and rustdoc refuses to write two crates to one path.
[doc('Build and open the Rust API docs for the published crates')]
doc:
    cargo doc --manifest-path rumi/Cargo.toml --no-deps -p rumi-core -p rumi-proto -p rumi-http -p rumi-kv -p rumi-cli --open

# Rebuild on change
watch:
    cargo watch --manifest-path rumi/Cargo.toml -x build

# Remove the Rust build artifacts
clean:
    cargo clean --manifest-path rumi/Cargo.toml

# ═══════════════════════════════════════════════════════════════════════════════
# Bindings (crusts)
#
# The crusts are outside default-members: the PyO3 extension module cannot link
# libpython under plain `cargo test`, so nothing in `just test` touches them.
# The test builds add `fixtures` (the conformance loader) and `bench` (the
# benchmark entry points); the published packages are built with neither.
# ═══════════════════════════════════════════════════════════════════════════════

# `cargo check` neither links nor builds a cdylib, so this needs no maturin,
# wasm-pack or wasm32 target and takes seconds.
[doc('Check that both crusts still compile')]
crust-compiles:
    cargo check --manifest-path rumi/crusts/python/Cargo.toml --features fixtures,bench
    cargo check --manifest-path rumi/crusts/wasm/Cargo.toml --features fixtures,bench

# --benchmark-disable runs each benchmark once as a plain test, so one that has
# stopped running fails here.
[doc('Build the PyO3 crust and run its tests and benchmarks')]
crust-py-check:
    cd rumi/crusts/python && maturin develop --uv --features fixtures,bench
    cd rumi/crusts/python && uv run pytest tests/ -q --benchmark-disable

# The Rust unit tests run natively and cover the code both crusts share, since
# the PyO3 crust cannot link `cargo test`.
[doc('Build the wasm crust and run its tests and benchmarks')]
crust-wasm-check:
    cd rumi/crusts/wasm && wasm-pack build --target web -- --features fixtures,bench
    cd rumi/crusts/wasm && bun install --frozen-lockfile && bun test
    cd rumi/crusts/wasm && bun run bench/floor.bench.ts && bun run bench/crusty.bench.ts && bun run bench/config.bench.ts
    cargo test --manifest-path rumi/crusts/wasm/Cargo.toml --lib

# Build and test both crusts
crust-check: crust-py-check crust-wasm-check

# Build the wasm crust as it is published
crust-wasm-build:
    cd rumi/crusts/wasm && wasm-pack build --target web

# Needs a wasm-pack build to have run.
[doc('Check the npm tarball for xuma-crust contains exactly what it promises')]
crust-pack:
    node scripts/pack-wasm-crust.mjs

# ═══════════════════════════════════════════════════════════════════════════════
# Conformance
# ═══════════════════════════════════════════════════════════════════════════════

# Each fixture names the implementations expected to pass it, and a runner not
# on that list must fail it, so a stale exception is caught either way.
[doc('Run the protojson conformance fixtures in rumi')]
test-protojson:
    cargo test --manifest-path rumi/Cargo.toml -p rumi-test --test proto_conformance --features rumi-test/registry,rumi-test/fixtures

# Run the Gateway API fixtures through the HTTP domain compiler
test-http-conformance:
    cargo test --manifest-path rumi/Cargo.toml -p rumi-test --test http_conformance --features rumi-test/http,rumi-test/fixtures

# The fixtures are the contract the bindings are held to, so every config
# message must appear in one or be listed as a gap with a reason.
[doc('Check the fixtures cover every config message')]
test-fixture-coverage:
    cargo test --manifest-path rumi/Cargo.toml -p rumi-test --test fixture_coverage --features rumi-test/registry,rumi-test/fixtures

# ═══════════════════════════════════════════════════════════════════════════════
# Documentation
# ═══════════════════════════════════════════════════════════════════════════════

# The site runs on the engine: the wasm crust is a `file:` dependency of the
# docs app, so the package has to exist before the app installs.
[doc('Build the wasm crust and install the docs site dependencies')]
docs-install: crust-wasm-build
    cd docs/experience && bun install --frozen-lockfile

# Run the docs site with hot reload (localhost:6200)
docs-dev: docs-install
    cd docs/experience && bun run dev

# Build the docs site, including the search index
docs-build: docs-install
    cd docs/experience && bun run build

# Preview the production docs build
docs-preview:
    cd docs/experience && bun run preview

# Type-check the docs site
docs-check: docs-install
    cd docs/experience && bun run check

# Type-checking cannot tell whether the playground's diagram describes the
# config it was given; these tests can.
[doc('Run the docs site tests')]
docs-test:
    cd docs/experience && bun test tests/

# Check every `rumi ...` command in the docs and README matches `rumi --help`
docs-commands:
    node scripts/check-doc-commands.mjs

# Check every internal Markdown link resolves to a page the site serves
docs-links:
    node scripts/check-doc-links.mjs

# --require-all turns a missing runtime into a failure, so this cannot pass by
# not checking one. Needs both crusts built.
[doc('Run every Python and TypeScript code block in the docs and package READMEs')]
docs-samples:
    node scripts/check-doc-samples.mjs --require-all

# Needs both crusts built.
[doc('Run the README config through every runtime and check they agree')]
readme-agreement:
    node scripts/check-readme-agreement.mjs

# Same crate list as `doc`: a bare `cargo doc` documents the default members,
# which include unpublished crates and omit rumi-proto.
[doc('Generate the Rust API docs for the published crates')]
docs-rust:
    cargo doc --manifest-path rumi/Cargo.toml --no-deps -p rumi-core -p rumi-proto -p rumi-http -p rumi-kv -p rumi-cli

# ═══════════════════════════════════════════════════════════════════════════════
# Benchmarks
# ═══════════════════════════════════════════════════════════════════════════════

# Not regression detection: there is no baseline, so a slower benchmark passes.
# It catches one that panics, calls a vanished API or carries a config that no
# longer parses. The crust benchmarks get the same check in `crust-check`.
[doc('Run every Rust benchmark once, to prove it still runs')]
bench-smoke:
    cargo bench --manifest-path rumi/Cargo.toml --all-features -- --test

# Run the Rust benchmarks
bench-rust:
    cargo bench --manifest-path rumi/Cargo.toml
    cargo bench --manifest-path rumi/Cargo.toml --bench config --features registry

# --release matters: a debug extension inverts every comparison drawn from it.
[doc('Run the PyO3 crust benchmarks')]
bench-xuma-crust-py:
    cd rumi/crusts/python && maturin develop --release --features bench && uv run pytest tests/test_bench_crusty.py tests/test_bench_config.py tests/test_bench_floor.py --benchmark-only --benchmark-disable-gc

# Run the wasm crust benchmarks
bench-xuma-crust-wasm:
    cd rumi/crusts/wasm && wasm-pack build --target web -- --features bench && bun run bench/crusty.bench.ts && bun run bench/config.bench.ts && bun run bench/floor.bench.ts

# Run all benchmarks
bench-all: bench-rust bench-xuma-crust-py bench-xuma-crust-wasm

# Run all benchmarks
bench: bench-all

# Builds the bench-feature crusts first unless NO_BUILD=1.
[doc('Run one cold launch of both crust benchmark suites into bench/raw/')]
bench-collect:
    python3 scripts/bench_collect.py

# The protocol behind every published number (D-066): cold launches on a clean
# tree at one commit, median per launch, and a noise gate from their spread.
[doc('Measure LAUNCHES cold launches into bench/artifacts/')]
bench-artifact LAUNCHES="10":
    python3 scripts/bench_artifact.py --launches {{LAUNCHES}}

# Render a benchmark artifact as bench/RESULTS.md
bench-report ARTIFACT:
    python3 scripts/bench_report.py {{ARTIFACT}} --output bench/RESULTS.md

# Compare two benchmark artifacts against the larger noise gate
bench-compare OLD NEW:
    python3 scripts/bench_compare.py {{OLD}} {{NEW}}

# ═══════════════════════════════════════════════════════════════════════════════
# Repository and release
# ═══════════════════════════════════════════════════════════════════════════════

# Check the harness names only recipes, paths and engines that exist
harness:
    node scripts/check-harness.mjs --self-test
    node scripts/check-harness.mjs

# Check no publishable crate depends on an unpublished one
publishable:
    node scripts/check-publishable.mjs

# `just ci` cannot see a file that exists locally but is untracked or ignored.
[doc('Build from a fresh `git archive` of HEAD, as a clone would')]
verify-clean-clone:
    ./scripts/verify-clean-clone.sh

# Cargo.lock is committed (D-031), so this audits what ships.
[doc('Audit the committed lockfile for security advisories')]
audit:
    cargo audit --file rumi/Cargo.lock

# Dry-run publishing rumi-core and rumi-http
publish-dry:
    cargo publish --manifest-path rumi/core/Cargo.toml --dry-run
    cargo publish --manifest-path rumi/ext/http/Cargo.toml --dry-run
