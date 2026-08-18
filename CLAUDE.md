# x.uma — Cross-Platform Matcher Engine

## What is x.uma?

A matcher engine implementing the xDS Unified Matcher API across multiple languages and domains.

| Package | Language | Notes |
|---------|----------|-------|
| **rumi** | Rust | Core engine (reference implementation) |
| **puma** | Python | Python implementation, no protobuf runtime (dir: `puma/`, **package `xuma`**) |
| **bumi** | Bun/TypeScript | Pure TypeScript implementation (dir: `bumi/`, **package `xuma`**) |
| **xuma-crust** | Python | Rust bindings via PyO3 (from `rumi/crusts/python/`) |
| **xuma-crust** | TypeScript | Rust bindings via wasm-bindgen (from `rumi/crusts/wasm/`) |

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
xuma.kv.v1      # Conformance testing
xuma.http.v1      # HTTP matching
xuma.claude.v1    # Claude Code hooks
xuma.grpc.v1      # gRPC matching
```

Type URLs:
- `type.googleapis.com/xuma.kv.v1.MapInput`
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
├── puma/                       # Python implementation (package: xuma)
│   └── proto/src/gen/          # buf-generated Python types (betterproto)
├── bumi/                       # Pure TypeScript implementation (package: xuma)
│   └── proto/src/gen/          # buf-generated TypeScript types (ts-proto)
├── buf.gen.yaml                # Polyglot codegen config (all 3 languages)
├── docs/
│   ├── content/                # plain Markdown, framework-free
│   └── experience/             # SvelteKit docs site (adapter-static)
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
| 5 | puma (Python + HTTP) | ✅ Done |
| 5.1 | puma arch-guild hardening | ✅ Done |
| 6 | bumi (Bun/TypeScript + HTTP) | ✅ Done |
| 6.1 | bumi arch-guild hardening | ✅ Done |
| 7 | xuma-crust: PyO3 Python bindings | ✅ Done |
| 7.5 | Claude domain: trace + HookMatch compiler (a **feature of `rumi-core`**, not a crate) | ✅ Done |
| 8 | xuma-crust: wasm-bindgen TypeScript bindings | ✅ Done |
| 9 | Cross-language benchmarks (all 5 variants) | ⚠️ Unverified |
| 10 | TypedExtensionConfig Registry (`IntoDataInput`, `RegistryBuilder`) | ✅ Done |
| 11 | Test audit (removed 18 ineffective tests → 216 total) | ✅ Done |
| 12 | Proto Alignment: buf codegen, `rumi-proto`, `AnyResolver`, xDS Matcher loading | ✅ Done |
| 13 | Config/Registry across all implementations | ✅ Done |
| 14 | Config-path benchmarks (all 5 variants) | ⚠️ Unverified |
| 15 | Crate restructure + publish prep (0.0.2) | ⚠️ Unverified |
| — | Semantic matching (cosine similarity via `CustomMatchData`) | Planned |
| — | RE2 migration: `google-re2` for puma, `re2js` for bumi | ✅ Done |

**Status legend.** ✅ Done means **CI executes something that would fail if it
regressed**. ⚠️ Unverified means the work exists but nothing checks it, so the
claim rests on someone's memory.

The general rule, learned the expensive way: **any phase whose subject is
outside CI's reach is unverified by construction.** Phase 12 sat at ✅ for
months while `rumi-proto` had never once compiled — nothing ran it, so nothing
could say otherwise. It is ✅ now because CI runs `-p rumi-proto` and a codegen
drift check.

What each ⚠️ needs to become ✅:

| Phase | Blocked on |
|---|---|
| ~~7, 8~~ | ~~CI building both crusts~~ — done 2026-08-17; both are built and their 160 tests run on every PR |
| 9, 14 | benchmarks running somewhere that can fail; they are currently manual |
| 15 | `cargo publish --dry-run` passing without path patching — `PLAN.md` Phase E |

## Current Work

**Public-readiness + mox-branded docsite**

Decisions of record live in [`DECISIONS.md`](DECISIONS.md). Read it before
revisiting anything below.

**Publish status — nothing is published yet.** Names are *chosen, not reserved*:
`rumi-core` on crates.io (lib name = `rumi`), `rumi-http`, `rumi-cli`; `xuma` on
PyPI; `xuma-crust` on PyPI/npm. All resolve 404 today. Both release workflows are
`workflow_dispatch` and have never been run. README and getting-started pages
carry pre-release notes until a release lands (D-015).

**Docsite** runs on SvelteKit, the cix pattern
(`docs/content/` + `docs/experience/`), register `cix · operator`, brand tokens
from `~/mox/brand/` (D-001 to D-005).

**Playground** is the `/playground` route of the docs app, not a separate
package. Its diagram renders with roughjs, and node sizing has a single source
in `measure.ts` (D-006 to D-009, D-024).

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
| **First-match-wins** | first match wins; `keep_matching` is **deferred, not implemented** | `Matcher::evaluate()` returns the first match. `keep_matching` appears in the proto and is accepted and ignored — see PLAN.md F2 / SF2 |

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
| **Domain compilers own the vocabulary** | `rumi-http` has `HttpRouteMatch`; the `claude` feature of `rumi-core` has `HookMatch`. |
| **Cross-domain = pipeline** | Different `Ctx` types are incomparable. Combine actions, not contexts. |

**Strategic path:** Build domain compilers now. Extract policy abstraction only when a second integration reveals cross-domain pain. The deliberation report is not in the repo; this table is the record of it.

## Domain Compiler Pattern

Each domain adapter provides a **compiler** that transforms user-friendly config into matcher trees:

| Domain | Config Type | Compiler | Output |
|--------|------------|----------|--------|
| HTTP | `HttpRouteMatch` | `compile_route_matches()` | `Matcher<HttpMessage, A>` |
| Claude | `HookMatch` | `compile_hook_matches()` | `Matcher<HookContext, A>` |

The compiler is the "door handle" — it makes the matcher engine usable without manual tree construction.

### Claude Domain Compiler (`rumi-core`, feature = "claude")

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
│       └── claude/     # Claude Code hooks (feature = "claude")
├── proto/              # Proto-generated types + conversion (package: rumi-proto, publish=false)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # Module tree for generated types
│       ├── any_resolver.rs     # google.protobuf.Any → TypedConfig bridge
│       ├── convert.rs          # Proto Matcher → MatcherConfig conversion
│       └── gen/                # buf-generated prost + prost-serde code
├── ext/
│   ├── test/           # rumi-test (conformance, publish=false)
│   └── http/           # rumi-http (HTTP matching)
└── crusts/             # Language bindings (🦀 crustacean → crusty, publish=false)
    ├── python/         # PyO3 → xuma-crust wheel (maturin)
    └── wasm/           # wasm-bindgen → xuma-crust (wasm-pack)
```

**Extension pattern:** Claude is a feature, HTTP is a separate crate:

```toml
[dependencies]
rumi-core = { version = "0.0.2", features = ["claude"] }
rumi-http = "0.0.2"
```

```rust
use rumi::prelude::*;
use rumi::claude::{HookContext, HookMatchExt};
use rumi_http::{HttpRequest, HeaderInput};
```

## Craft Judgment

Principles distilled from 13 elite Rust codebases. Each prevents a form of self-deception about quality. The evidence base is not in the repo; the `rust-mastery` skill carries what survived of it.

| Principle | What You're Fooling Yourself About | Test |
|-----------|-----------------------------------|------|
| **Measure Before Optimizing** | "This will be faster" — but you haven't profiled | Is there benchmark evidence? |
| **Config Format Is Frozen** | "We can iterate" — but 5 implementations consume it | Does this change `MatcherConfig` serialization? |
| **Foundation Crates Migrate Last** | "It's just a version bump" — but it cascades everywhere | Does this change a dep in `rumi/core/Cargo.toml`? |
| **Protocol Obligations Over Convenience** | "This shortcut is safe" — but it violates an invariant | Does this skip validate, reorder eval, or bypass None→false? |
| **Hold Position Until Evidence** | "Let's add this now" — but no second integration demands it | Is there concrete evidence, or is this speculative? |
| **Reversions Encode Wisdom** | "I'll push through" — but the architecture is pushing back | Did you try, discover it doesn't fit, and document why? |
| **Boring Code Wins** | "This is clever" — but clever breaks under maintenance | Would `Arc<Mutex<Vec>>` work here instead? |

### Anti-Patterns (From Research)

| Don't | Why | Source |
|-------|-----|--------|
| Add `thiserror` or utility crate deps | "17K lines replaced by 150" — hand-write what you need | ripgrep, hyper |
| Expand `MatcherConfig` format after shipping | Format is frozen across 5 implementations | rust-analyzer |
| Optimize without benchmarking | Bottleneck is rarely where you think | ripgrep, all 13 |
| Add `&mut self` to core traits | Breaks `Arc<T>` wrapper algebra, breaks FFI | hyper, tower |
| Test at the abstraction level | Mock the boundary (registry, FFI), not the abstraction | aya |
| Eagerly optimize the framework layer | Don't be clever with Registry — build once, use forever | rust-analyzer |

---

## Working Conventions

### Scratch Directory

`scratch/` is for session notes, research synthesis, and working documents.

### Conformance Tests

All implementations must pass all fixtures in `spec/tests/`. The fixture suite is the source of truth for correctness.

### Session Start

On new session, read `PLAN.md` top to bottom and confirm understanding with the user before proceeding. It carries the current milestone state, the open findings, and the skills to load first.

`reference/` holds the evidence `PLAN.md` and `DECISIONS.md` cite — the security review and recovered design prior art. Both are tracked; `scratch/` is not.

### Development Workflow

1. **Write fixture first** (conformance-driven development)
2. **Implement to pass fixture**
3. **Benchmark** (catch regressions early)
4. Use `just build`, `just test`, `just lint` for common tasks

### Code Quality Principles

1. **Always fix, never skip** — when lints/checks fail, fix immediately. Don't ask whether to skip.
2. **clippy --fix then fmt** — always run both in sequence before committing:
   ```bash
   cargo clippy --fix --allow-dirty --manifest-path rumi/Cargo.toml -- -W clippy::pedantic
   cargo fmt --manifest-path rumi/Cargo.toml --all
   ```
3. **Pre-commit auto-fixes** — if the hook fails, it auto-fixes and you re-stage + commit again.
