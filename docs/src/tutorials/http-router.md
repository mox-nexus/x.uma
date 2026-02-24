# Build an HTTP Router

This tutorial builds an HTTP route matcher step by step. You'll start with a single route and finish with a multi-route matcher that handles paths, methods, headers, and query parameters.

Examples are in Python. The same patterns apply to TypeScript and Rust — see the [Getting Started](../getting-started/python.md) guides for language-specific syntax.

## Step 1: Match a Single Path

Route requests starting with `/api` to the API backend:

```python
from xuma import Matcher, FieldMatcher, SinglePredicate, Action, PrefixMatcher
from xuma.http import HttpRequest, PathInput

matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/api")),
            on_match=Action("api_backend"),
        ),
    ),
    on_no_match=Action("not_found"),
)

assert matcher.evaluate(HttpRequest(raw_path="/api/users")) == "api_backend"
assert matcher.evaluate(HttpRequest(raw_path="/other")) == "not_found"
```

`PrefixMatcher("/api")` matches any path starting with `/api`. The `on_no_match` fallback catches everything else.

## Step 2: Add Method Matching

Only allow GET requests to the API:

```python
from xuma import And, ExactMatcher
from xuma.http import MethodInput

predicate = And((
    SinglePredicate(PathInput(), PrefixMatcher("/api")),
    SinglePredicate(MethodInput(), ExactMatcher("GET")),
))

matcher = Matcher(
    matcher_list=(
        FieldMatcher(predicate=predicate, on_match=Action("api_get")),
    ),
    on_no_match=Action("not_found"),
)

assert matcher.evaluate(HttpRequest(method="GET", raw_path="/api/users")) == "api_get"
assert matcher.evaluate(HttpRequest(method="POST", raw_path="/api/users")) == "not_found"
```

`And` combines conditions. All must be true. Short-circuits on the first false.

## Step 3: Multiple Routes

Add a health check endpoint:

```python
matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=And((
                SinglePredicate(PathInput(), PrefixMatcher("/api")),
                SinglePredicate(MethodInput(), ExactMatcher("GET")),
            )),
            on_match=Action("api_get"),
        ),
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), ExactMatcher("/health")),
            on_match=Action("health"),
        ),
    ),
    on_no_match=Action("not_found"),
)

assert matcher.evaluate(HttpRequest(method="GET", raw_path="/api/users")) == "api_get"
assert matcher.evaluate(HttpRequest(raw_path="/health")) == "health"
assert matcher.evaluate(HttpRequest(method="POST", raw_path="/api/users")) == "not_found"
```

Field matchers evaluate in order. First match wins. `/health` uses `ExactMatcher` — only the exact path matches, not `/health/check`.

## Step 4: Header Conditions

Require an authorization header for the API:

```python
from xuma.http import HeaderInput

api_predicate = And((
    SinglePredicate(PathInput(), PrefixMatcher("/api")),
    SinglePredicate(MethodInput(), ExactMatcher("GET")),
    SinglePredicate(HeaderInput("authorization"), PrefixMatcher("Bearer ")),
))

matcher = Matcher(
    matcher_list=(
        FieldMatcher(predicate=api_predicate, on_match=Action("api_authenticated")),
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), ExactMatcher("/health")),
            on_match=Action("health"),
        ),
    ),
    on_no_match=Action("unauthorized"),
)

# With valid auth header
request = HttpRequest(
    method="GET",
    raw_path="/api/users",
    headers={"authorization": "Bearer token123"},
)
assert matcher.evaluate(request) == "api_authenticated"

# Without auth header — HeaderInput returns None → predicate is false
request = HttpRequest(method="GET", raw_path="/api/users")
assert matcher.evaluate(request) == "unauthorized"
```

When the `authorization` header is missing, `HeaderInput` returns `None`. The None-to-false rule makes the predicate false without calling the matcher. Missing data never accidentally matches.

## Step 5: Use the Compiler

The Gateway API compiler builds the same matcher from declarative config:

```python
from xuma.http import (
    HttpRouteMatch, HttpPathMatch, HttpHeaderMatch,
    compile_route_matches, HttpRequest,
)

routes = [
    HttpRouteMatch(
        path=HttpPathMatch(type="PathPrefix", value="/api"),
        method="GET",
        headers=[
            HttpHeaderMatch(type="RegularExpression", name="authorization", value="^Bearer .+$"),
        ],
    ),
    HttpRouteMatch(
        path=HttpPathMatch(type="Exact", value="/health"),
    ),
]

matcher = compile_route_matches(routes, "allowed", on_no_match="denied")

# Authenticated API request
request = HttpRequest(
    method="GET",
    raw_path="/api/users",
    headers={"authorization": "Bearer token123"},
)
assert matcher.evaluate(request) == "allowed"

# Health check (no auth needed)
assert matcher.evaluate(HttpRequest(raw_path="/health")) == "allowed"

# Unauthenticated API request
assert matcher.evaluate(HttpRequest(method="GET", raw_path="/api/users")) == "denied"
```

The compiler:
- ANDs conditions within each `HttpRouteMatch`
- ORs multiple `HttpRouteMatch` entries
- Returns the first matching route's action

This is equivalent to the manual construction in Steps 1-4, with less boilerplate.

## Step 6: Custom Action Types

Use structured actions instead of strings:

```python
from dataclasses import dataclass

@dataclass(frozen=True)
class RouteAction:
    backend: str
    timeout_ms: int = 5000

routes = [
    HttpRouteMatch(
        path=HttpPathMatch(type="PathPrefix", value="/api"),
        method="GET",
    ),
]

matcher = compile_route_matches(
    routes,
    RouteAction(backend="api-service", timeout_ms=10000),
    on_no_match=RouteAction(backend="default", timeout_ms=1000),
)

result = matcher.evaluate(HttpRequest(method="GET", raw_path="/api/users"))
assert result.backend == "api-service"
assert result.timeout_ms == 10000
```

The generic `A` in `Matcher[HttpRequest, A]` accepts any type. Strings, dataclasses, enums — the engine doesn't interpret actions, it returns them.

## What You Built

A route matcher that:

1. Evaluates rules in order (first match wins)
2. Combines path, method, and header conditions with AND
3. Handles missing data safely (None-to-false)
4. Supports both manual construction and declarative config
5. Works with any action type

The same matcher can be built in TypeScript or Rust with identical semantics.

## Next

- [HTTP Matching](../domains/http.md) — full HTTP domain reference
- [First-Match-Wins Semantics](../concepts/semantics.md) — evaluation rules in depth
- [Predicate Composition](../concepts/predicates.md) — AND, OR, NOT
