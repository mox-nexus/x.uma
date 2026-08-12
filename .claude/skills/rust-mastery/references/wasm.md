# WASM — the `xuma-crust` browser binding

Scope: `rumi/crusts/wasm/`, built with `wasm-pack build --target web`.

This file replaces the former `frontend.md`. x.uma ships no Rust UI framework —
there is no Leptos, Dioxus, Tauri or egui anywhere in the tree. The only browser
target is the wasm-bindgen crust, and the playground, which is Svelte and
consumes the crust as a module.

For the binding patterns themselves (`js_sys::Reflect::get()`, `as_string()`,
the `cdylib`-only crate type), see **SKILL.md → FFI Boundary Patterns**. This
file covers only what that section does not: bundle size and boundary cost.

---

## Bundle size — currently unoptimized

The workspace defines **no `[profile.release]`** (`rumi/Cargo.toml`), so the
crust builds with default release settings. For a module shipped to browsers
that is a live gap, not a preference.

```toml
[profile.release]
opt-level = "z"      # optimize for size, not speed
lto = true
codegen-units = 1
panic = "abort"      # unwinding tables are dead weight in wasm
```

Post-process and measure:

```bash
wasm-opt -Oz -o out.wasm in.wasm
twiggy top -n 20 xuma_crust_bg.wasm     # what is actually big
```

**`wee_alloc` is dead** — it leaks and is unmaintained. Use the default
allocator. Any guidance recommending it is stale.

Before changing any of this, measure. `panic = "abort"` in particular trades
away unwinding, and the crust's error path crosses the JS boundary.

---

## Boundary crossings are the cost

Each Rust↔JS call carries overhead. The engine's evaluation is ~33ns; a boundary
crossing is not. Batch at the boundary rather than in the loop.

```rust
// Bad: one crossing per item
for item in items {
    js_log(&item.to_string());
}

// Good: one crossing
js_log(&items.join("\n"));
```

This is why the crust exposes an opaque compiled matcher and evaluates inside
Rust, rather than exposing the tree and letting JS walk it. Keep it that way:
a chatty API would spend more time crossing than matching.

**Intern strings that repeat across calls:**

```rust
let key = wasm_bindgen::intern("method");
```

Worth it for the fixed key set a context is read with; not worth it for values.

---

## Gotchas specific to this crust

- **`crate-type = ["cdylib"]` only**, unlike the PyO3 crust which is
  `["cdylib", "lib"]`. Deliberate: nothing depends on the wasm crust as a Rust
  library, and adding `lib` doubles compile time for no consumer.
- **`publish = false`** — the npm artifact is produced by `wasm-pack`, not by
  cargo. Publishing the crate would ship Rust source nobody consumes.
- The crust re-declares `MAX_PATTERN_LENGTH` / `MAX_REGEX_PATTERN_LENGTH` in
  `src/convert.rs` instead of importing them from `rumi-core`. If you change a
  limit in core, this copy does not follow. See SKILL.md → Arch-Guild
  Constraints.
