# Route on a header

You have requests carrying a header, and you want a different decision per value,
with a safe default when the header is absent or unrecognised.

## The shape

Two things do the work. A `singlePredicate` reads one input and compares it. An
`onNoMatch` at the matcher level catches everything the list did not.

Rules are tried in order. The first one that matches wins, and evaluation stops.

## Try it

Change `tier` to `free`, then delete the field entirely. The third result is the
one worth understanding: a missing input is not an error, it is a non-match.

<matcher
  config='{
  "matcherList": {
    "matchers": [
      {
        "predicate": {
          "singlePredicate": {
            "input": { "name": "tier", "typedConfig": { "@type": "type.googleapis.com/xuma.kv.v1.MapInput", "key": "tier" } },
            "valueMatch": { "exact": "enterprise" }
          }
        },
        "onMatch": { "action": { "name": "dedicated-pool", "typedConfig": { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", "name": "dedicated-pool" } } }
      },
      {
        "predicate": {
          "singlePredicate": {
            "input": { "name": "tier", "typedConfig": { "@type": "type.googleapis.com/xuma.kv.v1.MapInput", "key": "tier" } },
            "valueMatch": { "exact": "pro" }
          }
        },
        "onMatch": { "action": { "name": "shared-pool", "typedConfig": { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", "name": "shared-pool" } } }
      }
    ]
  },
  "onNoMatch": { "action": { "name": "free-pool", "typedConfig": { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", "name": "free-pool" } } }
}'
  context='{ "tier": "enterprise" }' />

## Why a missing header is not an error

When a `DataInput` finds nothing, it returns no data, and the predicate evaluates
to `false`. It does not throw and it does not halt evaluation. The matcher simply
moves to the next rule, and eventually to `onNoMatch`.

This is deliberate. A matcher that threw on absent input would make every rule
order-sensitive to data you do not control.

## Matching on part of a value

`exact` is one of several comparisons. `prefix`, `suffix`, `contains`, and
`safeRegex` all work in the same slot:

```json
"valueMatch": { "prefix": "internal-" }
```

Prefer the cheapest comparison that expresses the rule. `Regex` is
linear-time and safe against catastrophic backtracking, but it is still the most
expensive option here, and patterns are length-capped at load time.

## In the HTTP domain

The example above uses the test domain, which reads a flat string map. For real
requests, the HTTP domain provides inputs that understand a request:

```yaml
input:
  name: header
  typedConfig:
    "@type": type.googleapis.com/xuma.http.v1.HeaderInput
    name: x-tier
```

The evaluation model does not change. Only the input does.

## Next

- [Add a custom input](custom-input.md) when no built-in input reads what you need.
- [Debug why something matched](debug-a-match.md) when a rule fires and you cannot see why.
