# Design Principles — Evidence from 13 Elite Codebases

Architectural decisions in x.uma validated and guided by research from tower, bytes, leptos, crossbeam, ripgrep, serde, axum, hyper, rust-analyzer, rustls, tokio, aya, and embassy.

## Confirmed Correct (Do Not Change)

These decisions independently converge with master-level patterns:

| x.uma Decision | Master Evidence | Source |
|---------------|----------------|--------|
| `evaluate(&self, ctx: &Ctx)` — stateless evaluation | `&self` for pure queries; `&mut self` only for state tracking | tower (reverted &self → &mut self for backpressure), ripgrep (stateless Matcher::find_at), hyper (`&self` enables full wrapper algebra: `&S`, `Box<S>`, `Arc<S>`) |
| `MatchingData` type erasure at data level | Data models as contracts between two sides | serde (29-type model), axum (body type erased at boundary), bytes (MatchingDataType ≈ Bytes vtable) |
| Immutable after construction (`Matcher::new` → `validate` → use) | Immutable objects eliminate concurrency class entirely | crossbeam (mutation causes safety issues), tower (Clone trap), rustls (typestate builder → frozen config), bytes (`BytesMut::freeze()` → `Bytes`) |
| Validate at construction, trust at evaluation | Push errors to setup, make hot path infallible | rustls (8/13 codebases confirm), axum (Infallible error type), hyper (state machines over futures) |
| 1 dependency in core (regex) | Dependencies are liabilities | serde (reverted version_check), ripgrep (hand-rolls when measured), hyper (17K lines futures-util → 150 lines), rust-analyzer ("each dependency is supply chain risk + build cost") |
| `Matcher<Ctx, A>` — two type params, no more | When a type parameter serves < 5% of users, erase it | axum (removed body type B — infected everything) |
| Eager evaluation, no lazy/streaming | Lazy infects API with type bounds | leptos (reverted lazy AsyncDerived) |
| `Box<dyn DataInput<Ctx>>` + `Box<dyn InputMatcher>` trait objects | Type erasure caps monomorphization | axum (BoxedIntoRoute, BoxCloneSyncService) |
| Monomorphize at registration, erase behind `Box<dyn Fn>` | Generic code at boundary, dynamic code inside | axum (BoxedIntoRoute), tower (BoxCloneService), rust-analyzer (thin generic wrapper, fat dyn interior) |
| `RegistryBuilder → Registry` (immutable after build) | Builder → frozen struct for thread-safe registries | rustls (ConfigBuilder → ClientConfig), tower (ServiceBuilder), aya (BpfLoader) |
| Manual `Display` + `Error` impls (no thiserror) | Hand-write what you need, don't import what you don't | ripgrep, rust-analyzer, hyper — consensus across maintainers |
| Feature-gated `registry` module | Additive features that don't affect core | tokio (rt, net, io features), rustls (std, ring, aws-lc-rs) |
| `evaluate()` vs `evaluate_with_trace()` as separate methods | Different modes with different perf profiles — never conflate | ripgrep (Candidate vs Confirmed), rust-analyzer (push ifs up) |

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

### Config Format Is Frozen
`MatcherConfig<A>` serialization format is now used by all 5 implementations. Any change to it is a breaking change across rumi, puma, and bumi. rust-analyzer lesson: "By the time you have first users, it is already de-facto stable."

**Trigger:** Any change to `MatcherConfig`, `PredicateConfig`, or `SinglePredicateConfig` serialization shape.

### Foundation Crates Migrate Last
rumi is the foundation crate. Every dependency change (regex version bump, serde version bump) cascades to all downstream crates and all three language implementations. hyper learned this the hard way with the futures 0.2 migration ("Much sadness" — reverted entirely).

**Trigger:** Any dependency version change in `rumi/core/Cargo.toml`.

### Protocol Obligations Are Non-Negotiable
13/13 codebases confirm: never skip a protocol step for convenience.

x.uma's protocol obligations:
- `DataInput::get` returns `None` → predicate evaluates to `false`
- `OnMatch` is exclusive: action XOR nested matcher, never both
- First-match-wins in `Matcher::evaluate`
- `validate()` before evaluation (depth limit)
- Regex uses `regex` crate only (linear time, ReDoS-safe)
- Registry immutable after `build()` — no runtime registration
- `evaluate_with_trace()` evaluates ALL children (no short-circuit for debug visibility)

**Trigger:** Any shortcut that violates these invariants.

### FFI Boundaries Are Type Erasure Boundaries
Crusty bindings (PyO3, WASM) are like axum's body boundary — accept generic types from the host language, erase to concrete Rust types immediately. The opaque engine pattern is correct.

**Trigger:** Exposing Rust generics directly through FFI.

## Tacit Knowledge (Judgment, Not Rules)

Wisdom that can't be mechanically enforced — it requires judgment:

### `&self` Enables the Full Wrapper Algebra
hyper's `Service::call(&self)` enables blanket impls for `&S`, `Box<S>`, `Arc<S>`, `Rc<S>`. x.uma's `DataInput::get(&self)` and `InputMatcher::matches(&self)` already take `&self`. If either trait ever takes `&mut self`, the crusty FFI bindings break (can't share through `Arc`).

### Thin Generic Wrapper, Fat Dynamic Interior
rust-analyzer pattern: monomorphize at the API surface for ergonomics, erase to `dyn` for the implementation body. Prevents monomorphization explosion when generic utility functions process `MatcherConfig<A>` or `Registry<Ctx>`.

### Hold Position Until Evidence Warrants Change
ripgrep resisted multiline search for years, then implemented it only when the architecture could absorb it cleanly. The arch-guild "matcher engine, not policy engine" decision follows this pattern.

### History Shows Why, Code Shows What
The arch-guild reports in `scratch/arch-guild-reports/` capture the *why* behind decisions. Maintain these as the project evolves. Without them, future contributors see arbitrary decisions.

### Warnings Are Scars
Each arch-guild constraint encodes a hard-won lesson. Each entry in the anti-patterns table is a mistake that was either made or narrowly avoided. Treat them as battle documentation.

### The BurntSushi Method
1. Measure first (benchmarks in commit messages)
2. Document the tradeoff, not just the change
3. Prefer boring code (`Arc<Mutex<Vec>>` for years before work-stealing)
4. Acknowledge uncertainty in comments ("Should re-evaluate")
5. Own the entire stack

### Reversions Encode Wisdom
All 13 mined codebases have deliberate reversions that encode hard-won lessons. When an optimization or refactor doesn't feel right, revert immediately and document the reason rather than pushing through. Same-day reverts indicate a hard constraint discovery.

### Dependencies Earn Their Place Through Measurement
"A dependency earns its place by proving itself with measurements on real workloads, not micro-benchmarks." The `regex` crate carries its weight (linear-time guarantee). Every dependency in x.uma's stack should be held to this standard.

## Cross-Codebase Pattern: 13/13

> **Protocol obligations over convenience optimizations.**

| Codebase | Obligation |
|----------|-----------|
| tower | `poll_ready` before `call` |
| serde | Visitor methods must handle all offered types |
| bytes | `freeze()` before sharing; `remaining()` before `chunk()` |
| crossbeam | Epoch pinning before accessing shared data |
| embassy | `SpawnToken` must be spawned (panic on drop) |
| hyper | Zero-capacity channel = physical backpressure |
| ripgrep | `find_candidate_line` must never report false negatives |
| rustls | Handshake before application data |
| aya | `Pod` before kernel/userspace boundary crossing |
| rust-analyzer | Incremental query protocol between salsa layers |
| axum | Handler bounds proven at registration, not request time |
| tokio | Runtime must be entered before spawning |
| leptos | Reactive graph must be tracked (`FnMut`, not `Fn`) |
| **x.uma** | **None → false; OnMatch exclusive; first-match-wins; validate before evaluate; registry immutable after build** |

## Sources

Full mastery extracts: `~/oss/research/*-mastery.md`
Research synthesis: `scratch/research-synthesis.md`
Guild deliberations: `scratch/arch-guild-reports/`
