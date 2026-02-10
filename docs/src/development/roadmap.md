# Roadmap

Development phases and current status.

## Current Status

**x.uma is alpha software.** The API is under active development and will change.

Five implementations exist:
- **rumi** (Rust) — reference implementation, 195 tests
- **puma** (Python) — pure Python, zero dependencies, 194 tests
- **bumi** (TypeScript) — pure TypeScript, zero runtime deps, 202 tests
- **puma-crusty** (Python + Rust) — PyO3 bindings, 37 tests
- **bumi-crusty** (TypeScript + WASM) — wasm-bindgen bindings, 36 tests

All pass the same conformance test suite (`spec/tests/`). Total: 268 tests across 5 variants.

## Phase Overview

| Phase | Focus | Status |
|-------|-------|--------|
| 0 | Scaffolding | ✅ Done |
| 1 | Core Traits | ✅ Done |
| 2 | Conformance Fixtures | ✅ Done |
| 2.5 | Extensible MatchingData (`Custom` variant) | ✅ Done |
| 3 | StringMatcher, MatcherTree, RadixTree | ✅ Done |
| 4 | HTTP Domain (ext_proc model) | ✅ Done |
| 5 | puma (Pure Python + HTTP) | ✅ Done |
| 5.1 | puma arch-guild hardening | ✅ Done |
| 6 | bumi (Bun/TypeScript + HTTP) | ✅ Done |
| 6.1 | bumi arch-guild hardening | ✅ Done |
| 7 | puma-crusty: PyO3 Python bindings | ✅ Done |
| 7.5 | rumi-claude: trace + HookMatch compiler | ✅ Done |
| 8 | bumi-crusty: wasm-bindgen TypeScript bindings | ✅ Done |
| 9 | Cross-language benchmarks (all 5 variants) | 🚧 In Progress |
| 10 | Semantic matching (cosine similarity) | Planned |
| 11 | RE2 migration: `google-re2` for puma, `re2js` for bumi | Planned |

## Phase 11: RE2 Linear-Time Regex Migration

**Status:** Planned

Replace backtracking regex engines in puma and bumi with linear-time RE2 implementations.

**Motivation:** Phase 9 benchmarks proved ReDoS is catastrophic — Python `re` at N=20 takes 72ms, JS `RegExp` takes 11ms, while Rust `regex` (RE2 semantics) takes 11ns. The crusty bindings solve this but add FFI overhead. RE2 packages give linear-time regex natively without crossing a language boundary.

**puma:** [`google-re2`](https://pypi.org/project/google-re2/) — official Google C++ wrapper
- Drop-in for `RegexMatcher` internals (swap `re.search` → `re2.search`)
- Requires C++ toolchain to build (wheel available for common platforms)
- Trade-off: adds a native dependency to previously zero-dep puma

**bumi:** [`re2js`](https://github.com/le0pard/re2js) — pure JavaScript port of RE2
- Zero native deps, zero WASM — pure JS with linear-time guarantee
- Drop-in for `RegexMatcher` internals (swap `RegExp.test` → `re2js` equivalent)
- Best of both worlds: bumi's 9ns JIT speed + linear-time safety

**Limitations (both):** No backreferences, no lookahead assertions. Same as Rust `regex` crate. The vast majority of matcher patterns don't need these.

**Crusty still matters:** RE2 only fixes the regex leaf node. Crusty replaces the entire evaluation pipeline (tree walk, predicate composition, field matching) in Rust. For complex configs with 100+ rules, crusty's compiled pipeline wins. RE2 + crusty is the best of all worlds.

## Phase 5: puma (Pure Python)

**Status:** Complete (v0.1.0)

Pure Python implementation of the xDS Unified Matcher API. Zero dependencies. Python 3.12+.

**Architecture:**
- PEP 695 type params (`class Foo[T]:`) — modern Python generics
- `@dataclass(frozen=True, slots=True)` — immutability + performance
- Protocol-based ports (runtime-checkable)
- Union type aliases (`type Predicate = Single | And | Or | Not`)

**Type System Mapping:**

| Rust (rumi) | Python (puma) | Notes |
|-------------|---------------|-------|
| `trait DataInput<Ctx>` | `Protocol[Ctx]` | Contravariant in `Ctx` |
| `trait InputMatcher` | `Protocol` | Non-generic, runtime-checkable |
| `enum MatchingData` | `MatchingData = str \| int \| bool \| bytes \| None` | Type alias replaces enum |
| `enum Predicate<Ctx>` | `type Predicate[Ctx] = Single \| And \| Or \| Not` | Pattern-matchable |
| `enum OnMatch<Ctx, A>` | `type OnMatch[Ctx, A] = Action \| NestedMatcher` | xDS exclusivity |

**Deliverables:**
- [x] Core types (`Matcher`, `Predicate`, `FieldMatcher`, `OnMatch`)
- [x] String matchers (`Exact`, `Prefix`, `Suffix`, `Contains`, `Regex`)
- [x] HTTP domain (`HttpRequest`, `PathInput`, `MethodInput`, `HeaderInput`, `QueryParamInput`)
- [x] Gateway API compiler (`HttpRouteMatch` → `Matcher`)
- [x] 194 conformance tests passing (0.10s)
- [x] `py.typed` marker for type checker support
- [x] Auto-validation (depth limit enforced at construction)
- [x] Typed Gateway API (Literal types, proper return annotations)
- [x] Security documentation (ReDoS contract)

**Known Limitations:**
- `RegexMatcher` uses Python `re` (backtracking, ReDoS-vulnerable)
- For adversarial input, use `puma-crusty` (Phase 7) with Rust-backed linear-time regex
- No proto codegen (pure Python types only)

## Phase 5.1: Arch-Guild Hardening

**Status:** Complete

13-agent architecture review identified 4 gaps, all resolved:

1. **py.typed marker** — Type checkers now recognize puma as typed
2. **Auto-validation** — `Matcher.__post_init__` calls `validate()`, depth limits enforced
3. **Typed Gateway API** — Literal types for match types, proper return annotations
4. **ReDoS documentation** — SECURITY.md explains Python `re` vs Rust `regex` trade-off

**Verdict:** Architecture excellent (zero boundary violations, hexagonal textbook). Safety gap closed via strategic documentation + puma-crusty path for adversarial use cases.

## Phase 6: bumi (Bun/TypeScript)

**Status:** Complete (v0.1.0)

Pure TypeScript implementation using Bun runtime. Zero runtime dependencies. 202 tests passing.

**Architecture:**
- Classes with `readonly` props for immutability
- `instanceof` checks for union discrimination
- Biome for lint/format, `bun test` for testing
- `Object.create(null)` for params/headers (prototype pollution protection)

**Type System Mapping (Rust → TypeScript):**

| Rust (rumi) | TypeScript (bumi) | Notes |
|-------------|-------------------|-------|
| `trait DataInput<Ctx>` | `interface DataInput<Ctx>` | Generic interface |
| `trait InputMatcher` | `interface InputMatcher` | Non-generic |
| `enum MatchingData` | `type MatchingData = string \| number \| boolean \| Uint8Array \| null` | Union type |
| `enum Predicate<Ctx>` | `type Predicate<Ctx> = Single<Ctx> \| And<Ctx> \| Or<Ctx> \| Not<Ctx>` | Discriminated union |
| `enum OnMatch<Ctx, A>` | `type OnMatch<Ctx, A> = Action<A> \| NestedMatcher<Ctx, A>` | Discriminated union |

## Phase 7: puma-crusty (PyO3 Python Bindings)

**Status:** Complete

Rust-backed Python package via PyO3, providing opaque compiled `HookMatcher` for Claude Code hooks.

**Architecture:**
- PyO3 0.25+ for Python 3.14 support
- `#[pyclass(frozen)]` for immutable compiled matchers
- Opaque engine pattern: config in → compile in Rust → evaluate in Rust → simple types out
- `maturin develop` for local builds, `maturin build` for wheels
- Linear-time regex (Rust `regex` crate — ReDoS immune)

**Key APIs:**
- `HookMatcher.compile(rules, action, fallback)` → compiled matcher
- `matcher.evaluate(**context)` → action string or None
- `matcher.trace(**context)` → detailed trace for debugging

## Phase 8: bumi-crusty (wasm-bindgen TypeScript Bindings)

**Status:** Complete

Rust-backed TypeScript package via wasm-bindgen + wasm-pack.

**Architecture:**
- Config types use plain JS objects (discriminated unions), not opaque Rust structs
- `StringMatch` is a zero-size namespace struct with static factory methods
- `serde-wasm-bindgen` for trace output serialization (Rust → JS with camelCase)
- Same security hardening as puma-crusty (fail-closed, input limits, validate after compile)

## Phase 9: Cross-Language Benchmarks

**Status:** In Progress

Performance comparison across all 5 x.uma variants using language-native tools.

**Tooling:**

| Language | Tool |
|----------|------|
| Rust | divan (`#[divan::bench]` attribute API) |
| Python | pytest-benchmark |
| TypeScript | mitata |

**Benchmark Categories:**
- **Compile latency** — config → matcher construction
- **Evaluate throughput** — matcher + context → action (hot path)
- **FFI overhead** — pure vs crusty head-to-head comparison
- **ReDoS demonstration** — `(a+)+$` pattern, linear (Rust) vs exponential (Python/TS)
- **Scaling** — 1 to 200 rules, miss-heavy workloads, trace overhead

## Historical: Phase 4 (HTTP Domain)

Phase 4 delivered the HTTP domain for rumi (Rust). Two-layer architecture:

**User API:** Gateway API `HttpRouteMatch` (config-time, YAML/JSON)
**Data Plane API:** Envoy `ext_proc ProcessingRequest` (runtime)

```text
Gateway API HttpRouteMatch (config)
        ↓ compile()
rumi Matcher<ProcessingRequest, A>
        ↓ evaluate()
ext_proc ProcessingRequest (runtime)
```

**Why Two Layers?**
- Gateway API is the CNCF standard (Istio, Envoy Gateway, Contour, Kong)
- ext_proc is Envoy's universal HTTP processing model (REST, gRPC, GraphQL)
- x.uma provides the match vocabulary, actions are domain-specific

**Deliverables:**
- rumi-http crate with `HttpMessage` indexed context
- DataInputs: `PathInput`, `MethodInput`, `HeaderInput`, `QueryParamInput`, `SchemeInput`, `AuthorityInput`
- Gateway API types via `k8s-gateway-api` crate
- Compiler: `HttpRouteMatch::compile()` → `Matcher<HttpMessage, A>`
- Integration tests with ext_proc fixtures

## Contributing

See the [GitHub repository](https://github.com/mox-labs/x.uma) for contribution guidelines.
