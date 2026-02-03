#!/bin/bash
# Verifies all path references in project-paths.sh resolve correctly
# Run this to catch stale paths after restructuring

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/project-paths.sh"

FAILED=0

check_exists() {
    local path="$1"
    local name="$2"
    if [[ ! -e "$path" ]]; then
        echo "MISSING: $name ($path)"
        FAILED=1
    fi
}

check_dir() {
    local path="$1"
    local name="$2"
    if [[ ! -d "$path" ]]; then
        echo "NOT A DIRECTORY: $name ($path)"
        FAILED=1
    fi
}

check_file() {
    local path="$1"
    local name="$2"
    if [[ ! -f "$path" ]]; then
        echo "NOT A FILE: $name ($path)"
        FAILED=1
    fi
}

echo "Verifying project paths..."

# Directories
check_dir "$XUMA_ROOT" "XUMA_ROOT"
check_dir "$RUST_WORKSPACE" "RUST_WORKSPACE"
check_dir "$CRATE_CORE" "CRATE_CORE"
check_dir "$CRATE_CORE_SRC" "CRATE_CORE_SRC"
check_dir "$CRATE_EXT_TEST" "CRATE_EXT_TEST"
check_dir "$CRATE_EXT_HTTP" "CRATE_EXT_HTTP"
check_dir "$CRATE_EXT_CLAUDE" "CRATE_EXT_CLAUDE"
check_dir "$CLAUDE_DIR" "CLAUDE_DIR"
check_dir "$SCRIPTS_DIR" "SCRIPTS_DIR"

# Files
check_file "$RUST_MANIFEST" "RUST_MANIFEST"
check_file "$CRATE_CORE_MANIFEST" "CRATE_CORE_MANIFEST"

# Optional directories (warn if missing)
for dir in "$PROTO_DIR" "$SPEC_DIR" "$DOCS_DIR"; do
    if [[ ! -d "$dir" ]]; then
        echo "OPTIONAL MISSING: $dir"
    fi
done

if [[ $FAILED -eq 1 ]]; then
    echo ""
    echo "Path consistency check FAILED"
    echo "Update project-paths.sh to match current structure"
    exit 1
fi

echo "All paths verified!"
