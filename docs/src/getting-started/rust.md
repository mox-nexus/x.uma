# Rust Quick Start

Build an HTTP route matcher in 10 lines.

x.uma's Rust implementation (`rumi`) translates Gateway API route configuration into efficient runtime matchers. Routes defined at config time become compiled trees evaluated at request time.

## Install

Add `rumi-http` to your `Cargo.toml`:

```toml
[dependencies]
rumi = "0.1"
rumi-http = "0.1"
```

The HTTP extension (`rumi-http`) brings in the core (`rumi`) as a transitive dependency.

## Your First Matcher

Match GET requests to `/api/*` and POST requests to `/admin/*`:

```rust
use rumi_http::prelude::*;

fn main() {
    // Define routes using Gateway API syntax
    let routes = vec![
        HttpRouteMatch {
            path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
            method: Some(HttpMethod::Get),
            ..Default::default()
        },
        HttpRouteMatch {
            path: Some(HttpPathMatch::PathPrefix { value: "/admin".into() }),
            method: Some(HttpMethod::Post),
            ..Default::default()
        },
    ];

    // Compile routes into a matcher
    let matcher = compile_route_matches(
        &routes,
        "allowed",      // action when any route matches
        Some("denied"), // action when no routes match
    );

    // Evaluate against requests
    let request = HttpRequest::builder()
        .method("GET")
        .path("/api/users")
        .build();

    let result = matcher.evaluate(&request);
    assert_eq!(result, Some(&"allowed"));

    let request = HttpRequest::builder()
        .method("DELETE")
        .path("/api/users")
        .build();

    let result = matcher.evaluate(&request);
    assert_eq!(result, Some(&"denied")); // DELETE not in routes
}
```

The `compile_route_matches` function is the high-level API. It takes a list of `HttpRouteMatch` configs and produces a `Matcher<HttpRequest, A>` that runs in microseconds.

## How Compilation Works

Gateway API route configuration is declarative. You specify what to match, not how to evaluate it:

```rust
let route = HttpRouteMatch {
    path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
    method: Some(HttpMethod::Get),
    headers: Some(vec![
        HttpHeaderMatch::Exact {
            name: "content-type".into(),
            value: "application/json".into(),
        },
    ]),
    ..Default::default()
};
```

All conditions within a single `HttpRouteMatch` are ANDed together. When you pass multiple `HttpRouteMatch` entries to `compile_route_matches`, they are ORed.

The compiler produces a tree of predicates:

```
Matcher
└── FieldMatcher
    └── Predicate::Or
        ├── And(path=/api, method=GET, header=content-type)
        └── And(path=/admin, method=POST)
```

At evaluation time, the tree walks first-match-wins until a predicate succeeds.

## Under the Hood: Manual Construction

The Gateway API compiler is syntactic sugar. Here's what it generates:

```rust
use rumi_http::prelude::*;

// Manual construction of the same matcher
let matcher = Matcher::new(
    vec![
        FieldMatcher::new(
            Predicate::And(vec![
                Predicate::Single(SinglePredicate::new(
                    Box::new(SimplePathInput),
                    Box::new(PrefixMatcher::new("/api")),
                )),
                Predicate::Single(SinglePredicate::new(
                    Box::new(SimpleMethodInput),
                    Box::new(ExactMatcher::new("GET")),
                )),
            ]),
            OnMatch::Action("allowed"),
        ),
        FieldMatcher::new(
            Predicate::Single(SinglePredicate::new(
                Box::new(SimplePathInput),
                Box::new(PrefixMatcher::new("/admin")),
            )),
            OnMatch::Action("allowed"),
        ),
    ],
    Some(OnMatch::Action("denied")),
);
```

This is verbose but explicit. You control the exact tree structure.

**When to use manual construction:**
- Building matchers programmatically from non-Gateway-API configs
- Implementing custom `DataInput` or `InputMatcher` types
- Debugging compilation behavior

**When to use the compiler:**
- Standard HTTP routing (99% of use cases)
- Gateway API configurations from Kubernetes
- Less code, same performance

## Matching Against Real Requests

The examples above use `HttpRequest`, a lightweight test context. Production systems process gRPC `ProcessingRequest` messages from Envoy's external processor protocol.

Convert `ProcessingRequest` to `HttpMessage` for O(1) lookups:

```rust
use rumi_http::prelude::*;

// Production: indexed context from ext_proc
let processing_req: ProcessingRequest = /* from Envoy */;
let message = HttpMessage::from(&processing_req);

let result = matcher.evaluate(&message);
```

`HttpMessage` pre-indexes headers, query parameters, and pseudo-headers at construction time. Every `DataInput::get()` call becomes a `HashMap` lookup instead of a linear scan.

For more on the ext_proc integration, see [Build an HTTP Router](../tutorials/http-router.md).

## Actions: Beyond Strings

The examples use `&str` actions for simplicity. In production, actions are often enums or structs:

```rust
#[derive(Debug, Clone, PartialEq)]
enum RouteAction {
    Forward { backend: String, weight: u32 },
    Redirect { location: String, status: u16 },
    Deny { reason: String },
}

let matcher = compile_route_matches(
    &routes,
    RouteAction::Forward {
        backend: "api-service".into(),
        weight: 100,
    },
    Some(RouteAction::Deny {
        reason: "no route matched".into(),
    }),
);
```

The action type must be `Clone + Send + Sync + 'static`. Beyond that, use whatever makes sense for your domain.

## Validation and Safety

Matchers enforce safety constraints:

- **Depth limit**: Nested matchers cannot exceed 32 levels (prevents stack overflow)
- **ReDoS protection**: `regex` crate guarantees linear-time matching (no exponential backtracking)
- **Type safety**: Invalid matcher trees fail at compile time, not runtime

Call `matcher.validate()` to check depth limits:

```rust
match matcher.validate() {
    Ok(()) => println!("Matcher is valid"),
    Err(MatcherError::DepthExceeded { max, actual }) => {
        eprintln!("Matcher too deep: {} > {}", actual, max);
    }
}
```

The Gateway API compiler produces valid trees. Manual construction can violate depth limits if you nest `OnMatch::Matcher` recursively.

## Performance Notes

At 200 routing rules, `rumi` evaluates worst-case (last rule matches) in 3.5 microseconds on Apple M1 Max. That's 285,000 requests per second per core.

The matcher is thread-safe (`Send + Sync`). Share one instance across threads:

```rust
use std::sync::Arc;

let matcher = Arc::new(compile_route_matches(&routes, "allowed", None));

// Clone the Arc, not the matcher
let m = matcher.clone();
std::thread::spawn(move || {
    let result = m.evaluate(&request);
});
```

`Arc` adds one atomic reference count operation. The matcher itself has no internal mutability and requires no locking.

## Next Steps

- [Build an HTTP Router](../tutorials/http-router.md) — full ext_proc integration
- [Predicate Composition](../concepts/predicates.md) — AND/OR/NOT logic
- [Benchmark Results](../performance/benchmarks.md) — performance deep dive
- [Rust API Reference](../reference/rust.md) — complete type documentation
