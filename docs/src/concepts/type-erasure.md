# Type Erasure and Ports

Why does the same `ExactMatcher` work for HTTP headers and custom event types? Because type erasure happens at the data level, not the matcher level.

## The Problem

If `InputMatcher` were generic over the context type, every domain would need its own matcher implementations:

```python
# If InputMatcher were generic (DON'T DO THIS)
class InputMatcher[Ctx]:
    def matches(self, ctx: Ctx) -> bool: ...

# You'd need separate matchers for each domain
http_matcher = ExactMatcher[HttpRequest]("/api")
event_matcher = ExactMatcher[CloudEvent]("com.example")
# Can't put them in the same registry. No code reuse.
```

## The Solution

Erase the type at the **data level**. Extract the value first, then match the erased value:

```python
# DataInput is generic — knows about the context
class DataInput[Ctx]:
    def get(self, ctx: Ctx) -> MatchingData: ...

# InputMatcher is NOT generic — knows only about MatchingData
class InputMatcher:
    def matches(self, value: MatchingData) -> bool: ...
```

Now one `ExactMatcher` works everywhere:

```python
# HTTP path matching
path_pred = SinglePredicate(input=PathInput(), matcher=ExactMatcher("/api"))

# Event type matching — SAME ExactMatcher
event_pred = SinglePredicate(input=EventTypeInput(), matcher=ExactMatcher("/api"))
```

## MatchingData: The Bridge

`MatchingData` is the boundary between domain-specific and domain-agnostic code. Same name in all three implementations:

**Rust:**
```rust,ignore
pub enum MatchingData {
    None,
    String(String),
    Int(i64),
    Bool(bool),
    Bytes(Vec<u8>),
    Custom(Box<dyn CustomMatchData>),
}
```

**Python:**
```python
type MatchingData = str | int | bool | bytes | None
```

**TypeScript:**
```typescript
type MatchingData = string | number | boolean | Uint8Array | null;
```

Rust uses an enum. Python and TypeScript use union types. Same concept, idiomatic syntax.

## The Two Ports

Type erasure creates two ports — the seams where domain-specific and domain-agnostic code meet:

```text
┌─────────────────────────────────────────┐
│         Domain-Specific Layer           │
│   PathInput, HeaderInput, ToolNameInput │
│   (knows about Ctx)                     │
└──────────────┬──────────────────────────┘
               │ get() returns MatchingData
               ↓
┌──────────────▼──────────────────────────┐
│         Domain-Agnostic Layer           │
│   ExactMatcher, PrefixMatcher, Regex... │
│   (knows only about MatchingData)       │
└─────────────────────────────────────────┘
```

**Extraction port (`DataInput`)** — converts `Ctx` into `MatchingData`. Domain-specific. You write one per field you want to match.

**Matching port (`InputMatcher`)** — converts `MatchingData` into `bool`. Domain-agnostic. x.uma ships five: `ExactMatcher`, `PrefixMatcher`, `SuffixMatcher`, `ContainsMatcher`, `RegexMatcher`.

## Cross-Language Comparison

The same architecture in all three languages:

| Concept | Rust | Python | TypeScript |
|---------|------|--------|------------|
| Erased data | `enum MatchingData` | `type MatchingData` (union) | `type MatchingData` (union) |
| Extraction port | `trait DataInput<Ctx>` | `Protocol[Ctx]` | `interface DataInput<Ctx>` |
| Matching port | `trait InputMatcher` | `Protocol` | `interface InputMatcher` |
| Predicate tree | `enum Predicate<Ctx>` | `type Predicate[Ctx]` (union) | `type Predicate<Ctx>` (union) |
| Pattern match | `match` expression | `match`/`case` | `instanceof` checks |
| Immutability | Owned types | `@dataclass(frozen=True)` | `readonly` fields |

## The None Convention

When a `DataInput` returns `None`/`null` (data not present), the predicate evaluates to `false` without calling the matcher. This is enforced across all implementations.

```python
from xuma import SinglePredicate, ExactMatcher
from xuma.http import HttpRequest, HeaderInput

predicate = SinglePredicate(
    input=HeaderInput("x-api-key"),
    matcher=ExactMatcher("secret"),
)

# Header not present → DataInput returns None → predicate returns False
request = HttpRequest(headers={})
assert predicate.evaluate(request) == False
```

The matcher never sees `None`. Missing data is handled upstream. This is a security guarantee: missing data never accidentally matches.

## Next

- [The Matching Pipeline](pipeline.md) — how data flows through the full evaluation
