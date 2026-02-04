# Quick Start

Get x.uma running in your project.

## Install

Add to your `Cargo.toml`:

```toml
[dependencies]
rumi = "0.1"
```

## Basic Usage

```rust,ignore
use rumi::prelude::*;

// 1. Define your context type
#[derive(Debug)]
struct MyContext {
    value: String,
}

// 2. Implement DataInput to extract data
#[derive(Debug)]
struct ValueInput;

impl DataInput<MyContext> for ValueInput {
    fn get(&self, ctx: &MyContext) -> MatchingData {
        MatchingData::String(ctx.value.clone())
    }
}

// 3. Build a matcher
let matcher: Matcher<MyContext, &str> = Matcher::new(
    vec![
        FieldMatcher::new(
            Predicate::Single(SinglePredicate::new(
                Box::new(ValueInput),
                Box::new(ExactMatcher::new("hello")),
            )),
            OnMatch::Action("matched!"),
        ),
    ],
    None,
);

// 4. Evaluate
let ctx = MyContext { value: "hello".into() };
assert_eq!(matcher.evaluate(&ctx), Some("matched!"));
```

## Next Steps

- [Concepts](concepts.md) — understand the core abstractions
- [Adding a Domain](../guides/adding-domain.md) — create domain-specific matchers
