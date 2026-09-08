# API Reference

Generated from source, so it cannot drift from the code.

**Rust** — browse the published crates at
[mox-nexus.github.io/x.uma/api/rust](https://mox-nexus.github.io/x.uma/api/rust),
built and deployed with the rest of this site. To build them locally:

```bash
just doc
```

That runs `cargo doc` over the published crates by name. `--workspace` does not
work here: the two crusts both produce a library called `xuma_crust`, and
rustdoc refuses to write two crates to the same path.

**Python** — the binding's classes carry their documentation from the Rust
source. After `just crust-py-check`:

```bash
cd rumi/crusts/python && uv run python -m pydoc xuma_crust
```

**TypeScript** — `just crust-wasm-build` writes the typed surface to
`rumi/crusts/wasm/pkg/xuma_crust.d.ts`.
