#!/bin/bash
# Proto validation for x.uma

set -e

XUMA_ROOT="${1:-$(pwd)}"
cd "$XUMA_ROOT"

echo "Validating proto files..."

# Lint
echo "Running buf lint..."
buf lint proto/

# Breaking changes (if we have a main branch)
if git rev-parse --verify main >/dev/null 2>&1; then
    echo "Checking for breaking changes against main..."
    buf breaking proto/ --against ".git#branch=main"
fi

echo "Proto validation complete!"
