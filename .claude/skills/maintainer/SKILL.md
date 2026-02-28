---
name: maintainer
description: "x.uma project maintainer. Use when: working on x.uma codebase, implementing matchers, adding domain extensions, running builds/tests, understanding the architecture."
---

# x.uma Maintainer

Cross-platform Unified Matcher API ecosystem implementing the xDS matcher specification.

## Why This Architecture

xDS matchers are **domain-agnostic by design**. The core evaluates predicates; what it matches *on* (inputs) and what it *does* (actions) are pluggable via `TypedExtensionConfig`. We honor this with ACES principles.

**ACES** = Adaptable · Composable · Extensible · Software

The goal: sustainable excellence — no rewrites needed. Growth happens at edges, not core.

---

## Project Structure

| Package | Language | Role |
|---------|----------|------|
| **rumi** | Rust | Core engine (reference implementation) |
| **puma** | Python | Pure Python implementation |
| **bumi** | TypeScript | Pure TypeScript implementation |
| **xuma-crust** | Python+Rust | PyO3 bindings (rumi/crusts/python/) |
| **xuma-crust** | TS+WASM | wasm-bindgen bindings (rumi/crusts/wasm/) |

```
x.uma/
├── proto/
│   ├── xds/                    # upstream (buf dep)
│   └── xuma/                   # our extensions
│       ├── core/v1/
│       ├── test/v1/
│       ├── http/v1/
│       └── claude/v1/
├── spec/tests/                 # conformance fixtures (YAML)
├── rumi/                      # Rust workspace member
├── puma/                      # Pure Python implementation
├── bumi/                      # Pure TypeScript implementation
└── justfile                    # task orchestration
```

---

## The Extension Seam

Every `input` and `action` in xDS matchers is a `TypedExtensionConfig`:

```protobuf
message TypedExtensionConfig {
  string name = 1;                       // adapter identifier
  google.protobuf.Any typed_config = 2;  // adapter config
}
```

This IS the port. Adapters are concrete types registered at runtime.

### Extension Namespace: `xuma.*`

| Package | Purpose |
|---------|---------|
| `xuma.core.v1` | Base types, registry traits |
| `xuma.test.v1` | Conformance testing (StringInput, MapInput) |
| `xuma.http.v1` | HTTP matching (HeaderInput, PathInput) |
| `xuma.claude.v1` | Claude Code hooks (HookContext, ToolUseInput) |

Type URLs: `type.googleapis.com/xuma.http.v1.HeaderInput`

---

## Decision Framework: Where Does Code Go?

| Adding... | Location | Why |
|-----------|----------|-----|
| Matcher evaluation logic | `rumi/core/src/` | Core, domain-agnostic |
| New input/action type | `proto/xuma/<domain>/v1/` | Extension, domain-specific |
| DataInput impl | `rumi/ext/<domain>.rs` | Adapter behind port |
| Python-specific wrapper | `puma/python/` | Binding ergonomics |
| Conformance test case | `spec/tests/<matcher-type>/` | Source of truth |

**The filter:** Does it know about a specific domain (HTTP, Claude, etc.)? → Adapter (rumi/ext), not core.

---

## Naming Convention (xDS Alignment)

rumi uses xDS naming throughout for ecosystem compatibility:

| xDS Term | rumi Type | Purpose |
|----------|------------|---------|
| DataInput | `trait DataInput<Ctx>` | Extracts data from context |
| InputMatcher | `trait InputMatcher<V>` | Matches extracted values |
| Predicate | `enum Predicate<Ctx>` | Boolean combinations |
| OnMatch | `struct OnMatch<Ctx, A>` | Action + nested matcher |
| FieldMatcher | `struct FieldMatcher<Ctx, A>` | Predicate + on_match pair |
| Matcher | `struct Matcher<Ctx, A>` | Top-level container |

Methods follow Envoy's naming (`get()`, `matches()`, `evaluate()`).

---

## Tooling

| Concern | Tool | Command |
|---------|------|---------|
| Task orchestration | just | `just build-all`, `just test-all` |
| Proto codegen | buf | `just gen` |
| Rust build | cargo | `cargo build -p r-umi` |
| Python env | uv | `uv sync` |
| Python build | maturin | `maturin develop --uv` |
| WASM build | wasm-pack | `wasm-pack build` |

### Build Order

```
rumi (core) → puma (pure Python) / bumi (pure TypeScript) / crusts (FFI bindings)
```

### Development Workflow

```bash
# After Rust changes
just build-rust              # or: cargo build -p r-umi

# After proto changes
just gen                     # regenerate all bindings

# Python development
cd puma && maturin develop --uv   # build + install in venv
uv run pytest                       # test

# Run all conformance tests
just test-all
```

---

## Reference Implementations

### Envoy (C++) — The Original

Location: `~/oss/envoy/source/common/matcher/`

| File | What to learn |
|------|---------------|
| `list_matcher.h` | MatcherList (first-match-wins) |
| `exact_map_matcher.h` | O(1) hash lookup |
| `prefix_map_matcher.h` | Radix tree (longest prefix) |
| `field_matcher.h` | AND/OR/NOT predicate composition |

**Key insight from Envoy:** Three-state data availability (`NotAvailable`, `MoreDataMightBeAvailable`, `AllDataAvailable`) enables streaming matching.

### xDS Protos — The Spec

Location: `~/oss/xds/xds/type/matcher/v3/`

| File | Defines |
|------|---------|
| `matcher.proto` | Core Matcher, MatcherList, MatcherTree, OnMatch |
| `string.proto` | StringMatcher (exact, prefix, suffix, contains, regex) |
| `regex.proto` | RegexMatcher (RE2 engine) |
| `ip.proto` | IPMatcher (CIDR trie) |
| `cel.proto` | CelMatcher (CEL expressions) |

---

## Conformance Testing

**Principle:** The fixture suite is the source of truth. All implementations must produce identical results.

### Fixture Format (YAML)

```yaml
# spec/tests/string_matcher/exact.yaml
name: "exact string match"
cases:
  - description: "matches identical string"
    matcher:
      string_matcher:
        exact: "hello"
    input: "hello"
    expected: { matches: true }
```

### Test Categories

| Category | What it validates |
|----------|-------------------|
| Predicate composition | AND/OR/NOT with all result combinations |
| Data availability | NotAvailable → MoreData → AllData transitions |
| MatcherList ordering | First-match-wins semantics |
| PrefixMapMatcher | Longest-match-wins with fallback |
| Nested matchers | Recursive on_match.matcher |
| on_no_match | Fallback chain behavior |

---

## Common Tasks

### Adding a New Domain (e.g., `xuma.grpc.v1`)

1. Create proto: `proto/xuma/grpc/v1/inputs.proto`
2. Define input types (e.g., `MethodInput`, `MetadataInput`)
3. Run `just gen` to generate bindings
4. Implement adapter in `rumi/src/adapters/grpc.rs`
5. Register in extension registry
6. Add conformance tests in `spec/tests/grpc/`

### Adding a New Matcher Type to Core

1. Check if xDS already defines it (probably does)
2. Implement evaluation logic in `rumi/src/matchers/`
3. Add conformance tests first (TDD)
4. Ensure all three implementations pass

### Debugging Matcher Behavior

1. Check Envoy's implementation for canonical behavior
2. Write a minimal conformance test case
3. Run against all implementations to find divergence
4. Fix implementation, not test (unless test is wrong)

---

## Watch Out For

| Gotcha | Why it matters |
|--------|----------------|
| RE2 vs regex crate | Subtle semantic differences; xDS specifies RE2 |
| Predicate short-circuit | AND fails fast, OR succeeds fast — order matters for side effects |
| on_no_match recursion | Can trigger another matcher, not just an action |
| keep_matching flag | Match succeeds but evaluation continues (audit patterns) |
| TypedExtensionConfig resolution | Name lookup happens at runtime; typos fail late |

---

## References

For detailed documentation, see:

| Need | Load |
|------|------|
| Design principles (from 7 elite codebases) | [design-principles.md](references/design-principles.md) |
| Arch-guild constraints | [arch-constraints.md](references/arch-constraints.md) |
| xDS specification details | [xds-semantics.md](references/xds-semantics.md) |
| Build system workflows | [build-system.md](references/build-system.md) |

## Commands

| Command | Purpose |
|---------|---------|
| `/validate` | Run full validation suite |
| `/check-breaking` | Check for breaking changes |
| `/audit` | Run constraint auditor agent |
