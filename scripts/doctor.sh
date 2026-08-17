#!/usr/bin/env bash
#
# Check that every tool this repo needs is present, and report versions.
#
# A fresh clone needs cargo, just, uv, bun, buf, maturin, wasm-pack and
# cargo-audit, and until 2026-08-17 nothing said so and nothing checked. Two
# defects that week traced straight to it: the wasm crust had never been built
# on the machine that claimed it worked, and `uv run maturin` failed in CI while
# passing locally because maturin happened to be on one PATH and not the other.
#
# Exits non-zero if anything required is missing, so it is usable as a gate.
set -uo pipefail

cd "$(dirname "$0")/.."

missing=0
optional_missing=0

# name | command | version flag | why it is needed | required?
check() {
    local name="$1" cmd="$2" flag="$3" why="$4" required="$5"

    if ! command -v "$cmd" >/dev/null 2>&1; then
        if [ "$required" = "yes" ]; then
            printf '  \033[31mMISSING\033[0m  %-12s %s\n' "$name" "$why"
            missing=$((missing + 1))
        else
            printf '  \033[33mabsent \033[0m  %-12s %s (optional)\n' "$name" "$why"
            optional_missing=$((optional_missing + 1))
        fi
        return
    fi

    local version
    version=$("$cmd" $flag 2>&1 | head -1 | tr -d '\r')
    printf '  \033[32mok     \033[0m  %-12s %s\n' "$name" "$version"
}

echo "x.uma toolchain"
echo

check cargo      cargo      --version  "Rust workspace"                       yes
check rustc      rustc      --version  "Rust compiler"                        yes
check just       just       --version  "task runner"                          yes
check uv         uv         --version  "Python env + puma"                    yes
check bun        bun        --version  "bumi, docs site"                      yes
check node       node       --version  "doc and agreement check scripts"      yes
check buf        buf        --version  "proto codegen (just gen)"             yes
check maturin    maturin    --version  "PyO3 crust wheel"                     no
check wasm-pack  wasm-pack  --version  "wasm crust package"                   no
check cargo-audit cargo-audit --version "dependency advisories (just audit)"  no

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
    exit 1
fi

if [ "$optional_missing" -gt 0 ]; then
    echo "  All required tools present. $optional_missing optional tool(s) absent:"
    echo "    maturin      uv pip install maturin      (needed by: just crust-py-check)"
    echo "    wasm-pack    cargo install wasm-pack     (needed by: just crust-wasm-check)"
    echo "    cargo-audit  cargo install cargo-audit   (needed by: just audit)"
    echo
    echo "  'just ci' will run; the crust checks and audit will not."
    exit 0
fi

echo "  Everything present. 'just ci' should run clean."
