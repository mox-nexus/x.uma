# Python Quick Start

Build an HTTP route matcher with `xuma` in 10 lines.

## Install

```bash
uv add xuma
```

Requires Python 3.12+. The only runtime dependency is `google-re2` for linear-time regex.

## Your First Matcher

Match requests by path prefix:

```python
from xuma import Matcher, FieldMatcher, SinglePredicate, Action, PrefixMatcher
from xuma.http import HttpRequest, PathInput

# Build a predicate: path starts with /api
predicate = SinglePredicate(
    input=PathInput(),
    matcher=PrefixMatcher("/api"),
)

# Build the matcher tree
matcher = Matcher(
    matcher_list=(
        FieldMatcher(predicate=predicate, on_match=Action("api_backend")),
    ),
    on_no_match=Action("default_backend"),
)

# Evaluate
request = HttpRequest(method="GET", raw_path="/api/users")
assert matcher.evaluate(request) == "api_backend"

# No match falls through
other = HttpRequest(method="GET", raw_path="/other")
assert matcher.evaluate(other) == "default_backend"
```

`Matcher` takes a tuple of `FieldMatcher`s (tried in order) and an optional fallback. First match wins.

## The Gateway API Compiler

Manual predicate construction is explicit but verbose. The HTTP domain ships a compiler that builds matchers from Gateway API config:

```python
from xuma.http import (
    HttpRouteMatch,
    HttpPathMatch,
    HttpRequest,
    compile_route_matches,
)

# Declarative config
routes = [
    HttpRouteMatch(
        path=HttpPathMatch(type="PathPrefix", value="/api"),
        method="GET",
    ),
    HttpRouteMatch(
        path=HttpPathMatch(type="PathPrefix", value="/admin"),
        method="POST",
    ),
]

# One call compiles all routes
matcher = compile_route_matches(
    matches=routes,
    action="allowed",
    on_no_match="denied",
)

assert matcher.evaluate(HttpRequest(method="GET", raw_path="/api/users")) == "allowed"
assert matcher.evaluate(HttpRequest(method="DELETE", raw_path="/api/users")) == "denied"
```

Within a single `HttpRouteMatch`, all conditions are ANDed. Multiple routes are ORed. First match wins.

## Adding Header Conditions

```python
from xuma.http import HttpHeaderMatch

route = HttpRouteMatch(
    path=HttpPathMatch(type="PathPrefix", value="/api"),
    method="POST",
    headers=[
        HttpHeaderMatch(
            type="RegularExpression",
            name="authorization",
            value=r"^Bearer .+$",
        ),
    ],
)
```

Regex uses `google-re2` — linear time, no ReDoS vulnerability.

## Custom Action Types

The action type can be any Python object:

```python
from dataclasses import dataclass

@dataclass(frozen=True)
class RouteAction:
    target: str
    weight: int

matcher = compile_route_matches(
    matches=routes,
    action=RouteAction(target="api-service", weight=100),
    on_no_match=None,
)
```

## Safety

- **ReDoS protection** — `google-re2` guarantees linear-time regex matching.
- **Immutable** — all types are `frozen=True` dataclasses.
- **Depth limits** — nested matchers capped at 32 levels.
- **Fail-closed** — missing headers or query params return `None` from `DataInput`, which makes the predicate evaluate to `False`.

## Next Steps

- [The Matching Pipeline](../concepts/pipeline.md) — how data flows through the matcher
- [Build an HTTP Router](../tutorials/http-router.md) — full routing with headers and query params
- [HTTP Matching](../domains/http.md) — all inputs, config types, and the compiler
