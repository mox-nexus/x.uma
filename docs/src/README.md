# x.uma

> Cross-Platform Unified Matcher API

x.uma implements the [xDS Unified Matcher API](https://github.com/cncf/xds) across multiple languages and domains.

## What is it?

A **matcher** evaluates data against rules and returns an action. Think: routing, filtering, access control.

```text
Input → Matcher → Action
```

x.uma provides:
- **rumi** — Rust core engine (reference implementation)
- **p.uma** — Python bindings (coming soon)
- **j.uma** — TypeScript/WASM (coming soon)

## Why x.uma?

**Domain-agnostic core.** The same matcher logic works for HTTP headers, Claude hooks, gRPC metadata, or your custom domain.

**Type-safe composition.** Predicates compose with AND/OR/NOT. Matchers nest. Actions are exclusive.

**xDS compatible.** Built on the same protocol Envoy uses at scale.

## Quick Example

```rust,ignore
use rumi::prelude::*;

// Define what data we're matching on
#[derive(Debug)]
struct Request { path: String }

#[derive(Debug)]
struct PathInput;

impl DataInput<Request> for PathInput {
    fn get(&self, ctx: &Request) -> MatchingData {
        MatchingData::String(ctx.path.clone())
    }
}

// Build a matcher
let matcher: Matcher<Request, &str> = Matcher::new(
    vec![
        FieldMatcher::new(
            Predicate::Single(SinglePredicate::new(
                Box::new(PathInput),
                Box::new(PrefixMatcher::new("/api/")),
            )),
            OnMatch::Action("api_handler"),
        ),
    ],
    Some(OnMatch::Action("default_handler")),
);

// Evaluate
let req = Request { path: "/api/users".into() };
let result = matcher.evaluate(&req);
// → Some("api_handler")
```

## Documentation

| Section | What you'll learn |
|---------|-------------------|
| [Quick Start](learn/quick-start.md) | Get running in 5 minutes |
| [Concepts](learn/concepts.md) | Core ideas and terminology |
| [Architecture](explain/architecture.md) | Why it's built this way |
| [Proto API](reference/proto.md) | Generated from `.proto` files |
| [Rust API](reference/rust.md) | Generated from `rustdoc` |
