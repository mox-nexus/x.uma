# Type Erasure and Ports

Why does the same `ExactMatcher` work for HTTP headers and event types? Type erasure at the data level.

## The Problem

If `InputMatcher` were generic over the context type, you couldn't share matchers across domains:

```python
# If InputMatcher were generic (DON'T DO THIS)
class InputMatcher[Ctx]:
    def matches(self, ctx: Ctx) -> bool: ...

# Now you need different matchers for each domain
http_matcher = ExactMatcher[HttpRequest]("/api")
event_matcher = ExactMatcher[CloudEvent]("com.example.user")

# Can't put them in the same list!
matchers = [http_matcher, event_matcher]  # Type error!
```

Every domain would need its own matcher implementations. No code reuse. That's not scalable.

## The Solution

Erase the type at the **data level**, not the matcher level. Extract the value first, then match against the type-erased value.

```python
# InputMatcher is non-generic
class InputMatcher:
    def matches(self, value: MatchingData) -> bool: ...

# DataInput is generic and domain-specific
class DataInput[Ctx]:
    def get(self, ctx: Ctx) -> MatchingData: ...
```

Now one `ExactMatcher` works for all domains:

```python
# Same matcher works for HTTP paths
path_predicate = SinglePredicate(
    input=PathInput(),           # Extracts str from HttpRequest
    matcher=ExactMatcher("/api") # Matches the str
)

# And for event types
event_predicate = SinglePredicate(
    input=EventTypeInput(),      # Extracts str from CloudEvent
    matcher=ExactMatcher("/api") # SAME matcher!
)
```

## MatchingValue: The Erased Type

`MatchingValue` is the bridge between domain-specific and domain-agnostic code.

**Python:**
```python
type MatchingValue = str | int | bool | bytes | None
```

**Rust:**
```rust
pub enum MatchingData {
    None,
    String(String),
    Int(i64),
    Bool(bool),
    Bytes(Vec<u8>),
}
```

**TypeScript:**
```typescript
type MatchingData = string | number | boolean | Uint8Array | null;
```

Same concept, idiomatic types for each language. Rust uses an enum. Python and TypeScript use union types.

## Example: Sharing Matchers Across Domains

The same `ExactMatcher` matches HTTP headers and CloudEvent attributes:

```python
from puma import SinglePredicate, ExactMatcher
from puma.http import HttpRequest, HeaderInput
from dataclasses import dataclass

# HTTP domain
http_predicate = SinglePredicate(
    input=HeaderInput("content-type"),
    matcher=ExactMatcher("application/json")  # Match header value
)

http_request = HttpRequest(headers={"content-type": "application/json"})
assert http_predicate.evaluate(http_request) == True

# CloudEvent domain
@dataclass
class CloudEvent:
    content_type: str
    data: dict

@dataclass
class ContentTypeInput:
    def get(self, ctx: CloudEvent):
        return ctx.content_type

event_predicate = SinglePredicate(
    input=ContentTypeInput(),
    matcher=ExactMatcher("application/json")  # SAME matcher!
)

event = CloudEvent(content_type="application/json", data={})
assert event_predicate.evaluate(event) == True
```

One `ExactMatcher` implementation. Two domains. This is type erasure in action.

## Port Architecture

Type erasure creates two ports:

```text
┌─────────────────────────────────────────┐
│         Domain-Specific Layer           │
│  (knows about Ctx: HttpRequest, etc.)   │
│                                         │
│  PathInput, HeaderInput, EventTypeInput │
└──────────────┬──────────────────────────┘
               │ get() returns MatchingValue
               ↓
┌──────────────▼──────────────────────────┐
│         Domain-Agnostic Layer           │
│  (knows only about MatchingValue)       │
│                                         │
│  ExactMatcher, PrefixMatcher, Regex...  │
└─────────────────────────────────────────┘
```

**Extraction port (`DataInput`):** Converts `Ctx` → `MatchingValue`. Domain-specific.

**Matching port (`InputMatcher`):** Converts `MatchingValue` → `bool`. Domain-agnostic.

The boundary is `MatchingValue`. Cross it once, and matchers work everywhere.

## Cross-Language Comparison

All three implementations use type erasure. The syntax differs, but the pattern is identical.

**Rust:**
```rust
// Type-erased data
pub enum MatchingData {
    String(String),
    Int(i64),
    // ...
}

// Domain-specific extraction (generic)
pub trait DataInput<Ctx>: Send + Sync {
    fn get(&self, ctx: &Ctx) -> MatchingData;
}

// Domain-agnostic matching (non-generic)
pub trait InputMatcher: Send + Sync {
    fn matches(&self, value: &MatchingData) -> bool;
}
```

**Python:**
```python
# Type-erased data
type MatchingValue = str | int | bool | bytes | None

# Domain-specific extraction (generic)
class DataInput[Ctx](Protocol):
    def get(self, ctx: Ctx) -> MatchingValue: ...

# Domain-agnostic matching (non-generic)
class InputMatcher(Protocol):
    def matches(self, value: MatchingValue) -> bool: ...
```

**TypeScript:**
```typescript
// Type-erased data
type MatchingData = string | number | boolean | Uint8Array | null;

// Domain-specific extraction (generic)
interface DataInput<Ctx> {
  get(ctx: Ctx): MatchingData;
}

// Domain-agnostic matching (non-generic)
interface InputMatcher {
  matches(value: MatchingData): boolean;
}
```

Same structure. Same insight. Type erasure at the data level enables matcher reuse.

## Performance Note

Type erasure uses dynamic dispatch (`Box<dyn InputMatcher>` in Rust, protocol conformance in Python, vtable lookup in TypeScript). This has a small runtime cost.

**Benchmark (simple exact match):**
- TypeScript (JIT-optimized): 9.3 ns/op
- Rust (vtable dispatch): 33 ns/op
- Python (protocol check): ~100 ns/op

For simple operations, TypeScript's JIT can beat Rust's vtable dispatch. For complex operations (regex, deep trees), Rust's compiled code dominates. The point: dynamic dispatch is fast enough. The flexibility is worth the cost.

## Next Steps

- [The Matching Pipeline](pipeline.md) — How data flows through the system
- [Predicate Composition](predicates.md) — Building complex conditions
- [Adding a Domain](../guides/adding-domain.md) — Create your own DataInput types
