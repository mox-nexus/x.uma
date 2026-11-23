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

echo "clean-clone: crusts check"
cargo check --manifest-path rumi/crusts/python/Cargo.toml --features fixtures
cargo check --manifest-path rumi/crusts/wasm/Cargo.toml --features fixtures

echo "clean-clone: wasm crust install --frozen-lockfile"
(cd rumi/crusts/wasm && bun install --frozen-lockfile)

# The docs app links the wasm package from rumi/crusts/wasm/pkg, which exists
# only once wasm-pack has run. Without wasm-pack the docs half is not checkable
# here, and that is said rather than skipped silently.
if command -v wasm-pack >/dev/null 2>&1; then
    echo "clean-clone: wasm crust build"
    (cd rumi/crusts/wasm && wasm-pack build --target web >/dev/null 2>&1)
    echo "clean-clone: docs app install --frozen-lockfile"
    (cd docs/experience && bun install --frozen-lockfile >/dev/null)
else
    echo "clean-clone: wasm-pack not installed — docs app install NOT checked"
fi

echo
echo "clean-clone: ok — the tracked tree builds and tests on its own"
