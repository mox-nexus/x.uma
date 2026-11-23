#!/bin/sh
# PreToolUse guard for Write and Edit. Silence is "no decision"; the checks below deny or ask.
command -v jq >/dev/null 2>&1 || exit 0
input=$(cat)
path=$(printf '%s' "$input" | jq -r '.tool_input.file_path // empty')
text=$(printf '%s' "$input" | jq -r '[.tool_input.content, .tool_input.new_string, (.tool_input.edits[]?.new_string)] | map(select(. != null)) | join("\n")')

decide() {
  jq -n --arg d "$1" --arg r "$2" '{hookSpecificOutput: {hookEventName: "PreToolUse",
    permissionDecision: $d, permissionDecisionReason: $r}}'
  exit 0
}

case "$path" in
  */rumi/*.rs|*/rumi/*Cargo.toml)
    if printf '%s' "$text" | grep -Eq 'fancy[-_]regex'; then
      decide deny "fancy-regex is not allowed in rumi: it backtracks, so a config could hang the process. Use the regex crate."
    fi ;;
  */proto/xuma/*.proto)
    if printf '%s' "$text" | grep -Eq '^package[[:space:]]+' &&
       ! printf '%s' "$text" | grep -Eq '^package[[:space:]]+xuma\.'; then
      decide ask "x.uma extensions use the xuma.<area>.v1 package namespace. Is a different package intended?"
    fi ;;
esac
exit 0
