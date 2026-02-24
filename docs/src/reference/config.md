# Config Format

The config format is shared across all five implementations. Same JSON/YAML structure, same semantics.

## MatcherConfig

Top-level config for a matcher:

```json
{
  "matchers": [ ... ],
  "on_no_match": { ... }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `matchers` | array of `FieldMatcherConfig` | Yes | Field matchers evaluated in order |
| `on_no_match` | `OnMatchConfig` | No | Fallback when no field matcher matches |

## FieldMatcherConfig

A single rule: predicate + action:

```json
{
  "predicate": { ... },
  "on_match": { ... }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `predicate` | `PredicateConfig` | Yes | Condition to evaluate |
| `on_match` | `OnMatchConfig` | Yes | What to do when predicate matches |

## PredicateConfig

Boolean logic over conditions. Discriminated by `type`:

### single

Extract a value and match it:

```json
{
  "type": "single",
  "input": { "type_url": "xuma.test.v1.StringInput", "config": { "key": "method" } },
  "value_match": { "Exact": "GET" }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | `"single"` | Yes | Discriminator |
| `input` | `TypedConfig` | Yes | Data input reference (resolved via registry) |
| `value_match` | `ValueMatch` | One of | Built-in string match |
| `custom_match` | `TypedConfig` | One of | Custom matcher via registry |

Exactly one of `value_match` or `custom_match` must be set.

### and

All child predicates must match:

```json
{
  "type": "and",
  "predicates": [ { "type": "single", ... }, { "type": "single", ... } ]
}
```

### or

Any child predicate must match:

```json
{
  "type": "or",
  "predicates": [ { "type": "single", ... }, { "type": "single", ... } ]
}
```

### not

Negate a predicate:

```json
{
  "type": "not",
  "predicate": { "type": "single", ... }
}
```

## OnMatchConfig

Either a terminal action or a nested matcher. Discriminated by `type`:

### action

Return a value:

```json
{ "type": "action", "action": "route-get" }
```

The `action` field can be any JSON value — string, number, object. The engine doesn't interpret it.

### matcher

Continue evaluation with a nested matcher:

```json
{
  "type": "matcher",
  "matcher": {
    "matchers": [ ... ],
    "on_no_match": { ... }
  }
}
```

Action XOR matcher — never both. This enforces OnMatch exclusivity from the xDS spec.

## TypedConfig

Reference to a registered type:

```json
{ "type_url": "xuma.test.v1.StringInput", "config": { "key": "method" } }
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type_url` | string | Yes | Registered type identifier |
| `config` | object | No (defaults to `{}`) | Type-specific configuration |

The `type_url` is resolved at load time via the Registry. Unknown type URLs produce an error listing available types.

## ValueMatch

Built-in string matchers:

```json
{ "Exact": "hello" }
{ "Prefix": "/api" }
{ "Suffix": ".json" }
{ "Contains": "admin" }
{ "Regex": "^Bearer .+$" }
```

| Variant | Matches |
|---------|---------|
| `Exact` | Exact string equality |
| `Prefix` | String starts with value |
| `Suffix` | String ends with value |
| `Contains` | String contains value |
| `Regex` | RE2 regex pattern (linear time) |

## Core Type URLs

Registered by `register_core_matchers()` in all implementations:

| Type URL | Type | Config |
|----------|------|--------|
| `xuma.core.v1.StringMatcher` | InputMatcher | `StringMatchSpec` |
| `xuma.core.v1.BoolMatcher` | InputMatcher | `{ "value": true }` |

## Full Example

YAML config matching HTTP-like requests:

```yaml
matchers:
  - predicate:
      type: and
      predicates:
        - type: single
          input: { type_url: "xuma.test.v1.StringInput", config: { key: "method" } }
          value_match: { Exact: "GET" }
        - type: single
          input: { type_url: "xuma.test.v1.StringInput", config: { key: "path" } }
          value_match: { Prefix: "/api" }
    on_match: { type: action, action: "api_get" }

  - predicate:
      type: single
      input: { type_url: "xuma.test.v1.StringInput", config: { key: "path" } }
      value_match: { Exact: "/health" }
    on_match: { type: action, action: "health" }

on_no_match: { type: action, action: "not_found" }
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

If a config loads successfully, the resulting matcher is guaranteed to be structurally valid. Parse, don't validate.
