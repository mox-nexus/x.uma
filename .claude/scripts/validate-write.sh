#!/bin/bash
# PreToolUse hook for Write/Edit operations
# Validates x.uma files follow constraints
# Sources project-paths.sh for canonical path definitions

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/project-paths.sh"

# Read tool input from stdin
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
CONTENT=$(echo "$INPUT" | jq -r '.tool_input.content // .tool_input.new_string // empty')

# Check if we're editing x.uma files
if [[ "$FILE_PATH" != *"/x.uma/"* ]]; then
    exit 0  # Allow - not x.uma file
fi

# Proto file validation
if [[ "$FILE_PATH" == *.proto ]]; then
    # Check namespace follows xuma.* convention
    if echo "$CONTENT" | grep -qE "^package\s+" && ! echo "$CONTENT" | grep -qE "^package\s+xuma\."; then
        if ! echo "$CONTENT" | grep -qE "^package\s+xds\."; then
            echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"ask","reason":"Proto package should use xuma.* namespace. Is this intentional?"}}'
            exit 0
        fi
    fi
fi

# Rust file validation (core crate)
if [[ "$FILE_PATH" == $CORE_PATH_PATTERN && "$FILE_PATH" == *.rs ]]; then
    # Check for fancy-regex (ReDoS risk)
    if echo "$CONTENT" | grep -qE "fancy.regex"; then
        echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"block","reason":"fancy-regex blocked in core (ReDoS risk). Use regex crate only."}}'
        exit 0
    fi

    # Check for std usage without feature gate
    if echo "$CONTENT" | grep -qE "^use\s+std::" && ! echo "$CONTENT" | grep -qE "#\[cfg\(feature\s*=\s*\"std\"\)\]"; then
        echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"ask","additionalContext":"Core should be no_std compatible. Ensure std usage is behind #[cfg(feature = \"std\")] gate."}}'
        exit 0
    fi
fi

# Allow by default
echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}'
