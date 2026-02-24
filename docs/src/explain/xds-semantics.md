# xDS Semantics

x.uma implements the evaluation semantics of the [xDS Unified Matcher API](https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/advanced/matching/matching_api), the matching system used by Envoy at Google scale. This page explains where these semantics come from and why they matter.

## What is xDS?

xDS is a family of discovery service APIs developed for Envoy proxy. The "x" is a wildcard — CDS (Cluster), RDS (Route), LDS (Listener), EDS (Endpoint), and others. These APIs define how a data plane (Envoy) receives configuration from a control plane.

The **Unified Matcher API** is the matching subsystem within xDS. It replaces Envoy's earlier route-specific matching with a generic, tree-structured matcher that works across all xDS resource types. x.uma implements this matcher's evaluation semantics.

## The Proto Specification

The xDS matcher is defined in protobuf. The key messages:

```protobuf
// Simplified from xds.type.matcher.v3
message Matcher {
  message MatcherList {
    repeated FieldMatcher matchers = 1;
  }

  message MatcherTree { /* radix/map matching */ }

  oneof matcher_type {
    MatcherList matcher_list = 1;
    MatcherTree matcher_tree = 2;
  }

  Matcher.OnMatch on_no_match = 3;
}

message Matcher.OnMatch {
  oneof on_match {
    Matcher matcher = 1;              // nested matcher (continue)
    core.v3.TypedExtensionConfig action = 2;  // terminal action
  }
}

message Matcher.MatcherList.FieldMatcher {
  Predicate predicate = 1;
  Matcher.OnMatch on_match = 2;
}
```

Three things to notice:

1. **OnMatch is exclusive** — `oneof` means action XOR nested matcher. Never both. The proto schema makes illegal states unrepresentable.

2. **`on_no_match` is at the Matcher level**, not per-FieldMatcher. This is deliberate — it means "nothing in this matcher matched", not "this particular rule didn't match".

3. **TypedExtensionConfig** is the extension point. Inputs and actions are identified by type URL, enabling new domains without changing the proto schema.

## Evaluation Rules

x.uma implements six rules from the xDS specification. These are not design choices — they are protocol obligations.

### Rule 1: First-Match-Wins

`MatcherList` evaluates field matchers in order. The first match wins. Later entries are never consulted.

This comes from xDS `keep_matching` semantics. Envoy's implementation records the action but returns no-match when `keep_matching` is true, allowing later rules to override. x.uma enforces the simpler invariant: first match is final.

**Consequence:** Rule order matters. Put specific rules before general ones. `/api/v2` must come before `/api` if you need to distinguish them.

### Rule 2: OnMatch Exclusivity

Each `OnMatch` is either a terminal action or a nested matcher. The `oneof` in the proto enforces this — you cannot specify both.

x.uma carries this into the type system:

```rust
// Rust: enum makes illegal states unrepresentable
pub enum OnMatch<Ctx, A> {
    Action(A),
    Matcher(Box<Matcher<Ctx, A>>),
}
```

```python
# Python: union type
type OnMatch[Ctx, A] = Action[A] | NestedMatcher[Ctx, A]
```

```typescript
// TypeScript: discriminated union
type OnMatch<Ctx, A> = Action<A> | NestedMatcher<Ctx, A>;
```

When a predicate matches, the outcome is unambiguous.

### Rule 3: Nested Matcher Failure

This is the subtlest rule. If a predicate matches and its `OnMatch` is a nested matcher, but that nested matcher returns no match, **the parent continues to the next field matcher**.

The nested matcher's failure does not trigger the parent's `on_no_match`. It means "this branch didn't match — try the next one."

This comes from Envoy's implementation: a nested matcher returning no-match causes the parent `FieldMatcher` to be treated as non-matching, and evaluation proceeds to the next entry in the list.

### Rule 4: on_no_match Fallback

If no field matcher in `matcher_list` produces a match, the `Matcher` consults its `on_no_match` field. If absent, returns null.

`on_no_match` applies **only** when no predicate matched. It does not apply when a nested matcher failed (Rule 3). The distinction matters:

- No predicate matched → `on_no_match`
- Predicate matched, nested matcher failed → continue to next field matcher → eventually `on_no_match` if nothing else matches

### Rule 5: None-to-False

When a `DataInput` returns null (data not present), the predicate evaluates to false. The `InputMatcher` is never called.

This is a security invariant: missing data never accidentally matches. A header that doesn't exist cannot satisfy `ExactMatcher("secret")`.

### Rule 6: Depth Validation

Matcher trees exceeding MAX_DEPTH (32 levels) are rejected at construction time, not evaluation time.

This prevents stack overflow from deeply nested matchers and enforces the "parse, don't validate" principle — if a `Matcher` object exists, it's known to be structurally valid.

## TypedExtensionConfig: The Extension Seam

The xDS spec uses `TypedExtensionConfig` for both inputs and actions:

```protobuf
message TypedExtensionConfig {
  string name = 1;
  google.protobuf.Any typed_config = 2;
}
```

x.uma uses this as the extension mechanism. Each domain registers its types under a namespace:

| Namespace | Domain | Example Types |
|-----------|--------|---------------|
| `xuma.core.v1` | Core matchers | `StringMatcher`, `BoolMatcher` |
| `xuma.test.v1` | Test/conformance | `StringInput` |
| `xuma.http.v1` | HTTP matching | `PathInput`, `HeaderInput`, `MethodInput` |
| `xuma.claude.v1` | Claude Code hooks | `EventInput`, `ToolNameInput`, `ArgumentInput` |

The registry resolves type URLs to concrete implementations at config load time. Unknown type URLs are rejected with helpful error messages listing available types.

## Where x.uma Diverges from xDS

x.uma implements xDS evaluation semantics but is not a full xDS client:

| Feature | xDS | x.uma |
|---------|-----|-------|
| Evaluation semantics | Full spec | Implemented |
| `keep_matching` | Supported | Not supported (first-match-wins only) |
| MatcherTree (radix) | Supported | Supported (Rust only, not in config path yet) |
| xDS transport (gRPC) | Required | Not implemented — x.uma loads config from files |
| ADS/SotW/Delta | Discovery protocols | Not applicable |
| Resource versioning | Built-in | Not applicable |

x.uma takes the evaluation engine and makes it portable across languages. It does not implement the xDS discovery protocol — config arrives through files or programmatic construction, not gRPC streams.

## Why xDS Semantics?

Three reasons:

**Battle-tested.** These semantics run in production at every company that uses Envoy. The edge cases — nested matcher failure, `on_no_match` scoping, ordering — have been debugged at scale.

**Specification-grade.** The proto definition is unambiguous. When five implementations need to agree on behavior, ambiguity is the enemy. xDS gives us a formal spec to implement against.

**Extensible by design.** `TypedExtensionConfig` was designed for exactly this use case — adding new domains without changing the core protocol. x.uma's HTTP and Claude Code domains prove the pattern works.

## Next

- [First-Match-Wins Semantics](../concepts/semantics.md) — the six rules with code examples
- [Architecture](architecture.md) — how x.uma implements these semantics
- [Config Format](../reference/config.md) — the config schema that maps to xDS structures
