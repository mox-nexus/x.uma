# Contributing to x.uma

## Start here

```bash
just doctor
```

It lists every tool the repo needs, reports what you have, and exits non-zero if
something required is missing. Run it before anything else — two defects in one
week traced to a tool being present on one machine and absent on another.

Then:

```bash
just ci
```

That runs exactly what CI runs, in the same order. Green here means green there;
that is the whole premise, and anything that breaks it is a bug worth reporting.

## Prerequisites

| Tool | Why | Install |
|---|---|---|
| `cargo`, `rustc` | the Rust workspace | [rustup.rs](https://rustup.rs) — the version is pinned in `rust-toolchain.toml` |
| `just` | task runner | `cargo install just` |
| `uv` | Python environment and `puma` | [docs.astral.sh/uv](https://docs.astral.sh/uv/getting-started/installation/) |
| `bun` | `bumi` and the docs site | [bun.sh](https://bun.sh/docs/installation) |
| `node` | the doc and agreement check scripts | [nodejs.org](https://nodejs.org) |
| `buf` | proto codegen (`just gen`) | [buf.build](https://buf.build/docs/installation) |
| `maturin` | the PyO3 crust wheel | `uv pip install maturin` |
| `wasm-pack` | the wasm crust package | `cargo install wasm-pack` |
| `cargo-audit` | dependency advisories | `cargo install cargo-audit` |

The last three are optional: `just ci` runs without them, but the crust checks
and `just audit` will not.

## The rule that matters most

**A fixture comes first.** This project is conformance-driven: `spec/tests/`
is the source of truth for correctness, and every implementation must agree on
every fixture. Write the failing fixture, then make it pass.

The corollary is the rule the whole repo is organised around:

> **What CI does not check is not true.**

Every false claim ever found here — a phase marked done whose crate had never
compiled, an invariant documented as enforced that was never implemented, four
how-to pages teaching commands that did not exist, twenty-five dead links — was
outside CI's reach. If you are about to write down a claim, ask what would fail
if it stopped being true. If the answer is "nothing", write the check first.

## Rust conventions

Run these **in this order** before committing. `clippy --fix` rewrites code, so
formatting must come after it:

```bash
cargo clippy --fix --allow-dirty --manifest-path rumi/Cargo.toml -- -W clippy::pedantic
cargo fmt --manifest-path rumi/Cargo.toml --all
```

Other conventions, and the reasoning behind them, are in `CLAUDE.md`:

- **Errors are hand-written** enums with manual `Display`/`Error` impls.
  `thiserror` and `anyhow` are listed anti-patterns — each dependency is supply
  chain risk and build cost, and core has exactly one dependency (`regex`, which
  earns its place with a linear-time guarantee).
- **Core traits take `&self`.** `&mut self` breaks the `Arc<T>` wrapper algebra
  and the FFI bindings.
- **Features are strictly additive.** A feature never changes behaviour. One
  that did — `rumi-http` deleting trait impls when `proto` was enabled — was a
  bug, not a design.
- **Limits live in the constructor** of the type that holds the resource, never
  in a loader. A limit enforced in a loader is advisory to every other caller.

## Adding a conformance fixture

1. Read `spec/tests/README.md` first. There are **four** dialects and they are
   not interchangeable; `config:` is the only one a user can write.
2. Add the fixture under the right `spec/tests/` subdirectory.
3. Run it everywhere: `just test-fixtures`, `just puma-test`, `just bumi-test`.
4. If it passes in one implementation and fails in another, that is a finding,
   not a fixture bug. Cross-language disagreement is what the suite is for.

## Running one implementation

```bash
just test              # Rust
just puma-check        # Python: lint, type-check, tests
just bumi-check        # TypeScript: lint, format, type-check, tests
just crust-py-check    # PyO3 bindings (needs maturin)
just crust-wasm-check  # wasm bindings (needs wasm-pack)
just docs-build        # the docs site
```

## Before opening a pull request

```bash
just ci                  # what CI runs
just verify-clean-clone  # what a clone actually gets
```

The second one matters more than it looks. `just ci` runs against your working
tree, so it cannot see a file that exists locally but is untracked or ignored.
That class has bitten this repo three times: seventeen files hidden by a bare
`lib/` gitignore pattern, a workspace entry pointing at a deleted directory, and
generated proto code that was ignored while a crate depended on it.

## Decisions

Anything a future reader would otherwise have to reconstruct goes in
`DECISIONS.md` — what was decided, why, and what would justify revisiting it.

**Source comments do not cite `DECISIONS.md`.** A comment saying "see D-029"
couples code to a document's numbering, and this repo has already had that
numbering collide four ways. State the reason in the comment; it survives
renumbering and file moves.
