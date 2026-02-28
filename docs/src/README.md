# x.uma

> **Alpha (0.0.2)** — API is stabilizing. Expect breaking changes before 1.0.

One matcher engine. Five implementations. Same semantics everywhere.

x.uma implements the [xDS Unified Matcher API](https://www.envoyproxy.io/docs/envoy/latest/api-v3/xds/type/matcher/v3/matcher.proto) — the same matching protocol Envoy uses at Google scale — across Rust, Python, and TypeScript.

Write matching rules once. Evaluate them in any language. Get the same answer every time.

```
Context → DataInput → MatchingData → InputMatcher → bool
           domain-      erased         domain-
           specific                    agnostic
```

An `ExactMatcher` doesn't know whether it's matching HTTP paths, Claude Code hook events, or your custom domain. It matches *data*. The domain-specific part — extracting that data from your context — is a separate port.

## Implementations

| Package | Language | What it is |
|---------|----------|------------|
| **rumi** | Rust | Core engine (reference implementation) |
| **xuma** | Python 3.12+ | Pure Python, zero native deps beyond RE2 |
| **xuma** | TypeScript/Bun | Pure TypeScript, zero native deps beyond RE2 |
| **xuma-crust** | Python | Rust bindings via PyO3 |
| **xuma-crust** | TypeScript | Rust bindings via WASM |

All five pass the same conformance test suite (~958 tests total).

## Get Started

- **[Rust](getting-started/rust.md)** — `rumi` + `rumi-http` in your `Cargo.toml`
- **[Python](getting-started/python.md)** — `uv add xuma`, build a matcher in 10 lines
- **[TypeScript](getting-started/typescript.md)** — `bun add xuma`, same API shape as Python

## Guarantees

| Guarantee | How |
|-----------|-----|
| **No ReDoS** | Rust `regex` crate (linear time). Python uses `google-re2`. TypeScript uses `re2js`. |
| **Bounded depth** | Max 32 levels of nesting, validated at config load |
| **Fail-closed** | Missing data → predicate returns `false`. Never matches by accident. |
| **Thread-safe** | All types are `Send + Sync` (Rust) / immutable (Python, TypeScript) |
