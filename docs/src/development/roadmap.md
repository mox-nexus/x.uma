# Roadmap

Development phases and current status.

## Current Status

**x.uma is alpha software.** The API is under active development and will change.

Two implementations exist:
- **rumi** (Rust) — reference implementation, production-hardened constraints
- **puma** (Python) — pure Python port, zero dependencies, 194 tests passing

Both pass the same conformance test suite (`spec/tests/`). TypeScript (bumi) is next.

## Phase Overview

| Phase | Focus | Status |
|-------|-------|--------|
| 0 | Scaffolding | ✅ Done |
| 1 | Core Traits | ✅ Done |
| 2 | Conformance Fixtures | ✅ Done |
| 2.5 | Extensible MatchingData (`Custom` variant) | ✅ Done |
| 3 | StringMatcher, MatcherTree, RadixTree | ✅ Done |
| 4 | HTTP Domain (ext_proc model) | ✅ Done |
| 5 | p.uma (Pure Python + HTTP) | ✅ Done |
| 5.1 | p.uma arch-guild hardening | ✅ Done |
| 6 | b.uma (Bun/TypeScript + HTTP) | 🚧 Next |
| 7 | rumi/crusts/python (uniffi→puma-crusty) | Planned |
| 8 | rumi/crusts/wasm (wasm-pack→@x.uma/bumi-crusty) | Planned |
| 9 | Benchmarks (all variants) | Planned |

## Phase 5: p.uma (Pure Python)

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
| `enum MatchingData` | `type MatchingValue = str \| int \| bool \| bytes \| None` | Union replaces enum |
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

## Phase 6: b.uma (Bun/TypeScript)

**Status:** Next

Pure TypeScript implementation using Bun runtime.

**Planned:**
- Modern TypeScript with discriminated unions
- Bun-native testing (`bun test`)
- Zero runtime dependencies (no axios, no lodash)
- Same conformance test suite
- HTTP domain with Gateway API compiler

**Type System Mapping (Rust → TypeScript):**

| Rust (rumi) | TypeScript (bumi) | Notes |
|-------------|-------------------|-------|
| `trait DataInput<Ctx>` | `interface DataInput<Ctx>` | Generic interface |
| `trait InputMatcher` | `interface InputMatcher` | Non-generic |
| `enum MatchingData` | `type MatchingValue = string \| number \| boolean \| Uint8Array \| null` | Union type |
| `enum Predicate<Ctx>` | `type Predicate<Ctx> = Single<Ctx> \| And<Ctx> \| Or<Ctx> \| Not<Ctx>` | Discriminated union |
| `enum OnMatch<Ctx, A>` | `type OnMatch<Ctx, A> = Action<A> \| NestedMatcher<Ctx, A>` | Discriminated union |

## Phase 7: puma-crusty (Rust Bindings → Python)

**Status:** Planned

Rust-backed Python package providing the same puma API with Rust performance + safety.

**Approach:**
- uniffi bindings from `rumi/crusts/python/`
- maturin for wheel building
- Same API as pure puma (drop-in replacement)
- Linear-time regex (ReDoS-safe)

**Trade-off:** Adds compiled Rust binary dependency. Pure puma remains zero-dependency option.

## Phase 8: bumi-crusty (Rust Bindings → WASM)

**Status:** Planned

Rust-backed TypeScript package via WASM.

**Approach:**
- wasm-bindgen from `rumi/crusts/wasm/`
- wasm-pack for bundling
- Same API as pure bumi (drop-in replacement)
- Linear-time regex (ReDoS-safe)

## Phase 9: Benchmarks

**Status:** Planned

Cross-language performance comparison.

**Variants to benchmark:**
- rumi (Rust native)
- puma (pure Python)
- puma-crusty (Python → Rust via uniffi)
- bumi (pure TypeScript)
- bumi-crusty (TypeScript → Rust via WASM)

**Metrics:**
- Matcher compilation time
- Evaluation throughput (matches/sec)
- Memory usage
- Startup overhead (FFI/WASM)

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
