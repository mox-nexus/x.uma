# Gate a Claude Code tool call

The README shows a rule that blocks `rm -rf`. This is the path from that rule to
a hook Claude Code actually runs.

## 1. Write the rule

`hooks.yaml`:

```yaml
matcherList:
  matchers:
    - predicate:
        andMatcher:
          predicate:
            - singlePredicate:
                input:
                  name: event
                  typedConfig:
                    "@type": type.googleapis.com/xuma.claude.v1.EventTypeInput
                valueMatch:
                  exact: PreToolUse
            - singlePredicate:
                input:
                  name: tool
                  typedConfig:
                    "@type": type.googleapis.com/xuma.claude.v1.ToolNameInput
                valueMatch:
                  exact: Bash
            - singlePredicate:
                input:
                  name: command
                  typedConfig:
                    "@type": type.googleapis.com/xuma.claude.v1.ToolArgInput
                    name: command
                valueMatch:
                  contains: "rm -rf"
      onMatch:
        action:
          name: deny
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: deny
```

Check it before wiring it up. `check` reports what it read, so you can see the
rule count and the inputs it resolved rather than just `valid`:

```bash
rumi check claude hooks.yaml
```

## 2. Wire it into `settings.json`

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "rumi run claude /absolute/path/to/hooks.yaml --stdin"
          }
        ]
      }
    ]
  }
}
```

`--stdin` is the part that matters. Without it the CLI expects
`--event/--tool/--arg` flags, which is fine for experimenting from a shell and
useless as a hook: Claude Code delivers a JSON payload on stdin and reads an
exit code back.

Use an absolute path. Hooks do not run from your project directory.

## 3. The exit-code contract

| Exit | What Claude Code does |
|---|---|
| `0` | allows the tool call |
| `2` | **blocks** it, and shows stderr to the model |
| anything else | reports an error to you — **and lets the call proceed** |

`rumi` exits `2` when the matched action is `deny` or `block`, and `0`
otherwise. Use `--deny-action NAME` to nominate a different action name.

**Every failure exits `2`.** A malformed payload, an unreadable config, an
unknown event — all of them block. That is deliberate, and it follows from the
third row above: any other error code would let the call through. If `rumi`
cannot tell whether a call is safe, the only honest answer to "should this run?"
is no.

## 4. Try it without Claude Code

The hook is an ordinary program reading stdin, so you can drive it by hand:

```bash
echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rm -rf /"}}' \
  | rumi run claude hooks.yaml --stdin
echo $?    # 2 — blocked
```

```bash
echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"ls -la"}}' \
  | rumi run claude hooks.yaml --stdin
echo $?    # 0 — allowed
```

And confirm it fails closed, which is the property worth checking yourself:

```bash
echo 'not json' | rumi run claude hooks.yaml --stdin
echo $?    # 2 — blocked, because it could not decide
```

## Writing an allowlist

Match what is *allowed* and set the fallback to deny. Do not try to express it
by inverting the default:

```yaml
matcherList:
  matchers:
    - predicate:
        singlePredicate:
          input:
            name: tool
            typedConfig:
              "@type": type.googleapis.com/xuma.claude.v1.ToolNameInput
          valueMatch:
            exact: Read
      onMatch:
        action:
          name: allow
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: allow
onNoMatch:
  action:
    name: deny
    typedConfig:
      "@type": type.googleapis.com/xuma.core.v1.NamedAction
      name: deny
```

An empty `matcherList.matchers` list matches everything, so the polarity comes
entirely from how you assign actions. `rumi check` tells you which case you are
in — it prints the fallback, and says so explicitly when there are zero rules.

## The rule that catches people

**If an input finds no value, its predicate is false. That is not an error.**

A rule matching on `typedConfig: { name: "command" }` simply does not fire for a
tool call that has no `command` argument. It does not error, and it does not match.
For a deny rule that is usually what you want; for an allow rule in an
allowlist, it means the call falls through to `on_no_match`.

`rumi run claude ... --trace` shows this directly — a missing input renders as
`= None`, which is the difference between a rule that fired and lost and a rule
that never fired at all.
