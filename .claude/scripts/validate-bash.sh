#!/bin/bash
# PreToolUse hook for Bash commands
# Blocks dangerous commands in x.uma repo

# Read tool input from stdin
INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

# Check if we're in x.uma directory
if [[ "$PWD" != *"/x.uma"* ]]; then
    exit 0  # Allow - not in x.uma
fi

# Block dangerous git commands
if echo "$COMMAND" | grep -qE "git\s+push\s+.*--force|git\s+push\s+-f"; then
    echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"block","reason":"Force push blocked in x.uma. Use regular push or create PR."}}'
    exit 0
fi

if echo "$COMMAND" | grep -qE "git\s+reset\s+--hard"; then
    echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"block","reason":"Hard reset blocked. Use git stash or careful checkout instead."}}'
    exit 0
fi

# Block cargo publish without --dry-run
if echo "$COMMAND" | grep -qE "cargo\s+publish" && ! echo "$COMMAND" | grep -qE "--dry-run"; then
    echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"block","reason":"Use cargo publish --dry-run first. Direct publish blocked."}}'
    exit 0
fi

# Allow by default
echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}'
