---
name: rust-mastery
description: "x.uma Rust mastery — architectural judgment from 13 elite codebases, applied to this project. Use when: writing rumi code, designing traits, choosing dependencies, making performance decisions, building registry/config patterns, or working on xuma-crust FFI bindings. Overrides generic Rust advice with x.uma-specific conventions. Source: 13 mined codebases cross-referenced against x.uma architecture."
---

# Rust Mastery — x.uma Edition

Architectural judgment from 13 elite codebases (tower, bytes, leptos, crossbeam, ripgrep, serde, axum, hyper, rust-analyzer, rustls, tokio, aya, embassy), applied to x.uma's specific constraints and conventions.

**This skill overrides generic Rust advice** where x.uma's architecture demands different choices.

## x.uma Overrides

These conventions diverge from generic Rust practice. Follow these, not the defaults.

| Generic Advice | x.uma Override | Why |
|----------------|----------------|-----|
| Use `thiserror` for library errors | Manual `Display` + `Error` impls | Only dep is `regex`. ripgrep + rust-analyzer confirm: "each dependency is supply chain risk + build cost" |
| Use `anyhow` for applications | Manual error types | Crusty FFI boundaries need structured errors, not erased ones |
| Default to generics for performance | `Box<dyn DataInput>` + `Box<dyn InputMatcher>` | Type erasure at data level (Envoy pattern). Monomorphize at registration, erase behind `Box<dyn Fn>` — this IS the axum pattern |
| Use `derive_more` for boilerplate | Hand-write impls | Zero unnecessary deps in core. 1 dep (regex) is the standard. |
| `Arc<Mutex<T>>` for shared state | Immutable after build — no locks needed | `RegistryBuilder → Registry` pattern. Thread-safety through immutability, not synchronization |
| Feature flags for optional behavior | Feature flags that are strictly additive | `registry` enables serde + config. `proto` implies `registry`. Features NEVER change core behavior. |
| `async-trait` for async dyn dispatch | No async in core — evaluation is synchronous | 33ns hot path. Async would add overhead for zero benefit. |

## Judgment Patterns

### 1. `&self` Enables the Full Wrapper Algebra

**Source:** hyper (PR #3607)

x.uma's `DataInput::get(&self)` and `InputMatcher::matches(&self)` take `&self`. This enables blanket impls for `&T`, `Box<T>`, `Arc<T>` — the same wrapper algebra hyper gained when `Service::call` went from `&mut self` to `&self`.

**Rule:** If either trait ever takes `&mut self`, the xuma-crust FFI bindings break (can't share through `Arc`). This is a hard constraint.

### 2. Thin Generic Wrapper, Fat Dynamic Interior

**Source:** rust-analyzer (style.md)

```rust
// GOOD — thin generic wrapper, fat dynamic interior
fn frobnicate(f: impl FnMut()) {
    frobnicate_impl(&mut f)
}
fn frobnicate_impl(f: &mut dyn FnMut()) {
    // lots of code — only compiled once
}
```

Apply this in `Registry::load_matcher()` if the method body grows large. Monomorphize the `A` parameter at the call site, erase to `dyn` for the implementation.

### 3. Validate at Construction, Trust at Evaluation

**Source:** rustls (typestate builder → frozen config), confirmed across 8/13 codebases

x.uma does this on the loaded-config path: `Matcher::new` → `validate()` → use. The hot path (`evaluate()` at 33ns) is infallible.

**Know which path you are on.** `Matcher::new` (`matcher.rs:64`) is a bare struct constructor that validates nothing; `validate()` is a separate call. It happens for you on the registry path (`registry.rs:414`) and at every FFI entry point. It does **not** happen in either domain compiler (`ext/http/src/compiler.rs:127`, `core/src/claude/compiler.rs:109`) — the very path CLAUDE.md promotes as the door handle. A hand-built `Matcher` that never calls `validate()` carries no guarantees at all.

**Why this matters for x.uma:** Config loading can fail loudly. Evaluation must never panic. This is how Envoy works too — bad config is rejected at load, never at request time.

### 4. Monomorphize at Registration, Erase Behind `Box<dyn Fn>`

**Source:** axum (BoxedIntoRoute), tower (BoxCloneService)

x.uma's `RegistryBuilder::input::<T>(type_url)` monomorphizes at registration time, capturing the concrete type behind a `Box<dyn Fn(Value) -> Box<dyn DataInput<Ctx>>>`. After `build()`, the `Registry` contains only type-erased factories.

**Pattern:**
```rust
// At registration (generic, compiled once per type)
builder.input::<PathInput>("xuma.http.v1.PathInput");

// After build (dynamic, one code path)
registry.load_matcher(config)?;  // No generics in sight
```

### 5. MatchingData Is the Contract

**Source:** serde's 29-type data model

`MatchingData` is x.uma's data model — the contract between `DataInput` (produces it) and `InputMatcher` (consumes it). Everything that maps cleanly to `MatchingData` variants works perfectly. The `Custom(Arc<dyn CustomMatchData>)` variant (`matching_data.rs:139`) exists for extensibility but should be used sparingly.

`Arc`, not `Box`, and `CustomMatchData`, not `Any`. Both matter: `Arc` is what makes the `Arc::ptr_eq` identity comparison possible, and `CustomMatchData: Send + Sync + Debug` (`matching_data.rs:54`) carries the FFI bounds plus the `custom_type_name()` hook that config-time validation reads. `Box<dyn Any>` would compile away all three.

**Never bypass this contract.** Every DataInput produces MatchingData. Every InputMatcher consumes MatchingData. This indirection is what makes InputMatcher non-generic and shareable across all domain contexts.

### 6. OnMatch Exclusivity at the Type Level

**Source:** xDS spec, confirmed by Dijkstra (formal correctness)

```rust
// Rust enum enforces exclusivity — impossible to have both
pub enum OnMatch<Ctx, A> {
    Action(A),
    Matcher(Box<Matcher<Ctx, A>>),
}
```

This is making illegal states unrepresentable. The proto uses `oneof`, Rust uses `enum`. Both enforce the same constraint at the type level. Never weaken this to a struct with optional fields.

**The three mechanisms, in order of durability.** When you need an invariant to hold, reach for these before reaching for a runtime check or a comment.

1. **Enum over optional fields** — `OnMatch` (`on_match.rs:29`). Two things that must not co-occur become two variants. The wrong state has no spelling.

2. **Consume `self` to transition** — `RegistryBuilder::build(self)` (`registry.rs:230`) turns "do not register after build" from a rule into a move error. The general form is typestate:

   ```rust
   pub struct Thing<State> { _state: PhantomData<State> }
   pub struct Unvalidated;
   pub struct Validated;

   impl Thing<Unvalidated> {
       pub fn validate(self) -> Result<Thing<Validated>, Error> { /* ... */ }
   }

   impl Thing<Validated> {
       pub fn evaluate(&self) -> Decision { /* only reachable after validate */ }
   }
   ```

   Note what this would buy x.uma: today `Matcher::new` returns a `Matcher` whether or not `validate()` was ever called, so the domain compilers hand back matchers carrying no guarantees. A typestate `Matcher<Unvalidated>` / `Matcher<Validated>` would make that path a compile error rather than a convention. Not proposed as work here, but it is the shape of the answer if the split ever bites.

3. **Runtime check at a chokepoint** — `validate()` and the `Registry::load_*` limits. Weakest of the three, because it only holds on paths that call it. Use when the constraint is data-dependent and genuinely cannot be typed.

Prefer 1, then 2, then 3. A comment saying "remember to X" is not on the list.

### 7. Two Type Parameters, No More

**Source:** axum (removed body type `B` — infected everything)

`Matcher<Ctx, A>` has exactly two type parameters:
- `Ctx` — the domain context (HTTP, Claude, test)
- `A` — the action type (typically `String`)

If a proposed third type parameter would infect every type in the stack, erase it at the boundary instead. axum learned this the hard way.

### 8. Speculative Fast Path with Safe Fallback

**Source:** ripgrep (Candidate/Confirmed pipeline)

x.uma's evaluation follows this pattern already:
1. Check field matchers (fast path — string comparison)
2. If matched, check predicate (may involve regex)
3. If matched, return action (first-match-wins)

The fast path (exact string match at 33ns) handles the common case. Regex is the fallback. Pre-filter with literals before regex is a future optimization, but the `regex` crate already does internal literal optimization — **measure before adding**.

## Protocol Obligations (INV-1 through INV-7)

Non-negotiable. Every change must preserve these:

| # | Invariant | What Breaks If Violated |
|---|-----------|------------------------|
| INV-1 | `DataInput::get() → None` → predicate evaluates to `false` | Fail-open security — missing data could match |
| INV-2 | First-match-wins in `Matcher::evaluate()` | Semantic change — different action returned |
| INV-3 | `EvalTrace.result` always equals `evaluate()` result | Debug output lies — trace shows wrong result |
| INV-4 | `OnMatch` is exclusive — Action XOR Matcher, never both | Ambiguous semantics — which takes precedence? |
| INV-5 | `Registry` is immutable after `build()` | Thread-safety — concurrent mutation without locks |
| INV-6 | `MAX_DEPTH=32` enforced at `validate()` time | Stack overflow from deeply nested matchers |
| INV-7 | `Predicate::evaluate_with_trace()` evaluates all child **predicates** (no short-circuit) | Incomplete debug info — trace hides skipped branches |

INV-7 is scoped to the predicate tree deliberately. `Matcher::evaluate_with_trace` **does** stop at the first match (`matcher.rs:193`), because first-match-wins is INV-2 and the trace must agree with `evaluate()` per INV-3. Do not "restore" symmetry by deleting that early return. Known limitation: the matcher trace calls plain `evaluate` on the fallback (`matcher.rs:216`), so a nested `on_no_match` sub-trace is never recorded.

## Arch-Guild Constraints

| Constraint | Enforcement |
|------------|-------------|
| ReDoS Protection | `regex` crate only (linear time). Never `fancy-regex`. |
| Resource limits (5) | `MAX_DEPTH=32`, `MAX_FIELD_MATCHERS=256`, `MAX_PREDICATES_PER_COMPOUND=256`, `MAX_PATTERN_LENGTH=8192`, `MAX_REGEX_PATTERN_LENGTH=4096` (`lib.rs:183-205`). Only `MAX_DEPTH` is checked in `validate()`; the other four are enforced solely inside `Registry::load_*`, so a hand-built or compiler-built matcher bypasses them. The crusts **re-declare** the two pattern limits locally (`crusts/python/src/convert.rs:12`, `crusts/wasm/src/convert.rs:10`) instead of importing them — three copies with no compile-time link. |
| Max 32 Depth | `MatcherError::DepthExceeded` at `validate()` time |
| Registry Immutable | `&self` methods only after `build()` |
| Send + Sync + Debug | All public types — FFI requirement (PyO3, WASM) |
| Iterative Evaluation | **Deferred to v0.2.** Evaluation is recursive today (`matcher.rs:136`, `predicate.rs:150`). `MAX_DEPTH=32` holds the line. |
| Action: 'static + Clone + Send + Sync | Lifetime simplicity for FFI + first-match-wins cloning |

## FFI Boundary Patterns

### Crusty Bindings Follow the Opaque Engine Pattern

```
Config (JSON/Python dict/JS object) → Compile in Rust → Evaluate in Rust → Simple types out
```

- `#[pyclass(frozen)]` for immutable compiled matchers (Dijkstra: no state machine)
- Config types cross the FFI boundary as JSON/plain objects, NOT opaque Rust structs
- wasm-bindgen: `js_sys::Reflect::get()` for field extraction, `val.as_string()` for string detection
- PyO3: `PyStringMatchOrStr` enum + `FromPyObject` for bare string = exact match convenience

### Extension Module Gotchas

- PyO3 `extension-module` feature prevents linking to libpython → `cargo test` fails
- Solution: `default-members` in workspace excludes crusts; test via `maturin develop && pytest`
- `cdylib` + `lib` crate types for xuma-crust (Python); `cdylib` only for xuma-crust (WASM)
- Pre-commit hook uses bare `cargo test` (no `--workspace`) so `default-members` naturally excludes crusts

## Decision Quick Reference

| Decision | x.uma Answer | Evidence |
|----------|-------------|----------|
| Add a dependency? | Almost certainly not. | Core has 1 dep (regex). It earns its place through the linear-time guarantee. |
| `thiserror` or manual? | Manual `Display` + `Error` | ripgrep, rust-analyzer, hyper all hand-write errors |
| Add a type parameter? | Only if it serves >5% of uses | axum removed `B` (body type) because it infected everything |
| Optimize evaluation? | Measure first. 33ns is the baseline. | ripgrep: buffer size matters more than algorithm |
| Change `MatcherConfig` shape? | **Never.** Format is frozen. | rust-analyzer: "by the time you have first users, it is already de-facto stable" |
| Feature flag behavior? | Strictly additive. | tokio, rustls: features add capabilities, never change semantics |
| `&self` or `&mut self`? | `&self` for core traits. | hyper: `&self` enables the full wrapper algebra. `&mut self` breaks FFI. |
| Clone action in hot path? | `action.clone()` is fine at 33ns. | Measure before switching to `Arc<str>`. |
| Expand scope to policy? | **No.** Build domain compilers. | Arch-guild verdict. Generic `A` is the fence. |

## Specialized Domains

Reference files for domain-specific patterns:

| Detected | Load |
|----------|------|
| `PyO3`, `wasm-bindgen`, crust work | **FFI Boundary Patterns above** — not a reference file |
| WASM bundle size, JS interop cost | [wasm.md](references/wasm.md) |
| `clap`, `lexopt`, `rumi-cli` | [cli.md](references/cli.md) |
| Crate selection questions | [ecosystem.md](references/ecosystem.md) |
| Project setup, CI, configs | [tooling.md](references/tooling.md) |
| `bindgen`, `cbindgen`, `cxx`, raw `unsafe` | [ffi-unsafe.md](references/ffi-unsafe.md) — **future trigger.** The repo contains zero `unsafe`; the crusts use safe PyO3/wasm-bindgen. |

The corpus was trimmed on 2026-08-11. It previously carried eleven references
covering async, backend services, native GUI, embedded, proc-macros, networking
and proxy data planes — roughly 1,200 lines documenting domains this repo does
not have, three of which recommended dependencies the row above forbids. They
arrived wholesale in one commit (`52e22de`) as a side-car to an unrelated PR and
were never reconciled against this project. Do not re-add a reference for a
domain until the repo actually triggers it.
