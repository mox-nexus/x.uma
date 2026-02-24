# When to Use x.uma

x.uma is a matcher engine. It evaluates structured data against rules and returns the first matching action. This page helps you decide whether it fits your problem.

## x.uma Is For

**Structured matching with known fields.** You have data with named fields (HTTP method, path, headers; tool names, arguments; event types) and need to route, filter, or classify based on combinations of those fields.

**Cross-language consistency.** You need the same matching rules to produce the same results in Rust, Python, and TypeScript. One config format, five implementations, identical semantics.

**Config-driven matching.** Your matching rules come from configuration files (JSON, YAML, protobuf) rather than hardcoded logic. Rules change without redeployment.

**First-match-wins routing.** Your problem is "which rule matches first?" — not "what are all the rules that match?" or "what is the aggregate policy across all rules?"

**Safety-critical matching.** You need guarantees: linear-time regex (no ReDoS), depth limits (no stack overflow), immutable matchers (no race conditions), fail-closed on missing data.

## x.uma Is Not For

**General-purpose policy evaluation.** If you need attribute-based access control with complex policy logic (role hierarchies, contextual permissions, deny-overrides), use a policy engine like OPA or Cedar. x.uma finds matches — it doesn't evaluate policies.

**Free-text search.** x.uma matches structured fields against patterns. If you need full-text search, fuzzy matching, or semantic similarity, use a search engine.

**Stateful decisions.** x.uma matchers are pure functions — same input always produces same output. If your decision depends on previous requests, rate counters, or session state, you need stateful middleware.

**Dynamic rule updates at runtime.** x.uma registries and matchers are immutable after construction. If you need to add or remove rules during execution without reconstruction, x.uma's model doesn't fit.

## Comparison

| Need | x.uma | Alternative |
|------|-------|-------------|
| HTTP route matching | Yes — built-in domain | nginx, Envoy, framework routers |
| Tool/hook gating | Yes — Claude Code domain | Custom if/else chains |
| Config-driven matching | Yes — JSON/YAML/proto config | Hand-rolled config parsers |
| ABAC / RBAC policies | **No** — use a policy engine | OPA (Rego), Cedar, Zanzibar |
| Complex authorization | **No** — matcher, not policy engine | OPA, Casbin, custom logic |
| Full-text search | **No** — structured fields only | Elasticsearch, MeiliSearch |
| Rate limiting | **No** — stateless matching | Redis, middleware |

## x.uma + Policy Engines

x.uma and policy engines solve different problems. They compose well:

```text
Request → x.uma (structured matching) → action
                                            ↓
                              Policy engine (authorization) → permit/deny
```

x.uma handles the fast path: "which rule matches this request?" The policy engine handles the complex path: "given this match, is the action authorized?" This is the Istio pattern — data plane matches, control plane decides.

## When Performance Matters

x.uma evaluation is fast — 9-33ns for exact matches depending on implementation. But the performance advantage over hand-written `if/else` chains only matters when:

1. **Rules come from config** — you can't hardcode what you don't know at compile time
2. **Rules change** — reconstruction is cheaper than redeployment
3. **Cross-language parity** — you need the same rules in multiple runtimes
4. **Safety guarantees** — ReDoS protection and depth limits matter

If you have five static rules that never change and only run in one language, `if/else` is simpler. x.uma pays off when the rule set grows, changes, or crosses language boundaries.

## Choosing an Implementation

| Choose | When |
|--------|------|
| **rumi** (Rust) | Maximum performance, Rust codebase, embedding in other systems |
| **xuma** (Python) | Python codebase, prototyping, Django/Flask/FastAPI integration |
| **xuma** (TypeScript) | TypeScript codebase, Bun runtime, edge functions |
| **puma-crusty** (Python+Rust) | Python codebase needing Rust performance (especially regex) |
| **bumi-crusty** (TypeScript+WASM) | TypeScript needing Rust performance in the browser or edge |

Pure implementations (rumi, xuma Python, xuma TypeScript) are self-contained with no native dependencies beyond RE2. Crusty variants wrap the Rust core through FFI — same API, Rust performance.

## Next

- [Architecture](architecture.md) — the design behind the engine
- [Benchmarks](../performance/benchmarks.md) — concrete performance numbers
- [Security Model](../performance/security.md) — safety guarantees in depth
