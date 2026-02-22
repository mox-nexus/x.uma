---
name: rust-mastery
description: "x.uma Rust mastery — architectural judgment from 13 elite codebases, applied to this project. Use when: writing rumi code, designing traits, choosing dependencies, making performance decisions, building registry/config patterns, or working on crusty FFI bindings. Overrides generic Rust advice with x.uma-specific conventions. Source: 13 mined codebases cross-referenced against x.uma architecture."
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

**Rule:** If either trait ever takes `&mut self`, the crusty FFI bindings break (can't share through `Arc`). This is a hard constraint.

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

x.uma does this correctly: `Matcher::new` → `validate()` → use. The hot path (`evaluate()` at 33ns) is infallible. All validation happens at load time.

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

`MatchingData` is x.uma's data model — the contract between `DataInput` (produces it) and `InputMatcher` (consumes it). Everything that maps cleanly to `MatchingData` variants works perfectly. The `Custom(Box<dyn Any>)` variant exists for extensibility but should be used sparingly.

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
| INV-7 | `evaluate_with_trace()` evaluates ALL children (no short-circuit) | Incomplete debug info — trace hides skipped branches |

## Arch-Guild Constraints

| Constraint | Enforcement |
|------------|-------------|
| ReDoS Protection | `regex` crate only (linear time). Never `fancy-regex`. |
| Max 32 Depth | `MatcherError::DepthExceeded` at `validate()` time |
| Registry Immutable | `&self` methods only after `build()` |
| Send + Sync + Debug | All public types — FFI requirement (PyO3, WASM) |
| Iterative Evaluation | No recursive `evaluate()` — use explicit stack |
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
- `cdylib` + `lib` crate types for puma-crusty; `cdylib` only for bumi-crusty
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
| `clap`, `lexopt`, CLI binary | [cli.md](references/cli.md) |
| `axum`, `tonic`, `sqlx`, API/service | [backend.md](references/backend.md) |
| `leptos`, `dioxus`, `wasm-bindgen`, browser WASM | [frontend.md](references/frontend.md) |
| `tauri`, `egui`, desktop/mobile app | [native.md](references/native.md) |
| `#![no_std]`, `cortex-m`, `embassy`, `rtic` | [embedded.md](references/embedded.md) |
| `pingora`, `rama`, `proxy`, `xds` | [data-plane.md](references/data-plane.md) |
| `bindgen`, `cbindgen`, `cxx`, `PyO3`, `unsafe` | [ffi-unsafe.md](references/ffi-unsafe.md) |
| `proc-macro = true`, `syn`, `quote` | [proc-macros.md](references/proc-macros.md) |
| `reqwest`, HTTP client, protocols | [networking.md](references/networking.md) |
| Crate selection questions | [ecosystem.md](references/ecosystem.md) |
| Project setup, CI, configs | [tooling.md](references/tooling.md) |
| Deep async patterns, tokio internals | [async.md](references/async.md) |
