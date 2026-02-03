#!/bin/bash
# PostToolUse hook for Write/Edit operations
# Suggests regeneration and testing after changes
# Sources project-paths.sh for canonical path definitions

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/project-paths.sh"

# Read tool input from stdin
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# Check if we're editing x.uma files
if [[ "$FILE_PATH" != *"/x.uma/"* ]]; then
    exit 0  # No suggestion needed
fi

# After proto changes, suggest regeneration
if [[ "$FILE_PATH" == *.proto ]]; then
    echo '{"additionalContext":"Proto file modified. Run `just gen` to regenerate bindings, then `just lint-proto` to validate."}'
    exit 0
fi

# After Rust changes in core, suggest tests
if [[ "$FILE_PATH" == $CORE_PATH_PATTERN && "$FILE_PATH" == *.rs ]]; then
    echo '{"additionalContext":"Core Rust file modified. Run `just test` to verify, `just lint` for lints, and `just check-no-std` for no_std compat."}'
    exit 0
fi

# After any Rust changes
if [[ "$FILE_PATH" == *.rs ]]; then
    echo '{"additionalContext":"Rust file modified. Consider running `just test` and `just lint`."}'
    exit 0
fi

exit 0
