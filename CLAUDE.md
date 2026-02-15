# x.uma — Cross-Platform Matcher Engine

## What is x.uma?

A matcher engine implementing the xDS Unified Matcher API across multiple languages and domains.

| Package | Language | Notes |
|---------|----------|-------|
| **rumi** | Rust | Core engine (reference implementation) |
| **puma** | Python | Pure Python implementation (dir: `puma/`) |
| **bumi** | Bun/TypeScript | Pure TypeScript implementation (dir: `bumi/`) |
| **puma-crusty** | Python | Rust bindings via PyO3 (from `rumi/crusts/python/`) |
| **bumi-crusty** | TypeScript | Rust bindings via wasm-bindgen (from `rumi/crusts/wasm/`) |

All implementations pass the same conformance test suite (`spec/tests/`).

## Design Philosophy: ACES

**A**daptable · **C**omposable · **E**xtensible · **S**oftware

x.uma follows ACES principles using hexagonal architecture (ports & adapters) to achieve **sustainable excellence** — no rewrites needed.

### Architecture

```
                    ┌─────────────────────────────────┐
                    │         Domain Adapters         │
                    │ xuma.http xuma.claude xuma.grpc │
                    └───────────────┬─────────────────┘
                                    │
                    ┌───────────────▼─────────────────┐
                    │            PORTS                │
                    │   InputPort       ActionPort    │
                    │  (extract data)  (emit result)  │
                    └───────────────┬─────────────────┘
                                    │
                    ┌───────────────▼─────────────────┐
                    │            CORE                 │
                    │         rumi engine            │
                    │   Matcher · Predicate · Tree    │
                    │      (pure, domain-agnostic)    │
                    └─────────────────────────────────┘
```

### ACES Properties

| Property | Implementation |
|----------|----------------|
| **Adaptable** | New domains plug in without touching core |
| **Composable** | Matchers nest, predicates AND/OR/NOT, trees recurse |
| **Extensible** | `TypedExtensionConfig` is the extension seam |
| **Sustainable** | Core is stable; growth happens at edges |

### The Seam

`TypedExtensionConfig` from xDS is the architectural seam:

```protobuf
message TypedExtensionConfig {
  string name = 1;                       // adapter identifier
  google.protobuf.Any typed_config = 2;  // adapter config
}
```

Every `input` and `action` is a port. Adapters are concrete registered types.

### Extension Namespace: `xuma`

All x.uma extensions use the `xuma` proto package namespace:

```
xuma.core.v1      # Base types, registry
xuma.test.v1      # Conformance testing
xuma.http.v1      # HTTP matching
xuma.claude.v1    # Claude Code hooks
xuma.grpc.v1      # gRPC matching
```

Type URLs:
- `type.googleapis.com/xuma.test.v1.StringInput`
- `type.googleapis.com/xuma.http.v1.HeaderInput`
- `type.googleapis.com/xuma.claude.v1.HookContext`

## Project Structure

```
x.uma/
├── proto/
│   ├── xds/                    # upstream (buf dep)
│   └── xuma/                   # x.uma extensions (namespace: xuma.*)
├── spec/
│   └── tests/                  # conformance test fixtures (YAML)
├── rumi/                       # Rust workspace (core + extensions + crusts + proto)
│   └── proto/src/gen/          # buf-generated Rust types (prost + prost-serde)
├── puma/                       # Pure Python implementation (package: puma)
│   └── proto/src/gen/          # buf-generated Python types (betterproto)
├── bumi/                       # Pure TypeScript implementation (package: bumi)
│   └── proto/src/gen/          # buf-generated TypeScript types (ts-proto)
├── buf.gen.yaml                # Polyglot codegen config (all 3 languages)
├── docs/                       # mdBook documentation
└── justfile                    # polyglot task orchestration
```

## Roadmap

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
| 9 | Cross-language benchmarks (all 5 variants) | ✅ Done |
| 10 | TypedExtensionConfig Registry (`IntoDataInput`, `RegistryBuilder`) | ✅ Done |
| 11 | Test audit (removed 18 ineffective tests → 216 total) | ✅ Done |
| 12 | Proto Alignment: buf codegen, `rumi-proto`, `AnyResolver`, xDS Matcher loading | ✅ Done |
| 13 | Config/Registry across all implementations | 🚧 In Progress |
| — | Semantic matching (cosine similarity via `CustomMatchData`) | Planned |
| — | RE2 migration: `google-re2` for puma, `re2js` for bumi | Planned |

## Current Work

**Phase 13: Config/Registry Across All Implementations**

Bringing config-driven matcher construction to all 5 implementations. Same JSON config → working matchers everywhere.

- **Plan**: `scratch/phase-13-plan.md` (sub-phases 13.0–13.5, guild findings, design decisions)
- **Progress**: `scratch/phase-13-progress.md` (status tracker, dependency graph, completion criteria)
- **Guild report**: `scratch/arch-guild-reports/phase-13/`

Implementation order: rumi core hardening (13.0) → config fixtures (13.1) → puma/bumi/crusty ports (13.2–13.5 parallel).

## Tooling

| Concern | Tool |
|---------|------|
| Proto codegen | buf.build |
| Rust | Cargo workspace |
| Python | uv + maturin |
| WASM (optional) | wasm-pack (build target of rumi) |
| Task orchestration | just |
| Conformance tests | YAML fixtures, native runners |

## Reference Implementations

| Implementation | Language | Role |
|----------------|----------|------|
| Envoy | C++ | Original, production-proven |
| rumi | Rust | Our reference |

Envoy source: `~/oss/envoy/source/common/matcher/`

## rumi Type System (Envoy-Inspired)

**Key insight from spike**: Type erasure at the **data level**, not the predicate level.

```rust
// MatchingData — the erased data type (Envoy's MatchingDataType)
pub enum MatchingData { None, String(String), Int(i64), Bool(bool), Bytes(Vec<u8>) }

// DataInput — domain-specific, generic over context, returns erased type
pub trait DataInput<Ctx>: Send + Sync + Debug {
    fn get(&self, ctx: &Ctx) -> MatchingData;
}

// InputMatcher — domain-agnostic, NON-GENERIC, shareable across contexts!
pub trait InputMatcher: Send + Sync + Debug {
    fn matches(&self, value: &MatchingData) -> bool;
}

// SinglePredicate — where domain-specific meets domain-agnostic
pub struct SinglePredicate<Ctx> {
    input: Box<dyn DataInput<Ctx>>,
    matcher: Box<dyn InputMatcher>,
}
```

**Why this works:**
- `InputMatcher` is non-generic → same `ExactMatcher` works for HTTP, Claude, test contexts
- No GATs or complex lifetimes needed
- Battle-tested at Google scale (Envoy uses this approach)

## xDS Proto Semantics (Critical)

From official Envoy xDS proto research:

| Concept | xDS Semantics | rumi Implementation |
|---------|---------------|---------------------|
| **OnMatch exclusivity** | `oneof { Matcher matcher = 1; Action action = 2; }` | `enum OnMatch<Ctx, A> { Action(A), Matcher(Box<Matcher>) }` |
| **Nested matcher failure** | If nested matcher returns no-match, parent OnMatch fails | Continue to next field_matcher (no fallback) |
| **on_no_match** | At Matcher level only, not per-OnMatch | `Matcher.on_no_match: Option<OnMatch>` |
| **First-match-wins** | `keep_matching: true` records action but returns no-match | INV enforced in Matcher::evaluate() |

**Key insight**: OnMatch is EXCLUSIVE — action XOR nested matcher, never both. Making illegal states unrepresentable at the type level.

## Arch-Guild Constraints (Mandatory)

From 13-agent architecture review:

| Constraint | Source | Rationale |
|------------|--------|-----------|
| **ReDoS Protection** | Vector, Taleb | Use Rust `regex` crate only (linear time). No `fancy-regex`. |
| **Depth Limits** | Vector, Taleb | Max 32 levels for nested matchers. Validate at config load. |
| **Type Registry Immutability** | Vector, Lamport | Lock after initialization. No runtime registration. |
| **Send + Sync** | Lamport, Lotfi | All core types must be thread-safe (FFI requirement). |
| **Iterative Evaluation** | Taleb, Dijkstra | No recursive `evaluate()` — use explicit stack (deferred to v0.2). |
| **DataInput None → false** | Dijkstra | `None` from `DataInput::get()` → predicate evaluates to `false`. |
| **No unsafe impl** | Wolf | Let compiler derive Send/Sync — don't add restrictive bounds. |
| **Validate extension points at construction** | Phase 11 review | Extension hooks (e.g., `data_type()`, `supported_types()`) that exist but are never enforced create silent failure modes. Validate compatibility at construction/load time, not evaluation time. |

## Arch-Guild Decision: Matcher Engine, Not Policy Engine

From 8-agent deliberation (2026-02-08). **Verdict: DO NOT expand scope.**

The generic `A` in `Matcher<Ctx, A>` is the fence — core does not know about allow/deny.
Policy lives ABOVE the matcher (Istio pattern), not inside it.

| Rule | Rationale |
|------|-----------|
| **No "Policy" type in core** | The `A` parameter is the composition seam. Core doesn't interpret actions. |
| **Use "matcher engine" in docs** | Not "policy engine". Align vocabulary with what the code actually does. |
| **`NamedMatcher` over `Policy`** | If naming metadata is ever needed, use truthful names (Karman). |
| **Domain compilers own the vocabulary** | rumi-http has `HttpRouteMatch`, rumi-claude has `HookMatch`. |
| **Cross-domain = pipeline** | Different `Ctx` types are incomparable. Combine actions, not contexts. |

**Strategic path:** Build domain compilers now. Extract policy abstraction only when a second integration reveals cross-domain pain. See `scratch/arch-guild-reports/policy-deliberation/00-index.md`.

## Domain Compiler Pattern

Each domain adapter provides a **compiler** that transforms user-friendly config into matcher trees:

| Domain | Config Type | Compiler | Output |
|--------|------------|----------|--------|
| HTTP | `HttpRouteMatch` | `compile_route_matches()` | `Matcher<HttpMessage, A>` |
| Claude | `HookMatch` | `compile_hook_matches()` | `Matcher<HookContext, A>` |

The compiler is the "door handle" — it makes the matcher engine usable without manual tree construction.

### Claude Domain Compiler (rumi-claude)

Types to build (parallel to rumi-http's gateway):
- `HookMatch` — match conditions for Claude Code hook events
- `HookMatchExt` — extension trait for compile convenience
- `compile_hook_matches()` — transforms `HookMatch` configs into `Matcher<HookContext, A>`
- `evaluate_with_trace()` — returns the decision AND the path through the matcher tree

### Cross-Language Type Mapping

| Concept | Rust (rumi) | Python (puma) | TypeScript (bumi) |
|---------|-------------|---------------|-------------------|
| Erased data | `MatchingData` | `MatchingData` | `MatchingData` |
| Context type | `Ctx` (generic) | `Ctx` (TypeVar) | `Ctx` (generic) |
| Action type | `A` (generic) | `A` (TypeVar) | `A` (generic) |

`MatchingData` is the same name across all three implementations. In Rust it's an enum, in Python a type alias (`str | int | bool | bytes | None`), in TypeScript a type alias (`string | number | boolean | Uint8Array | null`). One concept, one name.

## rumi Crate Structure

Workspace with core + extension crates:

```
rumi/
├── Cargo.toml          # Workspace manifest
├── core/               # Core engine (package: rumi)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── matcher.rs, predicate.rs, ...
├── proto/              # Proto-generated types + conversion (package: rumi-proto)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # Module tree for generated types
│       ├── any_resolver.rs     # google.protobuf.Any → TypedConfig bridge
│       ├── convert.rs          # Proto Matcher → MatcherConfig conversion
│       └── gen/                # buf-generated prost + prost-serde code
├── ext/
│   ├── test/           # rumi-test (conformance)
│   ├── http/           # rumi-http (HTTP matching)
│   └── claude/         # rumi-claude (Claude Code hooks)
└── crusts/             # Language bindings (🦀 crustacean → crusty)
    ├── python/         # PyO3 → puma-crusty wheel (maturin)
    └── wasm/           # wasm-bindgen → bumi-crusty (wasm-pack)
```

**Extension pattern:** Users depend on an extension crate, get core transitively:

```toml
[dependencies]
rumi-http = "0.1"  # rumi comes transitively
```

```rust
use rumi::prelude::*;
use rumi_http::{HttpRequest, HeaderInput};
```

## Working Conventions

### Scratch Directory

`scratch/` is for session notes, research synthesis, and working documents.

### Conformance Tests

All implementations must pass all fixtures in `spec/tests/`. The fixture suite is the source of truth for correctness.

### Session Start

On new session, read `scratch/next-session.md` and confirm understanding with user before proceeding.

### Development Workflow

1. **Write fixture first** (conformance-driven development)
2. **Implement to pass fixture**
3. **Benchmark** (catch regressions early)
4. Use `just build`, `just test`, `just lint` for common tasks

### Code Quality Principles

1. **Always fix, never skip** — when lints/checks fail, fix immediately. Don't ask whether to skip.
2. **clippy --fix then fmt** — always run both in sequence before committing:
   ```bash
   cargo clippy --fix --allow-dirty --manifest-path rumi/Cargo.toml --workspace -- -W clippy::pedantic
   cargo fmt --manifest-path rumi/Cargo.toml --all
   ```
3. **Pre-commit auto-fixes** — if the hook fails, it auto-fixes and you re-stage + commit again.
