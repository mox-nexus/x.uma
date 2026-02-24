# Claude Code Hooks

The Claude Code hooks domain provides context types, inputs, and a compiler for matching Claude Code hook events. Available in Rust (`rumi` with `features = ["claude"]`).

## What Are Claude Code Hooks?

Claude Code fires hook events at key moments: before a tool runs, after it completes, when a session starts, when the agent stops. x.uma's Claude domain lets you build matchers that gate these events — block dangerous commands, audit tool usage, enforce policies.

## Hook Events

Nine event types, matching the Claude Code hooks API:

| Event | When it fires |
|-------|---------------|
| `PreToolUse` | Before a tool executes |
| `PostToolUse` | After a tool completes |
| `Stop` | Main agent considers stopping |
| `SubagentStop` | Subagent considers stopping |
| `UserPromptSubmit` | User submits a prompt |
| `SessionStart` | Session begins |
| `SessionEnd` | Session ends |
| `PreCompact` | Before context compaction |
| `Notification` | Notification sent to user |

## Inputs

Six `DataInput` types extract fields from `HookContext`:

| Input | Extracts | Returns |
|-------|----------|---------|
| `EventInput` | Event type as string | `"PreToolUse"`, `"PostToolUse"`, etc. |
| `ToolNameInput` | Tool name | `"Bash"`, `"Write"`, `"Read"`, etc. |
| `ArgumentInput(name)` | Tool argument value | `string` or `null` |
| `SessionIdInput` | Session identifier | `string` |
| `CwdInput` | Current working directory | `string` |
| `GitBranchInput` | Git branch name | `string` or `null` |

## The Compiler

The `HookMatch` compiler transforms declarative rules into matchers:

```rust,ignore
use rumi::claude::prelude::*;

// Block dangerous Bash commands
let rule = HookMatch {
    event: Some(HookEvent::PreToolUse),
    tool_name: Some(StringMatch::Exact("Bash".into())),
    arguments: Some(vec![ArgumentMatch {
        name: "command".into(),
        value: StringMatch::Contains("rm -rf".into()),
    }]),
    ..Default::default()
};

let matcher = rule.compile("block")?;

let ctx = HookContext::pre_tool_use("Bash")
    .with_arg("command", "rm -rf /important");
assert_eq!(matcher.evaluate(&ctx), Some("block"));
```

### HookMatch Fields

| Field | Type | Description |
|-------|------|-------------|
| `event` | `Option<HookEvent>` | Match specific event type |
| `tool_name` | `Option<StringMatch>` | Match tool name |
| `arguments` | `Option<Vec<ArgumentMatch>>` | Match tool arguments |
| `session_id` | `Option<StringMatch>` | Match session ID |
| `cwd` | `Option<StringMatch>` | Match working directory |
| `git_branch` | `Option<StringMatch>` | Match git branch |

All fields are optional. Set fields are ANDed together. Unset fields match anything.

### StringMatch Variants

| Variant | Matches |
|---------|---------|
| `StringMatch::Exact(s)` | Exact equality |
| `StringMatch::Prefix(s)` | Starts with |
| `StringMatch::Suffix(s)` | Ends with |
| `StringMatch::Contains(s)` | Contains substring |
| `StringMatch::Regex(s)` | RE2 regex pattern |

### Multiple Rules

`compile_hook_matches` compiles multiple rules with OR semantics:

```rust,ignore
use rumi::claude::prelude::*;

let rules = vec![
    HookMatch {
        event: Some(HookEvent::PreToolUse),
        tool_name: Some(StringMatch::Exact("Bash".into())),
        arguments: Some(vec![ArgumentMatch {
            name: "command".into(),
            value: StringMatch::Contains("rm -rf".into()),
        }]),
        ..Default::default()
    },
    HookMatch {
        event: Some(HookEvent::PreToolUse),
        tool_name: Some(StringMatch::Exact("Write".into())),
        cwd: Some(StringMatch::Prefix("/etc".into())),
        ..Default::default()
    },
];

let matcher = compile_hook_matches(&rules, "block", Some("allow"));
```

Any matching rule triggers the action. First match wins.

## HookContext Builder

`HookContext` uses a builder pattern:

```rust,ignore
let ctx = HookContext::pre_tool_use("Bash")
    .with_arg("command", "ls -la")
    .with_session_id("session-123")
    .with_cwd("/home/user/project")
    .with_git_branch("main");
```

Convenience constructors: `pre_tool_use(tool)`, `post_tool_use(tool)`, `stop()`, `subagent_stop()`, `user_prompt_submit()`, `session_start()`, `session_end()`, `pre_compact()`, `notification()`.

## Tracing

Debug match decisions with `HookMatchTrace`:

```rust,ignore
let trace = rule.trace(&ctx);
for step in &trace.steps {
    println!("{}: expected={}, actual={}, matched={}",
        step.field, step.expected, step.actual, step.matched);
}
```

Each step shows the field name, expected value, actual value, and whether it matched. Trace output tells you exactly why a rule matched or didn't.

## Registry Type URLs

When using the config path:

| Type URL | Input |
|----------|-------|
| `xuma.claude.v1.EventInput` | Event type extraction |
| `xuma.claude.v1.ToolNameInput` | Tool name extraction |
| `xuma.claude.v1.ArgumentInput` | Argument extraction (config: `{"name": "..."}`) |
| `xuma.claude.v1.SessionIdInput` | Session ID extraction |
| `xuma.claude.v1.CwdInput` | Working directory extraction |
| `xuma.claude.v1.GitBranchInput` | Git branch extraction |

## Manual Construction

You can build Claude matchers without the compiler:

```rust,ignore
use rumi::prelude::*;
use rumi::claude::*;

let matcher: Matcher<HookContext, &str> = Matcher::new(
    vec![FieldMatcher::new(
        Predicate::And(vec![
            Predicate::Single(SinglePredicate::new(
                Box::new(ToolNameInput),
                Box::new(ExactMatcher::new("Bash")),
            )),
            Predicate::Single(SinglePredicate::new(
                Box::new(ArgumentInput::new("command")),
                Box::new(ContainsMatcher::new("rm -rf")),
            )),
        ]),
        OnMatch::Action("block"),
    )],
    Some(OnMatch::Action("allow")),
);
```

The compiler handles the boilerplate. Manual construction gives full control.

## Next

- [Architecture](../explain/architecture.md) — how domains plug into the core
- [Adding a Domain Adapter](../guides/adding-domain.md) — build your own domain
- [Config Format](../reference/config.md) — JSON/YAML config for Claude matchers
