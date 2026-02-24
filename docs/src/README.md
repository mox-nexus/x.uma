# x.uma

> One matcher engine. Five implementations. Same semantics everywhere.

x.uma implements the [xDS Unified Matcher API](https://www.envoyproxy.io/docs/envoy/latest/api-v3/xds/type/matcher/v3/matcher.proto) — the same matching protocol Envoy uses at Google scale — across Rust, Python, and TypeScript.

Write matching rules once. Evaluate them in any language. Get the same answer every time.

```
Context → DataInput → MatchingData → InputMatcher → bool
           domain-      erased         domain-
           specific                    agnostic
```

An `ExactMatcher` doesn't know whether it's matching HTTP paths, Claude Code hook events, or your custom domain. It matches *data*. The domain-specific part — extracting that data from your context — is a separate port. This is the key architectural insight, borrowed from Envoy and proven at Google scale.

## Implementations

| Package | Language | What it is |
|---------|----------|------------|
| **rumi** | Rust | Core engine (reference implementation) |
| **xuma** | Python 3.12+ | Pure Python, zero native deps beyond RE2 |
| **xuma** | TypeScript/Bun | Pure TypeScript, zero native deps beyond RE2 |
| **puma-crusty** | Python | Rust bindings via PyO3 |
| **bumi-crusty** | TypeScript | Rust bindings via WASM |

All five pass the same conformance test suite (~958 tests total). They implement identical semantics with different performance characteristics.

## Pick Your Language

Already know which language you need? Start here:

- **[Rust](getting-started/rust.md)** — `rumi` + `rumi-http` in your `Cargo.toml`
- **[Python](getting-started/python.md)** — `uv add xuma`, build a matcher in 10 lines
- **[TypeScript](getting-started/typescript.md)** — `bun add xuma`, same API shape as Python

Each quick start gets you from install to working HTTP route matcher in under 5 minutes.

## Understand First

Not sure whether x.uma fits your problem? Read these:

- **[When to Use x.uma](explain/when-to-use.md)** — x.uma vs OPA vs Cedar vs Zanzibar. Honest comparison with decision framework.
- **[Architecture](explain/architecture.md)** — Hexagonal architecture, ACES design philosophy, why five implementations.

## Domains

x.uma ships with two domain adapters. Adding your own is [straightforward](guides/adding-domain.md).

**[HTTP Matching](domains/http.md)** — Path, method, header, and query parameter matching. Gateway API config types. Compiles `HttpRouteMatch` into a `Matcher` in one call.

**[Claude Code Hooks](domains/claude.md)** — Match Claude Code hook events by event type, tool name, arguments, session, working directory, or git branch. Compiles `HookMatch` into a `Matcher` for multi-rule OR semantics.

## What It Guarantees

| Guarantee | How |
|-----------|-----|
| **No ReDoS** | Rust `regex` crate (linear time). Python uses `google-re2`. TypeScript uses `re2js`. |
| **Bounded depth** | Max 32 levels of nesting, validated at config load |
| **Fail-closed** | Missing data means predicate returns `false`. Never matches by accident. |
| **Thread-safe** | All types are `Send + Sync` (Rust) / immutable (Python, TypeScript) |
| **Config validated at construction** | Invalid configs rejected before evaluation. Parse, don't validate. |

## Status

**Version 0.0.2** — Alpha. API is stabilizing but may change before 1.0.

Phases 0-18 complete. Core engine, HTTP domain, Claude domain, config/registry layer, RE2 migration, CLI, cross-language benchmarks — all shipped. Name resolution pending before crates.io/PyPI/npm publish.
