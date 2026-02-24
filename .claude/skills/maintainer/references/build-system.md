# Build System

x.uma uses a polyglot build system coordinated by `just`.

## Toolchain Overview

| Tool | Version | Purpose |
|------|---------|---------|
| `just` | latest | Task orchestration |
| `buf` | latest | Proto linting, codegen, breaking checks |
| `cargo` | stable | Rust builds |
| `uv` | latest | Python env management |
| `maturin` | latest | Rust→Python bindings |
| `wasm-pack` | latest | Rust→WASM builds |

---

## Proto Pipeline

### Configuration

**buf.yaml:**
```yaml
version: v2
modules:
  - path: proto
deps:
  - buf.build/envoyproxy/xds
  - buf.build/envoyproxy/protoc-gen-validate
lint:
  use:
    - STANDARD
breaking:
  use:
    - FILE
```

**buf.gen.yaml:**
```yaml
version: v2
plugins:
  - remote: buf.build/community/neoeinstein-prost
    out: rumi/src/gen
    opt:
      - file_descriptor_set
  - remote: buf.build/community/neoeinstein-prost-serde
    out: rumi/src/gen
```

### Commands

```bash
# Generate bindings
just gen            # or: buf generate

# Lint protos
just lint-proto     # or: buf lint

# Check breaking changes against main
just check-breaking # or: buf breaking --against .git#branch=main

# Full proto workflow
just proto          # gen + lint + breaking
```

### After Proto Changes

1. Run `just gen` to regenerate bindings
2. Run `just lint-proto` to validate
3. If public API changed, run `just check-breaking`
4. Commit both `.proto` files and generated code together

---

## Rust Pipeline

### Workspace Structure

```
rumi/
├── Cargo.toml       # Workspace root
├── core/            # Core engine (package: rumi)
│   └── src/claude/  # Claude Code hooks (feature = "claude")
└── ext/             # Domain extensions
    ├── test/        # rumi-test (conformance, publish=false)
    └── http/        # rumi-http
```

### Commands

```bash
# Build
just build-rust     # or: cargo build --manifest-path rumi/Cargo.toml --workspace

# Build all features
just build-rust-all # includes optional features

# Test
just test-rust      # or: cargo test --manifest-path rumi/Cargo.toml --workspace

# Lint
just clippy         # pedantic warnings

# Format
just fmt            # apply formatting
just fmt-check      # check only (CI)

# Docs
just doc            # generate and open docs

# Clean
just clean-rust     # remove target/
```

### Build Order

```
rumi (core, includes claude feature)
    ↓
rumi-test, rumi-http (depend on rumi)
```

### Publish Order

```bash
# Dry run first
just publish-dry-run

# Then publish in order:
# 1. rumi (core, includes claude feature)
# 2. rumi-http (separate crate, heavy deps)
```

---

## Python Pipeline (puma)

### Setup

```bash
cd puma
uv sync              # Create venv, install deps
```

### Development

```bash
maturin develop --uv  # Build Rust extension, install in venv
uv run pytest         # Run tests
uv run python         # Interactive shell with extension
```

### Build Wheel

```bash
maturin build --release
# Wheel in target/wheels/
```

---

## WASM Pipeline (bumi)

### Build

```bash
cd rumi/crusts/wasm
wasm-pack build --target web    # ESM with async init(), works in browser + Bun
```

**Note:** Always use `--target web`. The `--target nodejs` generates `require('fs')` calls that break browsers. The `web` target uses `fetch()` in browsers and works with Bun's ESM loader.

### Output

```
pkg/
├── rumi.js          # JS wrapper
├── rumi_bg.wasm     # WASM binary
├── rumi.d.ts        # TypeScript types
└── package.json
```

---

## CI Validation

Pre-commit and CI should run:

```bash
# 1. Proto
buf lint
buf breaking --against origin/main

# 2. Rust
cargo fmt --all -- --check
cargo clippy --workspace -- -W clippy::pedantic -D warnings
cargo test --workspace

# 3. Constraints
./scripts/check-constraints.sh
```

---

## Justfile Quick Reference

| Recipe | Purpose |
|--------|---------|
| `just gen` | Proto codegen |
| `just lint-proto` | Buf lint |
| `just check-breaking` | Breaking change detection |
| `just build-rust` | Cargo build |
| `just test-rust` | Cargo test |
| `just clippy` | Lint with pedantic |
| `just fmt` | Format code |
| `just fmt-check` | Check formatting |
| `just doc` | Generate docs |
| `just clean` | Clean all targets |
| `just watch` | Watch mode for development |
