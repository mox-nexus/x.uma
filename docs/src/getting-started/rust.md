# Rust Quick Start

Build an HTTP route matcher with `rumi` and `rumi-http`.

## Install

```toml
[dependencies]
rumi = "0.0.2"
rumi-http = "0.0.2"
```

`rumi-http` brings in `rumi` as a transitive dependency.

## Your First Matcher

Match requests by path prefix:

```rust,ignore
use rumi::prelude::*;
use rumi_http::*;

fn main() {
    // Build a simple request
    let request = HttpRequest::builder()
        .method("GET")
        .path("/api/users")
        .build();

    // Define a predicate: path starts with /api
    let predicate = Predicate::Single(SinglePredicate::new(
        Box::new(SimplePathInput),
        Box::new(PrefixMatcher::new("/api")),
    ));

    // Build the matcher tree
    let matcher: Matcher<HttpRequest, &str> = Matcher::new(
        vec![FieldMatcher::new(
            predicate,
            OnMatch::Action("api_backend"),
        )],
        Some(OnMatch::Action("default_backend")),
    );

    // Evaluate
    assert_eq!(matcher.evaluate(&request), Some(&"api_backend"));

    // No match falls through to on_no_match
    let other = HttpRequest::builder().method("GET").path("/other").build();
    assert_eq!(matcher.evaluate(&other), Some(&"default_backend"));
}
```

`Matcher::new` takes a list of `FieldMatcher`s (tried in order) and an optional fallback. First match wins.

## Combining Conditions

Use `Predicate::And` to require multiple conditions:

```rust,ignore
use rumi::prelude::*;
use rumi_http::*;

// GET /api/* — both path AND method must match
let predicate = Predicate::And(vec![
    Predicate::Single(SinglePredicate::new(
        Box::new(SimplePathInput),
        Box::new(PrefixMatcher::new("/api")),
    )),
    Predicate::Single(SinglePredicate::new(
        Box::new(SimpleMethodInput),
        Box::new(ExactMatcher::new("GET")),
    )),
]);

let matcher: Matcher<HttpRequest, &str> = Matcher::new(
    vec![FieldMatcher::new(predicate, OnMatch::Action("api_read"))],
    Some(OnMatch::Action("not_found")),
);

let get_api = HttpRequest::builder().method("GET").path("/api/users").build();
assert_eq!(matcher.evaluate(&get_api), Some(&"api_read"));

let post_api = HttpRequest::builder().method("POST").path("/api/users").build();
assert_eq!(matcher.evaluate(&post_api), Some(&"not_found"));
```

## Custom Action Types

The action type `A` is generic. Use enums, structs — anything `Clone + Send + Sync`:

```rust,ignore
#[derive(Debug, Clone, PartialEq)]
enum RouteAction {
    Forward(String),
    Deny(String),
}

let matcher: Matcher<HttpRequest, RouteAction> = Matcher::new(
    vec![FieldMatcher::new(
        predicate,
        OnMatch::Action(RouteAction::Forward("api-service".into())),
    )],
    Some(OnMatch::Action(RouteAction::Deny("no route".into()))),
);
```

## Thread Safety

Matchers are `Send + Sync`. Share one instance across threads with no locking:

```rust,ignore
use std::sync::Arc;

let matcher = Arc::new(matcher);
let m = matcher.clone();
std::thread::spawn(move || {
    let result = m.evaluate(&request);
});
```

The matcher is immutable after construction. `Arc` adds one atomic refcount — nothing else.

## Safety Guarantees

- **ReDoS protection** — the `regex` crate guarantees linear-time matching. No backtracking.
- **Depth limits** — nested matchers capped at 32 levels, validated at construction.
- **No unsafe in core** — all `Send + Sync` is compiler-derived.

## Next Steps

- [The Matching Pipeline](../concepts/pipeline.md) — how data flows through the matcher
- [Build an HTTP Router](../tutorials/http-router.md) — full routing with headers and query params
- [HTTP Matching](../domains/http.md) — all inputs, config types, and the Gateway API compiler
