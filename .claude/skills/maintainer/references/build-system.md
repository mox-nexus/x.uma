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
  - local: protoc-gen-prost
    out: rumi/rumi-proto/src/gen
    opt:
      - file_descriptor_set
  - local: protoc-gen-prost-serde
    out: rumi/rumi-proto/src/gen
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
├── rumi/            # Facade crate (re-exports)
├── rumi-core/       # Core engine (no_std)
├── rumi-proto/      # Proto bindings + registry
└── rumi-domains/    # Domain adapters
```

### Features

| Feature | Crate | Purpose |
|---------|-------|---------|
| `std` (default) | rumi-core | Standard library |
| `alloc` | rumi-core | no_std with allocator |
| `test` | rumi-domains | Test domain adapter |
| `http` | rumi-domains | HTTP domain adapter |
| `claude` | rumi-domains | Claude Code hooks adapter |

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

# Verify no_std
just build-no-std   # builds with --no-default-features --features alloc

# Docs
just doc            # generate and open docs

# Clean
just clean-rust     # remove target/
```

### Build Order

```
rumi-core (no deps)
    ↓
rumi-proto (depends on rumi-core)
    ↓
rumi-domains (depends on rumi-core, rumi-proto)
    ↓
rumi (facade, depends on all)
```

### Publish Order

```bash
# Dry run first
just publish-dry-run

# Then publish in order:
# 1. rumi-core
# 2. rumi-proto
# 3. rumi-domains
# 4. rumi
```

---

## Python Pipeline (p.uma)

### Setup

```bash
cd p.uma
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

## WASM Pipeline (j.uma)

### Build

```bash
cd rumi
wasm-pack build --target web    # For browser
wasm-pack build --target nodejs # For Node.js
```

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
cargo build --no-default-features --features alloc  # no_std check

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
| `just build-no-std` | Verify no_std compat |
| `just doc` | Generate docs |
| `just clean` | Clean all targets |
| `just watch` | Watch mode for development |
