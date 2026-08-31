# API Reference

Generated from source, so it cannot drift from the code.

**Rust** — browse the published crates at [`/api/rust`](/api/rust), built and
deployed with the rest of this site. To build them locally:

```bash
just doc
```

That runs `cargo doc` over the published crates by name. `--workspace` does not
work here: the two crusts both produce a library called `xuma_crust`, and
rustdoc refuses to write two crates to the same path.

**Python** — `cd puma && uv run pdoc xuma`

**TypeScript** — `cd bumi && bunx typedoc src/index.ts`
