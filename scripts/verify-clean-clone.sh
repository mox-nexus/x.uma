#!/usr/bin/env bash
#
# Build from what git would actually ship, not from the working tree.
#
# `just ci` runs against the working directory, so it cannot see a file that
# exists locally but is untracked or ignored. That class has bitten this repo
# three times: 17 files hidden by a bare `lib/` gitignore pattern, a `playground`
# workspace entry pointing at a deleted directory that only fails under
# --frozen-lockfile, and generated proto code that was ignored while a crate
# depended on it.
#
# `git archive HEAD` produces exactly the tracked tree. If something is missing
# from a clone, it fails here.
set -euo pipefail

cd "$(dirname "$0")/.."

STAGE=$(mktemp -d)
echo "clean-clone: staging in $STAGE"

git archive HEAD | tar -x -C "$STAGE"
cd "$STAGE"

echo "clean-clone: files from git archive: $(find . -type f | wc -l | tr -d ' ')"

echo "clean-clone: cargo build"
cargo build --manifest-path rumi/Cargo.toml

echo "clean-clone: bun install --frozen-lockfile"
bun install --frozen-lockfile

echo "clean-clone: bumi type-check"
(cd bumi && bun run typecheck)

echo "clean-clone: puma sync + tests"
uv sync --project puma --all-groups >/dev/null
# Scoped to puma/tests: a bare pytest from the root collects the crust suites,
# which need a wheel built by maturin and are covered by their own CI job.
(cd puma && uv run pytest tests -q --no-header)

echo
echo "clean-clone: ok — the tracked tree builds and tests on its own"
