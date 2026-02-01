# Concepts

Core abstractions in x.uma.

## The Matching Pipeline

```text
Context → DataInput → MatchingData → InputMatcher → bool
                                          ↓
                      Predicate (AND/OR/NOT composition)
                                          ↓
                              Matcher (first-match-wins)
                                          ↓
                                       Action
```

## Key Types

### MatchingData

Type-erased data that matchers operate on:

```rust
pub enum MatchingData {
    None,
    String(String),
    Int(i64),
    Bool(bool),
    Bytes(Vec<u8>),
}
```

### DataInput

Extracts data from your domain context:

```rust
pub trait DataInput<Ctx>: Send + Sync {
    fn get(&self, ctx: &Ctx) -> MatchingData;
}
```

### InputMatcher

Matches against `MatchingData` (domain-agnostic):

```rust
pub trait InputMatcher: Send + Sync {
    fn matches(&self, value: &MatchingData) -> bool;
}
```

Built-in: `ExactMatcher`, `PrefixMatcher`, `SuffixMatcher`, `ContainsMatcher`, `BoolMatcher`

### Predicate

Boolean composition of matchers:

- `Single` — one DataInput + InputMatcher
- `And` — all must match (short-circuit)
- `Or` — any must match (short-circuit)
- `Not` — negation

### OnMatch

What happens when a predicate matches:

```rust
pub enum OnMatch<Ctx, A> {
    Action(A),              // Terminal: return this action
    Matcher(Box<Matcher>),  // Nested: evaluate another matcher
}
```

**Exclusive:** action XOR nested matcher, never both.

### Matcher

Top-level container with first-match-wins semantics:

```rust
pub struct Matcher<Ctx, A> {
    rules: Vec<FieldMatcher<Ctx, A>>,
    on_no_match: Option<OnMatch<Ctx, A>>,
}
```
