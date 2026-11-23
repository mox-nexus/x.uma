#!/bin/sh
# PreToolUse guard for Bash. Silence is "no decision", so the normal permission flow still runs;
# only the commands below get a deny. If jq is missing the guard stays silent rather than
# blocking every command.
command -v jq >/dev/null 2>&1 || exit 0
cmd=$(jq -r '.tool_input.command // empty')

deny() {
  jq -n --arg r "$1" '{hookSpecificOutput: {hookEventName: "PreToolUse",
    permissionDecision: "deny", permissionDecisionReason: $r}}'
  exit 0
}

if printf '%s' "$cmd" | grep -Eq 'git[[:space:]]+push([[:space:]].*)?[[:space:]](--force(-with-lease)?|-f)([[:space:]]|$)'; then
  deny "Force push is not used in x.uma. Push normally and merge through a pull request."
fi
if printf '%s' "$cmd" | grep -Eq 'git[[:space:]]+reset([[:space:]].*)?[[:space:]]--hard'; then
  deny "git reset --hard discards work. Use git stash, or check out the specific files."
fi
if printf '%s' "$cmd" | grep -Eq '(cargo|maturin|npm|wasm-pack)[[:space:]]+publish' &&
   ! printf '%s' "$cmd" | grep -Eq -- '--dry-run'; then
  deny "Publishing happens through the release workflows, not from a session. Use --dry-run, or just publish-dry."
fi
exit 0
