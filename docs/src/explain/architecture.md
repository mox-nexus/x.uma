# Architecture

Why x.uma is built the way it is, and how the same architecture maps across Rust, Python, and TypeScript.

## Design Philosophy: ACES

**A**daptable · **C**omposable · **E**xtensible · **S**ustainable

```text
┌─────────────────────────────────────┐
│         Domain Adapters             │
│   xuma.http  xuma.claude  xuma.grpc │
└───────────────┬─────────────────────┘
                │
┌───────────────▼─────────────────────┐
│              PORTS                  │
│     DataInput       ActionPort      │
│   (extract data)  (emit result)     │
└───────────────┬─────────────────────┘
                │
┌───────────────▼─────────────────────┐
│              CORE                   │
│           Matcher Engine            │
│     Matcher · Predicate · Tree      │
│       (pure, domain-agnostic)       │
└─────────────────────────────────────┘
```

This architecture applies to **all implementations** — rumi (Rust), puma (Python), bumi (TypeScript). The ports and hexagonal design are language-agnostic.

## The Extension Seam

`TypedExtensionConfig` from xDS is the architectural seam:

```protobuf
message TypedExtensionConfig {
  string name = 1;                       // adapter identifier
  google.protobuf.Any typed_config = 2;  // adapter config
}
```

Every `input` and `action` is a port. Adapters are concrete registered types.

## Why Type Erasure at Data Level

Key insight from the spike phase: erase types at the **data level**, not the predicate level.

### Rust (rumi)

```rust
// MatchingData — the erased type
pub enum MatchingData { None, String(String), Int(i64), Bool(bool), Bytes(Vec<u8>) }

// DataInput — domain-specific, returns erased type
pub trait DataInput<Ctx>: Send + Sync + Debug {
    fn get(&self, ctx: &Ctx) -> MatchingData;
}

// InputMatcher — domain-agnostic, NON-GENERIC
pub trait InputMatcher: Send + Sync + Debug {
    fn matches(&self, value: &MatchingData) -> bool;
}
```

**Why this works:**
- `InputMatcher` is non-generic → same `ExactMatcher` works everywhere
- No GATs or complex lifetimes
- Battle-tested at Google scale (Envoy uses this approach)

### Python (puma)

Python gets type erasure for free via union types:

```python
# MatchingValue — union type replaces Rust's enum
type MatchingValue = str | int | bool | bytes | None

# DataInput — protocol with contravariant Ctx
class DataInput(Protocol[Ctx]):
    def get(self, ctx: Ctx, /) -> MatchingValue: ...

# InputMatcher — protocol, non-generic
class InputMatcher(Protocol):
    def matches(self, value: MatchingValue, /) -> bool: ...
```

**Key differences:**
- No enum needed — union types are native
- Protocols instead of traits (runtime-checkable)
- `MatchingValue` is just a type alias, not a wrapped type

### TypeScript (bumi)

```typescript
// MatchingData — union type
type MatchingData = string | number | boolean | Uint8Array | null;

// DataInput — generic interface
interface DataInput<Ctx> {
  get(ctx: Ctx): MatchingData;
}

// InputMatcher — non-generic interface
interface InputMatcher {
  matches(value: MatchingData): boolean;
}
```

**Key differences:**
- Interfaces instead of traits/protocols
- Structural typing (duck-typed) vs nominal (Rust) vs runtime-checkable (Python)
- Union types native like Python

## Type System Mappings

How the same architecture translates across languages:

| Concept | Rust (rumi) | Python (puma) | TypeScript (bumi) |
|---------|-------------|---------------|-------------------|
| **Erased data** | `enum MatchingData` | `type MatchingValue` (union) | `type MatchingData` (union) |
| **Extraction port** | `trait DataInput<Ctx>` | `Protocol[Ctx]` | `interface DataInput<Ctx>` |
| **Matching port** | `trait InputMatcher` | `Protocol` | `interface InputMatcher` |
| **Predicate tree** | `enum Predicate<Ctx>` | `type Predicate[Ctx]` (union) | `type Predicate<Ctx>` (discriminated union) |
| **OnMatch** | `enum OnMatch<Ctx, A>` | `type OnMatch[Ctx, A]` (union) | `type OnMatch<Ctx, A>` (discriminated union) |
| **Pattern match** | `match` expression | `match`/`case` statement | `instanceof` checks |
| **Immutability** | Owned types, no `mut` | `@dataclass(frozen=True)` | `readonly` fields |
| **Thread safety** | `Send + Sync` bounds | Not applicable (GIL) | Not applicable (single-threaded) |

## Predicate Composition

All three languages express the same Boolean logic:

```rust
// Rust
pub enum Predicate<Ctx> {
    Single(SinglePredicate<Ctx>),
    And { predicates: Vec<Predicate<Ctx>> },
    Or { predicates: Vec<Predicate<Ctx>> },
    Not { predicate: Box<Predicate<Ctx>> },
}
```

```python
# Python
type Predicate[Ctx] = SinglePredicate[Ctx] | And[Ctx] | Or[Ctx] | Not[Ctx]

@dataclass(frozen=True)
class And[Ctx]:
    predicates: tuple[Predicate[Ctx], ...]
```

```typescript
// TypeScript — classes with instanceof, not discriminated unions
class SinglePredicate<Ctx> {
    constructor(
        readonly input: DataInput<Ctx>,
        readonly matcher: InputMatcher
    ) {}
}

class And<Ctx> {
    constructor(readonly predicates: readonly Predicate<Ctx>[]) {}
}

class Or<Ctx> {
    constructor(readonly predicates: readonly Predicate<Ctx>[]) {}
}

class Not<Ctx> {
    constructor(readonly predicate: Predicate<Ctx>) {}
}

type Predicate<Ctx> = SinglePredicate<Ctx> | And<Ctx> | Or<Ctx> | Not<Ctx>;

// Pattern matching via instanceof
if (p instanceof SinglePredicate) { /* ... */ }
else if (p instanceof And) { /* ... */ }
```

**Key difference:** TypeScript uses classes with `instanceof` checks, not objects with `type` discriminator fields. This matches Python's dataclass approach more closely than traditional TS discriminated unions.

## OnMatch Exclusivity (xDS Semantics)

All three enforce the xDS invariant: action XOR nested matcher, never both.

```rust
// Rust — illegal states unrepresentable
pub enum OnMatch<Ctx, A> {
    Action(A),
    Matcher(Box<Matcher<Ctx, A>>),
}
```

```python
# Python — union type enforces exclusivity
type OnMatch[Ctx, A] = Action[A] | NestedMatcher[Ctx, A]

@dataclass(frozen=True)
class Action[A]:
    value: A

@dataclass(frozen=True)
class NestedMatcher[Ctx, A]:
    matcher: Matcher[Ctx, A]
```

```typescript
// TypeScript — classes with instanceof
class Action<A> {
    constructor(readonly value: A) {}
}

class NestedMatcher<Ctx, A> {
    constructor(readonly matcher: Matcher<Ctx, A>) {}
}

type OnMatch<Ctx, A> = Action<A> | NestedMatcher<Ctx, A>;

// Check variant
if (onMatch instanceof Action) { return onMatch.value; }
else if (onMatch instanceof NestedMatcher) { return onMatch.matcher.evaluate(ctx); }
```

## Evaluation Semantics

First-match-wins is identical across all implementations:

1. Evaluate `field_matchers` in order
2. Return action from first matching predicate
3. If nested matcher returns `None`/`null`, continue to next field
4. If no matches, consult `on_no_match` fallback
5. If no fallback, return `None`/`null`

## Cross-Language Conformance

All implementations pass the same YAML conformance test suite (`spec/tests/`):

```yaml
# spec/tests/predicate/single/exact.yaml
name: "Single predicate with exact match"
cases:
  - input: "hello"
    matcher: { exact: "hello" }
    expected: { matches: true }
```

**Test runners:**
- Rust: `cargo test` (rumi-test crate) — 195 tests
- Python: `pytest` (puma/tests/) — 194 tests
- TypeScript: `bun test` (bumi/tests/) — 202 tests

Each language's test runner parses the same YAML fixtures, constructs matchers in its type system, and asserts the same expected outcomes. Total: **268 tests across 5 variants** (including puma-crusty and bumi-crusty).

## Crate/Package Structure

### Rust (rumi)

```text
rumi/
├── rumi/               # Core engine (package: rumi)
└── ext/                # Domain extensions
    ├── test/           # rumi-test (conformance)
    ├── http/           # rumi-http
    └── claude/         # rumi-claude (Claude Code hooks)
```

Dependencies point inward. Core knows nothing about domains.

### Python (puma)

```text
puma/
└── src/puma/
    ├── __init__.py     # Core types (flat exports)
    ├── _types.py       # Protocols
    ├── _predicate.py   # Predicates
    ├── _matcher.py     # Matcher tree
    ├── _string_matchers.py
    └── http/
        ├── __init__.py # HTTP domain (flat exports)
        ├── _request.py
        ├── _inputs.py
        └── _gateway.py # Gateway API compiler
```

Flat exports via `__init__.py`. Private modules prefixed with `_`.

### TypeScript (bumi)

```text
bumi/
└── src/
    ├── index.ts        # Core types
    ├── types.ts        # Interfaces
    ├── predicate.ts
    ├── matcher.ts
    ├── string-matchers.ts
    └── http/
        ├── index.ts    # HTTP domain
        ├── request.ts
        ├── inputs.ts
        └── gateway.ts  # Gateway API compiler
```

Standard TypeScript barrel exports.

## Performance Characteristics

| Implementation | Regex Engine | Thread Safety | Memory Model | FFI Overhead |
|----------------|--------------|---------------|--------------|--------------|
| rumi (Rust) | `regex` crate (linear-time) | Send + Sync | Zero-copy where possible | N/A |
| puma (Python) | `re` module (backtracking) | GIL (not parallel-safe) | Reference-counted | N/A |
| puma-crusty | `regex` via PyO3 (linear-time) | GIL | Crossing FFI boundary | ~200ns/call |
| bumi (TypeScript) | JS `RegExp` (V8 engine) | Single-threaded | Garbage-collected | N/A |
| bumi-crusty | `regex` via WASM (linear-time) | Single-threaded | Crossing WASM boundary | ~50ns/call |

**Benchmark highlights** (from Phase 9):
- rumi evaluation: 11ns (native Rust, zero overhead)
- puma evaluation: 200ns (Python overhead, CPython 3.14)
- bumi evaluation: 9ns (V8 JIT, near-native performance)
- puma-crusty: 400ns (Python + FFI crossing)
- bumi-crusty: 60ns (WASM + boundary crossing)

**ReDoS comparison** (`(a+)+$` pattern, N=20):
- rumi: 11ns (linear-time regex)
- puma: 72ms (exponential backtracking)
- bumi: 11ms (V8 optimizations help but still exponential)
- puma-crusty: 11ns (Rust regex via FFI)
- bumi-crusty: 11ns (Rust regex via WASM)

See [Performance > Benchmarks](../performance/benchmarks.md) for full data.

## Why Multiple Implementations?

**Ecosystem reach:** Deploy matchers where your code lives.
- Rust: Envoy ext_proc, high-performance services
- Python: Data pipelines, ML inference, scripting
- TypeScript: Edge workers (Cloudflare, Deno), serverless

**Reference consistency:** All implementations are ports, not wrappers. Same architecture, same semantics, same test suite.

**Learning path:** Pure implementations (rumi, puma, bumi) are readable references. Crusty variants (PyO3, WASM) provide Rust performance when needed.

## See Also

- [Roadmap](../development/roadmap.md) — Implementation status
- [Adding a Domain](../guides/adding-domain.md) — Extend with custom contexts
- [Why ACES](why-aces.md) — Design philosophy deep dive
