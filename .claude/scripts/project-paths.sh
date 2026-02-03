#!/bin/bash
# Single source of truth for x.uma project paths
# Source this in all scripts: source "${SCRIPT_DIR}/project-paths.sh"
#
# This file defines the canonical project structure. When paths change,
# update this file only - all scripts derive their paths from here.

# Project root (auto-detected or overridden)
XUMA_ROOT="${XUMA_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"

# Rust workspace
RUST_WORKSPACE="${XUMA_ROOT}/rumi"
RUST_MANIFEST="${RUST_WORKSPACE}/Cargo.toml"

# Core crate (package name: rumi, directory: rumi/core)
CRATE_CORE="${RUST_WORKSPACE}/core"
CRATE_CORE_SRC="${CRATE_CORE}/src"
CRATE_CORE_MANIFEST="${CRATE_CORE}/Cargo.toml"
CRATE_CORE_PACKAGE="rumi"

# Extension crates
CRATE_EXT_TEST="${RUST_WORKSPACE}/ext/test"
CRATE_EXT_HTTP="${RUST_WORKSPACE}/ext/http"
CRATE_EXT_CLAUDE="${RUST_WORKSPACE}/ext/claude"

# Proto and spec
PROTO_DIR="${XUMA_ROOT}/proto"
SPEC_DIR="${XUMA_ROOT}/spec"
SPEC_TESTS="${SPEC_DIR}/tests"

# Documentation
DOCS_DIR="${XUMA_ROOT}/docs"

# Claude configuration
CLAUDE_DIR="${XUMA_ROOT}/.claude"
SCRIPTS_DIR="${CLAUDE_DIR}/scripts"

# Pattern matchers (for grep/find operations)
# These match the core crate in path patterns
CORE_PATH_PATTERN="*/rumi/core/*"
CORE_RS_PATTERN="*/rumi/core/*.rs"

# Verify function (call to validate paths exist)
verify_paths() {
    local failed=0

    for path in "$RUST_MANIFEST" "$CRATE_CORE_SRC" "$CLAUDE_DIR"; do
        if [[ ! -e "$path" ]]; then
            echo "MISSING: $path" >&2
            failed=1
        fi
    done

    return $failed
}

# Export for subshells
export XUMA_ROOT RUST_WORKSPACE RUST_MANIFEST
export CRATE_CORE CRATE_CORE_SRC CRATE_CORE_MANIFEST CRATE_CORE_PACKAGE
export CRATE_EXT_TEST CRATE_EXT_HTTP CRATE_EXT_CLAUDE
export PROTO_DIR SPEC_DIR SPEC_TESTS DOCS_DIR
export CLAUDE_DIR SCRIPTS_DIR
export CORE_PATH_PATTERN CORE_RS_PATTERN
