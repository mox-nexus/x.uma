# Config Format

The config format is canonical protojson — protobuf's own JSON mapping of
`xds.type.matcher.v3.Matcher`. All five implementations (rumi, puma, bumi, and
both `xuma-crust` bindings) read the same document and agree on the same
answers; see [`DECISIONS.md` D-026](https://github.com/mox-nexus/x.uma/blob/main/DECISIONS.md)
for why this format and not a bespoke one.

**Field names accept `lowerCamelCase` (canonical) or the proto's own
`snake_case`.** A third spelling, or a field the schema does not define, is a
load error — not a silently ignored typo. That is deliberate: a misspelled key
in a deny rule must not become a rule that never fires.

## Matcher

Top-level config for a matcher — a `oneof`, so exactly one of `matcherList` /
`matcherTree` is set. `matcherTree` is not implemented; use `matcherList`.

```json
{
  "matcherList": { "matchers": [ ... ] },
  "onNoMatch": { ... }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `matcherList` | `MatcherList` | One of | Field matchers evaluated in order, first match wins |
| `matcherTree` | `MatcherTree` | One of | Not implemented — rejected at load |
| `onNoMatch` | `OnMatch` | No | Fallback when no field matcher matches |

## FieldMatcher

A single rule inside `matcherList.matchers`: predicate + action.

```json
{
  "predicate": { ... },
  "onMatch": { ... }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `predicate` | `Predicate` | Yes | Condition to evaluate |
| `onMatch` | `OnMatch` | Yes | What to do when predicate matches |

## Predicate

A `oneof`: exactly one of `singlePredicate` / `andMatcher` / `orMatcher` /
`notMatcher` is set.

### singlePredicate

Extract a value and match it:

```json
{
  "singlePredicate": {
    "input": {
      "name": "method",
      "typedConfig": { "@type": "type.googleapis.com/xuma.kv.v1.MapInput", "key": "method" }
    },
    "valueMatch": { "exact": "GET" }
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `input` | `TypedExtensionConfig` | Yes | Data input reference (resolved via registry) |
| `valueMatch` | `StringMatcher` | One of | Built-in string match |
| `customMatch` | `TypedExtensionConfig` | One of | Custom matcher via registry |

Exactly one of `valueMatch` or `customMatch` must be set — setting both, or
neither, is a load error.

### andMatcher / orMatcher

All (`andMatcher`) or any (`orMatcher`) child predicates must match:

```json
{ "andMatcher": { "predicate": [ { "singlePredicate": { ... } }, { "singlePredicate": { ... } } ] } }
```

### notMatcher

Negate a predicate:

```json
{ "notMatcher": { "singlePredicate": { ... } } }
```

## OnMatch

A `oneof`: exactly one of `action` / `matcher` is set — never both, never
neither. This enforces `OnMatch` exclusivity from the xDS spec.

### action

Return a value. `xuma.core.v1.NamedAction` is the action type this engine
ships — its own `name` field carries the value the engine returns:

```json
{
  "action": {
    "name": "route-get",
    "typedConfig": { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", "name": "route-get" }
  }
}
```

An empty `name` is a load error, not a rule that fires and returns nothing.
`metadata` is an optional `map<string, string>` for attaching data the engine
does not interpret.

### matcher

Continue evaluation with a nested matcher:

```json
{
  "matcher": {
    "matcherList": { "matchers": [ ... ] },
    "onNoMatch": { ... }
  }
}
```

A nested matcher that fails to match does **not** fall back to the parent's
next rule with a match recorded — the parent continues to its next field
matcher.

`keepMatching: true` is a sibling field of `action`/`matcher` in xDS. It is
**not implemented** — it would record the action and keep evaluating, and this
engine returns the first match — so setting it is a load error rather than a
silently ignored field.

## TypedExtensionConfig

Reference to a registered type, carrying an `Any` payload:

```json
{
  "name": "method",
  "typedConfig": {
    "@type": "type.googleapis.com/xuma.kv.v1.MapInput",
    "key": "method"
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | A label, not part of matching — any short string |
| `typedConfig` | `Any` | Yes | `@type` names the message; its own fields sit beside it |

`@type` must carry the full `type.googleapis.com/` prefix — protojson requires
it, and a bare name is refused. It is resolved at load time; unknown type URLs
produce an error listing available types.

## StringMatcher

Built-in string matchers, plus an optional `ignoreCase` sibling:

```json
{ "exact": "hello" }
{ "prefix": "/api" }
{ "suffix": ".json" }
{ "contains": "admin" }
{ "safeRegex": { "regex": "^Bearer .+$" } }
{ "exact": "hello", "ignoreCase": true }
```

| Variant | Matches |
|---------|---------|
| `exact` | Exact string equality |
| `prefix` | String starts with value |
| `suffix` | String ends with value |
| `contains` | String contains value |
| `safeRegex.regex` | RE2 regex pattern (linear time) |
| `ignoreCase` | Case-insensitive comparison, any variant above |

`ignoreCase: true` on a regex that turns case-insensitivity back off inline
with `(?-i)` is a load error — an inline flag always wins, so the combination
would silently read one way and behave another.

## Type URL Reference

### Core (all domains)

Registered by `register_core_matchers()` in all implementations:

| Type URL | Type | Config |
|----------|------|--------|
| `xuma.core.v1.NamedAction` | action | `{ "name": "...", "metadata": {...} }` |

### Key-Value Domain

The domain `rumi run` uses by default:

| Type URL | Config | Extracts |
|----------|--------|----------|
| `xuma.kv.v1.MapInput` | `{ "key": "method" }` | Value for key from a string map context |

### HTTP Domain

| Type URL | Config | Extracts |
|----------|--------|----------|
| `xuma.http.v1.PathInput` | `{}` | Request path, without query string |
| `xuma.http.v1.MethodInput` | `{}` | HTTP method |
| `xuma.http.v1.HeaderInput` | `{ "name": "content-type" }` | Header value by name |
| `xuma.http.v1.QueryParamInput` | `{ "name": "page" }` | Query parameter by name |
| `xuma.http.v1.AuthorityInput` | `{}` | `:authority` pseudo-header (Host in HTTP/1) |
| `xuma.http.v1.SchemeInput` | `{}` | `:scheme` pseudo-header |

### Claude Domain

| Type URL | Config | Extracts |
|----------|--------|----------|
| `xuma.claude.v1.EventTypeInput` | `{}` | Hook event name (e.g. `PreToolUse`) |
| `xuma.claude.v1.ToolNameInput` | `{}` | Tool name (e.g. `Bash`) |
| `xuma.claude.v1.ToolArgInput` | `{ "name": "command" }` | Tool argument by name |
| `xuma.claude.v1.SessionIdInput` | `{}` | Session ID |
| `xuma.claude.v1.CwdInput` | `{}` | Working directory |
| `xuma.claude.v1.GitBranchInput` | `{}` | Git branch |

## Full Examples

### HTTP Route Matching

```yaml
matcherList:
  matchers:
    - predicate:
        andMatcher:
          predicate:
            - singlePredicate:
                input:
                  name: path
                  typedConfig:
                    "@type": type.googleapis.com/xuma.http.v1.PathInput
                valueMatch:
                  prefix: /api
            - singlePredicate:
                input:
                  name: method
                  typedConfig:
                    "@type": type.googleapis.com/xuma.http.v1.MethodInput
                valueMatch:
                  exact: GET
      onMatch:
        action:
          name: api_read
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: api_read

    - predicate:
        singlePredicate:
          input:
            name: content-type
            typedConfig:
              "@type": type.googleapis.com/xuma.http.v1.HeaderInput
              name: content-type
          valueMatch:
            exact: application/json
      onMatch:
        action:
          name: json_handler
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: json_handler

onNoMatch:
  action:
    name: not_found
    typedConfig:
      "@type": type.googleapis.com/xuma.core.v1.NamedAction
      name: not_found
```

### Claude Code Hook Policy

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
          name: block
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: block

onNoMatch:
  action:
    name: allow
    typedConfig:
      "@type": type.googleapis.com/xuma.core.v1.NamedAction
      name: allow
```

### Key-Value Domain

```yaml
matcherList:
  matchers:
    - predicate:
        andMatcher:
          predicate:
            - singlePredicate:
                input:
                  name: method
                  typedConfig:
                    "@type": type.googleapis.com/xuma.kv.v1.MapInput
                    key: method
                valueMatch:
                  exact: GET
            - singlePredicate:
                input:
                  name: path
                  typedConfig:
                    "@type": type.googleapis.com/xuma.kv.v1.MapInput
                    key: path
                valueMatch:
                  prefix: /api
      onMatch:
        action:
          name: api_get
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: api_get

    - predicate:
        singlePredicate:
          input:
            name: path
            typedConfig:
              "@type": type.googleapis.com/xuma.kv.v1.MapInput
              key: path
          valueMatch:
            exact: /health
      onMatch:
        action:
          name: health
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: health

onNoMatch:
  action:
    name: not_found
    typedConfig:
      "@type": type.googleapis.com/xuma.core.v1.NamedAction
      name: not_found
```

## Validation Limits

Configs are validated at load time:

| Limit | Value | Error |
|-------|-------|-------|
| Max nesting depth | 32 levels | `DepthExceeded` |
| Max field matchers per matcher | 256 | `TooManyFieldMatchers` |
| Max predicates per AND/OR | 256 | `TooManyPredicates` |
| Max pattern length | 8192 chars | `PatternTooLong` |
| Max regex pattern length | 4096 chars | `PatternTooLong` |
| Max document nesting (before a matcher exists) | 128 levels | rejected during `@type` expansion |

If a config loads successfully, the resulting matcher is guaranteed to be
structurally valid. Parse, don't validate.
