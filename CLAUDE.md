# x.uma — Cross-Platform Unified Matcher API

## What is x.uma?

A matcher ecosystem implementing the xDS Unified Matcher API across multiple languages and domains.

| Package | Language | Notes |
|---------|----------|-------|
| **rumi** | Rust | Core engine (reference implementation) |
| **p.uma** | Python | Pure Python implementation |
| **j.uma** | TypeScript | Pure TypeScript implementation |
| **p.uma[crusty]** | Python | Rust bindings via uniffi |
| **@x.uma/crusty** | TypeScript | Rust bindings via WASM |

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
├── rumi/                       # Rust core (reference implementation)
├── p.uma/                      # Pure Python implementation
├── j.uma/                      # Pure TypeScript implementation
├── crusty/                     # Rust→FFI bindings (uniffi)
│   ├── p.uma/                  # Python bindings
│   └── j.uma/                  # WASM bindings
├── docs/                       # mdBook documentation
└── justfile                    # polyglot task orchestration
```

## Roadmap

| Phase | Focus | Status |
|-------|-------|--------|
| 0 | Scaffolding | ✅ Done |
| 1 | Core Traits (rumi-core) | ✅ Done |
| 2 | Conformance Fixtures | ⏳ Next |
| 3 | StringMatcher, MatcherTree | Planned |
| 4 | p.uma (Pure Python) | Planned |
| 5 | j.uma (Pure TypeScript) | Planned |
| 6 | crusty/p.uma (uniffi→Python) | Planned |
| 7 | crusty/j.uma (uniffi→WASM) | Planned |
| 8 | HTTP Domain | Planned |
| 9 | Benchmarks (all variants) | Planned |

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

## rumi Crate Structure

```
rumi/
├── rumi-core/      # Pure engine, no_std + alloc compatible
├── rumi-proto/     # Proto types + ExtensionRegistry
├── rumi-domains/   # Feature-gated adapters (test, http, claude)
└── rumi/           # Facade crate
```

Dependencies point inward: `rumi → rumi-domains → rumi-core`, `rumi → rumi-proto → rumi-core`.

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
