# Build an HTTP Router

Route HTTP requests to handlers in under 50 lines. Start simple, add complexity progressively.

## Step 1: Three Path Prefixes

Route requests based on path prefix:
- `/api/*` → `"api_backend"`
- `/admin/*` → `"admin_backend"`
- `/health` → `"health_check"`

**Python:**
```python
from puma.http import HttpRouteMatch, HttpPathMatch, compile_route_matches, HttpRequest

# Define routes with Gateway API syntax
routes = [
    HttpRouteMatch(path=HttpPathMatch(type="PathPrefix", value="/api")),
    HttpRouteMatch(path=HttpPathMatch(type="PathPrefix", value="/admin")),
    HttpRouteMatch(path=HttpPathMatch(type="Exact", value="/health")),
]

# Compile to a matcher (returns action or None)
matcher = compile_route_matches(routes, action="matched", on_no_match="not_found")

# Evaluate requests
assert matcher.evaluate(HttpRequest(raw_path="/api/users")) == "matched"
assert matcher.evaluate(HttpRequest(raw_path="/admin/config")) == "matched"
assert matcher.evaluate(HttpRequest(raw_path="/health")) == "matched"
assert matcher.evaluate(HttpRequest(raw_path="/other")) == "not_found"
```

**Rust:**
```rust
use rumi_http::{HttpRouteMatch, HttpPathMatch, compile_route_matches, HttpMessage};

let routes = vec![
    HttpRouteMatch {
        path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
        ..Default::default()
    },
    HttpRouteMatch {
        path: Some(HttpPathMatch::PathPrefix { value: "/admin".into() }),
        ..Default::default()
    },
    HttpRouteMatch {
        path: Some(HttpPathMatch::Exact { value: "/health".into() }),
        ..Default::default()
    },
];

let matcher = compile_route_matches(&routes, "matched", Some("not_found"));

assert_eq!(matcher.evaluate(&http_message), Some("matched"));
```

Gateway API `HttpRouteMatch` is the config layer. The compiler builds the matcher tree for you.

## Step 2: Add Method Matching

Route GET and POST differently:
- `GET /api/*` → `"api_read"`
- `POST /api/*` → `"api_write"`

**Python:**
```python
from puma.http import HttpRouteMatch, HttpPathMatch, HttpRequest

routes = [
    HttpRouteMatch(
        path=HttpPathMatch(type="PathPrefix", value="/api"),
        method="GET"
    ),
    HttpRouteMatch(
        path=HttpPathMatch(type="PathPrefix", value="/api"),
        method="POST"
    ),
]

# Different actions per route
matchers = [route.compile(action) for route, action in zip(routes, ["api_read", "api_write"])]

# Combine into one matcher with on_no_match
from puma import Matcher, FieldMatcher, Or, NestedMatcher, Action

combined = Matcher(
    matcher_list=tuple(
        FieldMatcher(
            predicate=route.to_predicate(),
            on_match=Action(action)
        )
        for route, action in zip(routes, ["api_read", "api_write"])
    ),
    on_no_match=Action("method_not_allowed")
)

# Test
assert combined.evaluate(HttpRequest(method="GET", raw_path="/api/users")) == "api_read"
assert combined.evaluate(HttpRequest(method="POST", raw_path="/api/users")) == "api_write"
assert combined.evaluate(HttpRequest(method="DELETE", raw_path="/api/users")) == "method_not_allowed"
```

Within a single `HttpRouteMatch`, all conditions are ANDed (path AND method). Multiple `HttpRouteMatch` entries are ORed.

**Rust:**
```rust
use rumi_http::{HttpRouteMatch, HttpPathMatch, HttpMessage};

let routes = vec![
    HttpRouteMatch {
        path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
        method: Some("GET".into()),
        ..Default::default()
    },
    HttpRouteMatch {
        path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
        method: Some("POST".into()),
        ..Default::default()
    },
];

// Build separate matchers or use compile_route_matches with action types
```

## Step 3: Add Header Conditions

Require authentication for POST requests:
- `POST /api/*` with `Authorization: Bearer *` → `"api_authenticated"`
- `POST /api/*` without auth → `"unauthorized"`

**Python:**
```python
from puma.http import HttpRouteMatch, HttpPathMatch, HttpHeaderMatch, HttpRequest

auth_route = HttpRouteMatch(
    path=HttpPathMatch(type="PathPrefix", value="/api"),
    method="POST",
    headers=[
        HttpHeaderMatch(type="RegularExpression", name="authorization", value=r"^Bearer .+$")
    ]
)

noauth_route = HttpRouteMatch(
    path=HttpPathMatch(type="PathPrefix", value="/api"),
    method="POST"
)

# Order matters: more specific (auth required) comes first
routes = [auth_route, noauth_route]
actions = ["api_authenticated", "unauthorized"]

matcher = Matcher(
    matcher_list=tuple(
        FieldMatcher(predicate=route.to_predicate(), on_match=Action(action))
        for route, action in zip(routes, actions)
    ),
    on_no_match=Action("not_found")
)

# Test
request = HttpRequest(
    method="POST",
    raw_path="/api/users",
    headers={"authorization": "Bearer token123"}
)
assert matcher.evaluate(request) == "api_authenticated"

request = HttpRequest(method="POST", raw_path="/api/users", headers={})
assert matcher.evaluate(request) == "unauthorized"
```

First-match-wins semantics: the auth route matches first if the header is present. If it doesn't match, evaluation continues to the noauth route.

**Rust:**
```rust
use rumi_http::{HttpRouteMatch, HttpPathMatch, HttpHeaderMatch};

let auth_route = HttpRouteMatch {
    path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
    method: Some("POST".into()),
    headers: Some(vec![
        HttpHeaderMatch::RegularExpression {
            name: "authorization".into(),
            value: r"^Bearer .+$".into(),
        }
    ]),
    ..Default::default()
};
```

## Step 4: Add Query Parameter Matching

Route based on query parameters:
- `/search?version=2` → `"search_v2"`
- `/search?version=1` → `"search_v1"`
- `/search` (no version) → `"search_latest"`

**Python:**
```python
from puma.http import HttpRouteMatch, HttpPathMatch, HttpQueryParamMatch, HttpRequest

routes = [
    HttpRouteMatch(
        path=HttpPathMatch(type="Exact", value="/search"),
        query_params=[
            HttpQueryParamMatch(type="Exact", name="version", value="2")
        ]
    ),
    HttpRouteMatch(
        path=HttpPathMatch(type="Exact", value="/search"),
        query_params=[
            HttpQueryParamMatch(type="Exact", name="version", value="1")
        ]
    ),
    HttpRouteMatch(
        path=HttpPathMatch(type="Exact", value="/search")
    ),
]

actions = ["search_v2", "search_v1", "search_latest"]

matcher = Matcher(
    matcher_list=tuple(
        FieldMatcher(predicate=route.to_predicate(), on_match=Action(action))
        for route, action in zip(routes, actions)
    )
)

# Test
assert matcher.evaluate(HttpRequest(raw_path="/search?version=2")) == "search_v2"
assert matcher.evaluate(HttpRequest(raw_path="/search?version=1")) == "search_v1"
assert matcher.evaluate(HttpRequest(raw_path="/search")) == "search_latest"
assert matcher.evaluate(HttpRequest(raw_path="/search?other=param")) == "search_latest"
```

Query parameters are parsed from `raw_path` automatically. Order matters: version-specific routes come before the no-version route.

**Rust:**
```rust
use rumi_http::{HttpRouteMatch, HttpPathMatch, HttpQueryParamMatch};

let v2_route = HttpRouteMatch {
    path: Some(HttpPathMatch::Exact { value: "/search".into() }),
    query_params: Some(vec![
        HttpQueryParamMatch::Exact {
            name: "version".into(),
            value: "2".into(),
        }
    ]),
    ..Default::default()
};
```

## Step 5: Multiple Routes with Fallback

Combine everything into a production-ready router:

**Python:**
```python
from puma.http import (
    HttpRouteMatch, HttpPathMatch, HttpHeaderMatch, HttpQueryParamMatch,
    compile_route_matches, HttpRequest
)

# Define routes
api_auth = HttpRouteMatch(
    path=HttpPathMatch(type="PathPrefix", value="/api"),
    method="POST",
    headers=[HttpHeaderMatch(type="RegularExpression", name="authorization", value=r"^Bearer .+$")]
)

api_get = HttpRouteMatch(
    path=HttpPathMatch(type="PathPrefix", value="/api"),
    method="GET"
)

admin = HttpRouteMatch(
    path=HttpPathMatch(type="PathPrefix", value="/admin"),
    headers=[HttpHeaderMatch(type="Exact", name="x-admin-key", value="secret")]
)

health = HttpRouteMatch(
    path=HttpPathMatch(type="Exact", value="/health")
)

# Build matcher
routes = [api_auth, api_get, admin, health]
actions = ["api_write", "api_read", "admin_panel", "health_check"]

matcher = Matcher(
    matcher_list=tuple(
        FieldMatcher(predicate=route.to_predicate(), on_match=Action(action))
        for route, action in zip(routes, actions)
    ),
    on_no_match=Action("not_found")
)

# Test all cases
assert matcher.evaluate(
    HttpRequest(method="POST", raw_path="/api/users", headers={"authorization": "Bearer token"})
) == "api_write"

assert matcher.evaluate(
    HttpRequest(method="GET", raw_path="/api/users")
) == "api_read"

assert matcher.evaluate(
    HttpRequest(raw_path="/admin", headers={"x-admin-key": "secret"})
) == "admin_panel"

assert matcher.evaluate(HttpRequest(raw_path="/health")) == "health_check"

assert matcher.evaluate(HttpRequest(raw_path="/unknown")) == "not_found"
```

This is a complete HTTP router in 30 lines of config. No framework. No DSL. Pure Gateway API.

**Rust equivalent:**
```rust
use rumi::prelude::*;
use rumi_http::{HttpRouteMatch, HttpPathMatch, HttpHeaderMatch, HttpMessage};

let routes = vec![
    HttpRouteMatch {
        path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
        method: Some("POST".into()),
        headers: Some(vec![HttpHeaderMatch::RegularExpression {
            name: "authorization".into(),
            value: r"^Bearer .+$".into(),
        }]),
        ..Default::default()
    },
    HttpRouteMatch {
        path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
        method: Some("GET".into()),
        ..Default::default()
    },
    HttpRouteMatch {
        path: Some(HttpPathMatch::PathPrefix { value: "/admin".into() }),
        headers: Some(vec![HttpHeaderMatch::Exact {
            name: "x-admin-key".into(),
            value: "secret".into(),
        }]),
        ..Default::default()
    },
    HttpRouteMatch {
        path: Some(HttpPathMatch::Exact { value: "/health".into() }),
        ..Default::default()
    },
];

// Build matcher per route with different actions
// Or use custom action enum to distinguish routes
```

## What's Next

This tutorial showed the Gateway API compiler pattern. You can also build matchers manually for finer control:

**Manual matcher construction:**
```python
from puma import Matcher, FieldMatcher, SinglePredicate, PrefixMatcher, Action
from puma.http import PathInput

# Bypass the compiler, build the tree directly
matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/api")),
            on_match=Action("api_backend")
        ),
    )
)
```

Manual construction is more verbose but gives you full control over the matcher tree structure.

**Learn more:**
- [The Matching Pipeline](../concepts/pipeline.md) — How data flows through the matcher
- [Type Erasure and Ports](../concepts/type-erasure.md) — Why matchers are reusable
- [Predicate Composition](../concepts/predicates.md) — AND, OR, NOT in detail
- [First-Match-Wins Semantics](../concepts/semantics.md) — Evaluation order and fallbacks
- [Adding a Domain](../guides/adding-domain.md) — Create matchers for CloudEvents, gRPC, etc.

**Performance:**
- Matchers are immutable and thread-safe — share them across threads
- Regex compilation happens at construction time — no per-request overhead
- First-match-wins means early rules short-circuit evaluation
- For production workloads, consider `puma-crusty` (Rust-backed) for 10x+ speedup
