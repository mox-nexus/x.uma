#!/bin/sh
# PostToolUse note after a proto edit: the generated code and the lint must follow.
command -v jq >/dev/null 2>&1 || exit 0
path=$(jq -r '.tool_input.file_path // empty')
case "$path" in
  *.proto)
    jq -n '{hookSpecificOutput: {hookEventName: "PostToolUse",
      additionalContext: "A .proto file changed. Run `just gen`, then `just lint-proto` and `just breaking`; CI fails if the generated tree is stale."}}' ;;
esac
exit 0
