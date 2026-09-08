# Type Erasure and Ports

Why does the same `ExactMatcher` work for HTTP headers and custom event types? Because type erasure happens at the data level, not the matcher level.

## The Problem

If `InputMatcher` were generic over the context type, every domain would need its own matcher implementations:

```rust,ignore
// If InputMatcher were generic (DON'T DO THIS)
trait InputMatcher<Ctx> {
    fn matches(&self, ctx: &Ctx) -> bool;
}

// You'd need separate matchers for each domain
let http_matcher = ExactMatcher::<HttpRequest>::new("/api");
let event_matcher = ExactMatcher::<CloudEvent>::new("com.example");
// Can't put them in the same registry. No code reuse.
```

## The Solution

Erase the type at the **data level**. Extract the value first, then match the erased value:

```rust,ignore
// DataInput is generic — knows about the context
trait DataInput<Ctx> {
    fn get(&self, ctx: &Ctx) -> Option<MatchingData>;
}

// InputMatcher is NOT generic — knows only about MatchingData
trait InputMatcher {
    fn matches(&self, value: &MatchingData) -> bool;
}
```

Now one `ExactMatcher` works everywhere:

```rust,ignore
// HTTP path matching
let path_pred = SinglePredicate::new(Box::new(SimplePathInput), Box::new(ExactMatcher::new("/api")));

// Event type matching — SAME ExactMatcher
let event_pred = SinglePredicate::new(Box::new(EventTypeInput), Box::new(ExactMatcher::new("/api")));
```

## MatchingData: The Bridge

`MatchingData` is the boundary between domain-specific and domain-agnostic code:

```rust,ignore
// Declaration, shown for shape. `Arc`, not `Box`: the Arc is what makes the
// `Arc::ptr_eq` identity comparison possible, and `CustomMatchData` carries the
// `Send + Sync + Debug` bounds the FFI boundary needs.
pub enum MatchingData {
    String(String),
    Int(i64),
    Bool(bool),
    Bytes(Vec<u8>),
    Custom(Arc<dyn CustomMatchData>),
}
// Absence is not a variant. `DataInput::get` returns `Option<MatchingData>`.
```

A `DataInput` produces it; an `InputMatcher` consumes it. Nothing else touches
it, which is what lets an input and a matcher that have never heard of each
other be paired by a config file.

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

## What a Binding Sees

The types above are the engine's. The Python and TypeScript packages expose
none of them: a binding takes a config document, compiles it in Rust, and
returns a decision. There is no `DataInput` to implement in Python or
TypeScript. A new input is a Rust type registered under a type URL, and every
runtime reaches it the same way, by naming that URL in the config. One port,
one implementation, every language.

## The None Convention

`DataInput::get` returns `Option<MatchingData>`. When it returns `None`, the predicate evaluates to `false` without calling the matcher.

```rust
use rumi::prelude::*;
use rumi_http::{HttpRequest, SimpleHeaderInput};

let predicate = Predicate::Single(SinglePredicate::new(
    Box::new(SimpleHeaderInput::new("x-api-key")),
    Box::new(ExactMatcher::new("secret")),
));

// Header not present → DataInput returns None → predicate returns false
let request = HttpRequest::builder().method("GET").path("/").build();
assert!(!predicate.evaluate(&request));
```

The matcher never sees an absent value, because there is no `MatchingData` that means absent. Missing data is handled upstream, by the type. This is a security guarantee: missing data never accidentally matches, in the engine and therefore in every binding.

## Next

- [The Matching Pipeline](pipeline.md) — how data flows through the full evaluation
