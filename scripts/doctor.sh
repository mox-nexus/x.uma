#!/usr/bin/env bash
#
# Check that every tool this repo needs is present, and report versions.
#
# A fresh clone needs cargo, just, uv, bun, buf, maturin, wasm-pack and
# cargo-audit. Without a check, a tool missing from one PATH and present on
# another passes locally and fails in CI.
#
# Exits non-zero if anything is missing, so it is usable as a gate.
set -uo pipefail

cd "$(dirname "$0")/.."

missing=0

# name | command | version flag | why it is needed
check() {
    local name="$1" cmd="$2" flag="$3" why="$4"

    if ! command -v "$cmd" >/dev/null 2>&1; then
        printf '  \033[31mMISSING\033[0m  %-12s %s\n' "$name" "$why"
        missing=$((missing + 1))
        return
    fi

    local version
    version=$("$cmd" $flag 2>&1 | head -1 | tr -d '\r')
    printf '  \033[32mok     \033[0m  %-12s %s\n' "$name" "$version"
}

echo "x.uma toolchain"
echo

check cargo      cargo      --version  "Rust workspace"
check rustc      rustc      --version  "Rust compiler"
check just       just       --version  "task runner"
check uv         uv         --version  "PyO3 crust env"
check bun        bun        --version  "wasm crust tests, docs site"
check node       node       --version  "doc and agreement check scripts"
check buf        buf        --version  "proto codegen (just gen)"
check maturin    maturin    --version  "PyO3 crust wheel"
check wasm-pack  wasm-pack  --version  "wasm crust package, docs site"
check cargo-audit cargo-audit --version "dependency advisories (just audit)"

echo
if [ "$missing" -gt 0 ]; then
    echo "  $missing required tool(s) missing. Install them before running 'just ci'."
    echo
    echo "  cargo/rustc  https://rustup.rs"
    echo "  just         cargo install just"
    echo "  uv           https://docs.astral.sh/uv/getting-started/installation/"
    echo "  bun          https://bun.sh/docs/installation"
    echo "  node         https://nodejs.org"
    echo "  buf          https://buf.build/docs/installation"
    echo "  maturin      uv tool install maturin"
    echo "  wasm-pack    cargo install wasm-pack"
    echo "  cargo-audit  cargo install cargo-audit"
    exit 1
fi

echo "  Everything present. 'just ci' should run clean."
