# Design Principles — Evidence from 7 Elite Codebases

Architectural decisions in x.uma validated and guided by research from tower, bytes, leptos, crossbeam, ripgrep, serde, and axum.

## Confirmed Correct (Do Not Change)

These decisions independently converge with master-level patterns:

| x.uma Decision | Master Evidence | Source |
|---------------|----------------|--------|
| `evaluate(&self, ctx: &Ctx)` — stateless evaluation | `&self` for pure queries; `&mut self` only for state tracking | tower (reverted &self → &mut self for backpressure), ripgrep (stateless Matcher::find_at) |
| `MatchingData` type erasure at data level | Data models as contracts between two sides | serde (29-type model), axum (body type erased at boundary), bytes (MatchingDataType ≈ Bytes vtable) |
| Immutable after construction (`Matcher::new` → `validate` → use) | Immutable objects eliminate concurrency class entirely | crossbeam (mutation causes safety issues), tower (Clone trap) |
| 1 dependency in core (regex) | Dependencies are liabilities | serde (reverted version_check), ripgrep (hand-rolls when measured) |
| `Matcher<Ctx, A>` — two type params, no more | When a type parameter serves < 5% of users, erase it | axum (removed body type B — infected everything) |
| Eager evaluation, no lazy/streaming | Lazy infects API with type bounds | leptos (reverted lazy AsyncDerived) |
| `Box<dyn DataInput<Ctx>>` + `Box<dyn InputMatcher>` trait objects | Type erasure caps monomorphization | axum (BoxedIntoRoute, BoxCloneSyncService) |

## Actionable Improvements (Applied)

| Improvement | Principle | Source |
|-------------|-----------|--------|
| `#[diagnostic::on_unimplemented]` on `DataInput` and `InputMatcher` | Error messages are UI | serde (118 .stderr snapshots), axum (3-layer error DX) |
| `#[diagnostic::do_not_recommend]` on blanket impls | Hide noise from internal impls | axum (18 uses), serde |
| Actionable error messages in `MatcherError` | Tell user what to do, not just what's wrong | serde (composable Expected), axum (debug_handler) |

## Guardrails (Carry Forward)

Principles to enforce when making future decisions:

### Never Violate the MatchingData Contract
`MatchingData` is x.uma's 29-type model. Every `DataInput` produces it, every `InputMatcher` consumes it. Features that bypass this contract (like `#[serde(flatten)]` bypasses serde's model) will create cascading bugs.

**Trigger:** Any proposal for a new `MatchingData` variant or a "raw" evaluation path.

### No Runtime Registry in Core
Compilers (pattern matching at compile time) are strictly more correct than registries (string lookup at runtime). The `A` generic in `Matcher<Ctx, A>` flows inward from caller — cannot be resolved from config.

Registry belongs in a separate crate (`rumi-registry`) for xDS proto interop only.

**Trigger:** Any proposal to add type registration to `rumi` core. See `scratch/guild-registry-deliberation-2026-02-12.md`.

### Type Parameter Discipline
If a proposed type parameter would infect every type in the stack, erase it at the boundary instead. axum learned this with body type `B`.

**Trigger:** Third type parameter on `Matcher<Ctx, A, ???>`.

### Measure Before Optimizing
ripgrep measures everything on real workloads. Conventional wisdom is wrong (mmap = NEVER, buffer size > algorithm). x.uma benchmarks across 5 variants — keep that discipline.

**Trigger:** Any optimization without benchmark evidence.

### Protocol Obligations Are Non-Negotiable
7/7 codebases confirm: never skip a protocol step for convenience.

x.uma's protocol obligations:
- `DataInput::get` returns `None` → predicate evaluates to `false`
- `OnMatch` is exclusive: action XOR nested matcher, never both
- First-match-wins in `Matcher::evaluate`
- `validate()` before evaluation (depth limit)
- Regex uses `regex` crate only (linear time, ReDoS-safe)

**Trigger:** Any shortcut that violates these invariants.

### FFI Boundaries Are Type Erasure Boundaries
Crusty bindings (PyO3, WASM) are like axum's body boundary — accept generic types from the host language, erase to concrete Rust types immediately. The opaque engine pattern is correct.

**Trigger:** Exposing Rust generics directly through FFI.

## Cross-Codebase Pattern: 7/7

> **Protocol obligations over convenience optimizations.**

| Codebase | Obligation |
|----------|-----------|
| tower | `poll_ready` before `call` |
| bytes | `advance()` reduces capacity |
| leptos | Read signals inside reactive context |
| crossbeam | Stamp garbage with global epoch |
| ripgrep | Pre-filter: no false negatives |
| serde | 29-type data model is the contract |
| axum | Body extractors last; services Infallible |
| **x.uma** | **None → false; OnMatch exclusive; first-match-wins; validate before evaluate** |

## Sources

Full mastery extracts: `~/oss/research/*-mastery.md`
Guild deliberations: `scratch/arch-guild-reports/`
