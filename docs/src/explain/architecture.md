# Architecture

x.uma makes one bet: the boundary between "what data do I have?" and "how do I match it?" is the most valuable seam in a matcher engine.

## The Bet

Every matcher engine faces a choice. Couple the matching logic to the domain, and you get performance and simplicity — but you rebuild the engine for every new domain. Decouple them, and you get reuse — but you pay in abstraction tax and runtime indirection.

x.uma's answer: erase the type at the **data level**, not the matcher level. One `ExactMatcher` works for HTTP paths, Claude Code tool names, gRPC service identifiers, and types that don't exist yet.

```text
Context (your data)
    ↓
DataInput.get()          ← knows your type, returns erased data
    ↓
MatchingData             ← string | int | bool | bytes | null
    ↓
InputMatcher.matches()   ← doesn't know your type, doesn't need to
    ↓
bool
```

The split happens at `MatchingData`. Above it, domain-specific code that knows about `HttpRequest` or `HookContext`. Below it, domain-agnostic matchers that work with primitives. This is the seam.

## Why This Works

The insight comes from Envoy, where it runs at Google scale. Envoy's xDS Unified Matcher API uses the same split — domain-specific inputs feed type-erased data into generic matchers. x.uma implements these semantics in Rust, Python, and TypeScript.

What the seam buys:

**Write a matcher once, use it everywhere.** `PrefixMatcher("/api")` matches HTTP paths, event source URIs, file paths — anything that produces a string through `MatchingData`. Five string matchers (`Exact`, `Prefix`, `Suffix`, `Contains`, `Regex`) cover most matching needs across all domains.

**Add a domain without touching core.** HTTP matching, Claude Code hooks, and the test domain all plug in by implementing `DataInput` — a single method that extracts a value from the context. The core engine never changes.

**Share config across languages.** The same JSON/YAML config produces the same matcher tree in Rust, Python, and TypeScript. `MatchingData` is the same name, same semantics, in all three.

## The Shape

```text
┌─────────────────────────────────────────┐
│           Domain Adapters               │
│   xuma.http    xuma.claude    xuma.test │
│   (DataInput implementations)           │
└──────────────────┬──────────────────────┘
                   │  get() → MatchingData
                   ↓
┌──────────────────▼──────────────────────┐
│           Core Engine                   │
│   Matcher · Predicate · InputMatcher    │
│   (domain-agnostic, immutable)          │
└─────────────────────────────────────────┘
```

Two ports define the boundary:

| Port | Direction | Generic? | You implement |
|------|-----------|----------|---------------|
| **DataInput** | Domain → Core | Yes (knows `Ctx`) | One per field you want to match |
| **InputMatcher** | Core → bool | No (knows `MatchingData`) | Rarely — five ship with x.uma |

Domain adapters implement `DataInput`. The core ships `InputMatcher` implementations. `SinglePredicate` wires one to the other.

## ACES

The architecture follows four properties:

**Adaptable.** New domains plug in without modifying core. HTTP matching didn't require changes to the predicate engine. Claude Code hooks didn't require changes to HTTP matching. Each domain is independent.

**Composable.** Predicates compose with AND, OR, NOT. Matchers nest up to 32 levels deep. A matcher's action can be another matcher, creating trees of arbitrary complexity from simple building blocks.

**Extensible.** `TypedExtensionConfig` from the xDS protobuf spec is the extension seam. Every input and action is identified by a type URL (`xuma.http.v1.PathInput`, `xuma.claude.v1.ToolNameInput`). New types register without modifying existing ones.

**Sustainable.** Core is stable. Growth happens at the edges. Adding a domain means adding `DataInput` implementations and a compiler — not touching `Matcher`, `Predicate`, or `InputMatcher`. The architecture sustains extension without rewrites.

## What Core Owns

The core engine (`rumi` in Rust, `xuma` in Python/TypeScript) provides:

- **Matcher** — first-match-wins evaluation over a list of field matchers
- **Predicate** — Boolean tree (Single, And, Or, Not) with short-circuit evaluation
- **SinglePredicate** — pairs a `DataInput` with an `InputMatcher`
- **MatchingData** — the type-erased bridge (`string | int | bool | bytes | null`)
- **InputMatcher** — five string matchers plus `BoolMatcher`
- **OnMatch** — action XOR nested matcher (illegal states unrepresentable)
- **Depth/width limits** — MAX_DEPTH=32, MAX_FIELD_MATCHERS=256
- **Registry** — immutable type registry for config-driven construction
- **Trace** — step-by-step evaluation debugging

Core does not own domain knowledge. It does not know what an HTTP request is, what a Claude Code hook event is, or what your custom context type contains. It matches erased values.

## What Domains Own

Each domain provides:

- **Context type** — `HttpRequest`, `HookContext`, your type
- **DataInput implementations** — extractors for each matchable field
- **Compiler** — transforms domain-specific config into matcher trees
- **Registry function** — registers domain inputs with the type registry

The compiler is the user-facing API. Instead of manually constructing predicate trees, you write:

```rust
// HTTP: Gateway API config → matcher
let matcher = compile_route_matches(&routes, "allowed", "denied");

// Claude: hook rules → matcher
let matcher = rule.compile("block")?;
```

Compilers are syntactic sugar over the core engine. They produce the same `Matcher<Ctx, A>` you'd build by hand.

## Matcher Engine, Not Policy Engine

x.uma is a matcher engine. It finds the first matching rule and returns an action. It does not interpret that action.

The generic `A` in `Matcher<Ctx, A>` is the boundary. `A` can be a string, an enum, a struct — anything. Core never inspects it. Whether `"allow"` means permit and `"deny"` means block is your concern, not the engine's.

Policy (allow/deny, rate limits, routing decisions) lives **above** the matcher. This is the Istio pattern — the data plane matches, the control plane decides. x.uma is the data plane.

This means x.uma doesn't compete with OPA or Cedar. It complements them. Use x.uma for fast, structured matching. Use a policy engine for policy logic that operates on the match result.

## Two Construction Paths

Matchers can be built two ways:

**Compiler path** — domain-specific DSL produces matchers directly. Ergonomic, type-safe, no serialization overhead.

```python
from xuma.http import HttpRouteMatch, compile_route_matches

routes = [HttpRouteMatch(path=HttpPathMatch(type="PathPrefix", value="/api"), method="GET")]
matcher = compile_route_matches(routes, "api", "not_found")
```

**Config path** — JSON/YAML config loaded through the registry. Portable across languages, storable, versionable.

```json
{
  "matcher_list": [{
    "predicate": {
      "single": {
        "input": { "type_url": "xuma.test.v1.StringInput", "config": { "key": "method" } },
        "matcher": { "type_url": "xuma.core.v1.StringMatcher", "config": { "exact": "GET" } }
      }
    },
    "on_match": { "action": "route-get" }
  }],
  "on_no_match": { "action": "fallback" }
}
```

Both paths produce the same `Matcher`. The compiler path is for programmatic construction. The config path is for declarative, cross-language use.

## Five Implementations, One Spec

| Implementation | Language | Type |
|----------------|----------|------|
| **rumi** | Rust | Reference implementation |
| **xuma** (Python) | Python | Pure Python |
| **xuma** (TypeScript) | TypeScript | Pure TypeScript |
| **puma-crusty** | Python | Rust core via PyO3 |
| **bumi-crusty** | TypeScript | Rust core via WASM |

All five pass the same conformance test suite. Same config format, same evaluation semantics, same results. Choose based on your runtime and performance needs.

## Next

- [The Matching Pipeline](../concepts/pipeline.md) — how data flows through evaluation
- [Type Erasure and Ports](../concepts/type-erasure.md) — the technical details of the seam
- [When to Use x.uma](when-to-use.md) — where x.uma fits and where it doesn't
