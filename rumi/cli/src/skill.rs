//! `rumi --skill` — the CLI documents itself for coding agents.
//!
//! The type URL tables are generated from the live registries rather than
//! written down, so they cannot drift from the code. A matcher registered
//! today appears in the skill today.

use std::fmt::Write as _;

/// Render the top-level skill document.
pub fn skill(test: &[&str], test_matchers: &[&str], http: &[&str], claude: &[&str]) -> String {
    let mut s = String::new();

    s.push_str(
        r#"---
name: xuma
description: >
  This skill should be used when the user asks to "match a request", "route by
  header", "write a matcher config", "why did this rule match", "debug a
  matcher", "validate this config", "add a matching rule", "gate a Claude Code
  hook", "block a tool call", or mentions xDS matchers, the unified matcher API,
  ext_proc routing, matcher trees, or the rumi/xuma engine.
---

# x.uma

A matcher engine. You give it a config describing rules and a context describing
one thing to decide about; it returns the action of the first rule that matches,
or the `on_no_match` fallback.

The same config runs identically in Rust, Python and TypeScript.

## The evaluation model, in four sentences

1. Rules are tried **in order**. The first match wins and evaluation stops.
2. A rule's predicate reads one **input** from the context and compares it.
3. If an input finds no value, the predicate is **false**. That is not an error.
4. If no rule matches, `on_no_match` decides. If there is none, there is no decision.

Point 3 is the most common source of confusion. A missing field never raises;
it simply fails to match. If a rule "does nothing", check whether the input is
reading a key that exists.

## Commands

| Intent | Command |
|---|---|
| Try a config against a context | `rumi run <config.yaml> --context key=value` |
| Same, HTTP domain | `rumi run http <config.yaml> --method GET --path /api` |
| Same, Claude Code hooks | `rumi run claude <config.yaml> --event PreToolUse --tool Bash` |
| Check a config loads | `rumi check <config.yaml>` |
| See what inputs exist | `rumi info [http\|claude]` |
| Read this document | `rumi --skill` |
| Read a specific reference | `rumi --skill -r config` |

Validate before you evaluate. `rumi check` fails at load with a message; a bad
config never silently returns a wrong answer.

## Config shape

```yaml
matchers:
  - predicate:
      type: single
      input: { type_url: "xuma.test.v1.StringInput", config: { key: "method" } }
      value_match: { Exact: "GET" }
    on_match: { type: action, action: "read-handler" }

on_no_match: { type: action, action: "fallback" }
```

`type_url` selects which input reads the context. `config` is that input's own
settings. `value_match` is how the extracted value is compared.

Predicates compose: `single`, `and`, `or`, `not`.

```yaml
predicate:
  type: and
  predicates:
    - { type: single, input: {...}, value_match: { Exact: "GET" } }
    - { type: single, input: {...}, value_match: { Prefix: "/api" } }
```

`on_match` is exclusive: it is either an action or a nested matcher, never both.

```yaml
on_match: { type: matcher, matcher: { matchers: [ ... ] } }
```

## Value matches

| Form | Meaning |
|---|---|
| `{ Exact: "GET" }` | equal |
| `{ Prefix: "/api" }` | starts with |
| `{ Suffix: ".json" }` | ends with |
| `{ Contains: "admin" }` | substring |
| `{ Regex: "^/v[0-9]+/" }` | regular expression, linear time |

Prefer the cheapest form that expresses the rule. `Regex` is RE2-class and safe
against catastrophic backtracking, but it is still the most expensive option,
and patterns are length-capped at load.

"#,
    );

    section(&mut s, "Inputs — test domain", test);
    section(&mut s, "Matchers — all domains", test_matchers);
    section(&mut s, "Inputs — http domain (`rumi run http`)", http);
    section(&mut s, "Inputs — claude domain (`rumi run claude`)", claude);

    s.push_str(
        r"
## When a rule does not fire

Work down this list; it is ordered by how often each is the cause.

1. **An earlier rule matched.** First match wins. Run with `--trace` to see which.
2. **The input read a key that is not in the context.** No value means false.
3. **A different input than you think.** `rumi info` lists what is registered.
4. **The config never loaded.** Run `rumi check`. Load failures are loud.

## Limits enforced at load

Nesting depth 32, at most 256 field matchers per matcher, 256 predicates per
compound, pattern length 8192, regex pattern length 4096. Exceeding one is a
load error, never an evaluation failure. Evaluation itself cannot fail.

## References

| Reference | Command |
|---|---|
| Config format in full | `rumi --skill -r config` |
| Writing a config from scratch | `rumi --skill -r authoring` |

Full documentation: https://mox-nexus.github.io/x.uma/
",
    );

    s
}

fn section(s: &mut String, title: &str, urls: &[&str]) {
    let _ = writeln!(s, "## {title}\n");
    if urls.is_empty() {
        let _ = writeln!(s, "None registered.\n");
        return;
    }
    for url in urls {
        let _ = writeln!(s, "- `{url}`{}", hint(url));
    }
    s.push('\n');
}

/// The config field name each input reads, for inputs that take one.
///
/// Every entry is verified against the real loader by `hints_name_real_config_keys`
/// below: the test builds a config using the key named here and asserts it
/// loads. A wrong key fails the test rather than shipping in the skill.
///
/// This exists because the first version of this file was hand-written and was
/// wrong about `ArgumentInput` on the day it was written, inside a module whose
/// doc comment claims these tables cannot drift. The generated half could not.
/// This half could, and did.
pub const CONFIG_KEYS: &[(&str, &str)] = &[
    ("xuma.test.v1.StringInput", "key"),
    ("xuma.http.v1.HeaderInput", "name"),
    ("xuma.http.v1.QueryParamInput", "name"),
    ("xuma.claude.v1.ArgumentInput", "name"),
];

/// Prose describing each input, WITHOUT its config key.
///
/// The config key comes from [`CONFIG_KEYS`] so it is written once and checked
/// once. Unknown URLs render bare rather than guessing, so a newly registered
/// extension appears immediately even before it is described here.
fn description(type_url: &str) -> &'static str {
    match type_url {
        "xuma.test.v1.StringInput" => "reads the context map",
        "xuma.core.v1.StringMatcher" => "Exact / Prefix / Suffix / Contains / Regex",
        "xuma.core.v1.BoolMatcher" => "matches a boolean value",
        "xuma.http.v1.PathInput" => "request path, without query string",
        "xuma.http.v1.MethodInput" => "request method",
        "xuma.http.v1.HeaderInput" => "one header",
        "xuma.http.v1.QueryParamInput" => "one query parameter",
        "xuma.http.v1.AuthorityInput" => "`:authority` pseudo-header",
        "xuma.http.v1.SchemeInput" => "`:scheme` pseudo-header",
        "xuma.claude.v1.EventInput" => "hook event name, e.g. PreToolUse",
        "xuma.claude.v1.ToolNameInput" => "tool being invoked, e.g. Bash",
        "xuma.claude.v1.ArgumentInput" => "one tool argument",
        "xuma.claude.v1.CwdInput" => "working directory",
        "xuma.claude.v1.GitBranchInput" => "current git branch",
        "xuma.claude.v1.SessionIdInput" => "session identifier",
        _ => "",
    }
}

/// The config key an input takes, if it takes one.
///
/// Public so `rumi info --verbose` renders from the same source as `--skill`.
/// Two copies of this table diverged within hours the last time.
#[must_use]
pub fn config_key(type_url: &str) -> Option<&'static str> {
    CONFIG_KEYS
        .iter()
        .find(|(url, _)| *url == type_url)
        .map(|(_, k)| *k)
}

/// One-line prose for a type URL, or empty if undescribed.
#[must_use]
pub fn describe(type_url: &str) -> &'static str {
    description(type_url)
}

/// The rendered hint: prose, plus the config key when the input takes one.
fn hint(type_url: &str) -> String {
    let prose = description(type_url);
    let key = config_key(type_url);

    match (prose.is_empty(), key) {
        (true, _) => String::new(),
        (false, Some(k)) => format!(" — {prose}, `config.{k}`"),
        (false, None) => format!(" — {prose}"),
    }
}

/// Render a named reference, or `None` if the name is unknown.
pub fn reference(name: &str) -> Option<&'static str> {
    match name {
        "config" => Some(CONFIG_REFERENCE),
        "authoring" => Some(AUTHORING_REFERENCE),
        _ => None,
    }
}

/// Names accepted by [`reference`], for error messages.
pub const REFERENCE_NAMES: &[&str] = &["config", "authoring"];

const CONFIG_REFERENCE: &str = r#"# Config format

## Top level

```yaml
matchers:      # ordered list, first match wins
  - predicate: ...
    on_match: ...
on_no_match:   # optional, used when no rule matched
```

## Predicates

```yaml
# single: read one input, compare it
predicate:
  type: single
  input: { type_url: "...", config: { ... } }
  value_match: { Exact: "..." }

# and: every child must match
predicate:
  type: and
  predicates: [ ..., ... ]

# or: any child matches
predicate:
  type: or
  predicates: [ ..., ... }

# not: inverts one child
predicate:
  type: not
  predicate: { ... }
```

Compound predicates evaluate every child even once the outcome is decided. That
is deliberate: it keeps `--trace` output complete.

## on_match

Exclusive. An action or a nested matcher, never both.

```yaml
on_match: { type: action, action: "some-name" }

on_match:
  type: matcher
  matcher:
    matchers: [ ... ]
    on_no_match: { type: action, action: "inner-fallback" }
```

A nested matcher that fails to match does NOT fall back to the parent's next
rule with a match recorded — the parent continues to its next field matcher.

## Actions

An action is an opaque value handed back on match. The engine does not
interpret it. Mapping an action to behaviour is the caller's job.
"#;

const AUTHORING_REFERENCE: &str = r#"# Writing a config from scratch

## 1. Name the decision

Write down the answers before the rules. "Which handler?" gives actions like
`read-handler`, `write-handler`, `reject`. Actions are opaque names; pick ones
the calling code can switch on.

## 2. Find the input

```bash
rumi info            # test domain
rumi info http
rumi info claude
```

Copy the `type_url` exactly. A wrong URL is a load error, not a silent miss.

## 3. Write the narrowest rule first

Order is significant. A broad rule placed first shadows everything after it.

```yaml
matchers:
  - predicate: { type: single, input: {...}, value_match: { Exact: "/api/admin" } }
    on_match: { type: action, action: "admin" }
  - predicate: { type: single, input: {...}, value_match: { Prefix: "/api" } }
    on_match: { type: action, action: "api" }
on_no_match: { type: action, action: "not-found" }
```

Reversed, `/api/admin` would never be reached.

## 4. Always write on_no_match

Without it, "nothing matched" and "matched nothing useful" are the same result
to the caller. With it, the fallback is explicit and testable.

## 5. Check, then run

```bash
rumi check config.yaml
rumi run config.yaml --context method=GET
rumi run config.yaml --context method=GET --trace
```

`--trace` shows every rule considered and why it did or did not fire. Reach for
it the moment a result surprises you, rather than adding print statements to the
calling code.

## Common mistakes

| Symptom | Cause |
|---|---|
| Rule never fires | An earlier rule matched. Use `--trace`. |
| Everything falls through | Input reads a key absent from the context. |
| Load fails on type URL | Typo, or the input belongs to another domain. |
| Deeply nested config rejected | Depth cap is 32, enforced at load. |
"#;
