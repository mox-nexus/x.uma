# Python Quick Start

Build an HTTP route matcher in 10 lines.

x.uma's Python implementation (`puma`) translates Gateway API route configuration into efficient runtime matchers. Routes defined at config time become compiled trees evaluated at request time.

## Install

Add `puma` to your project:

```bash
uv add puma
```

If you're not using `uv`, `pip` works too:

```bash
pip install puma
```

**Requires Python 3.12+** for PEP 695 type parameter syntax (`class Matcher[Ctx, A]`).

## Your First Matcher

Match GET requests to `/api/*` and POST requests to `/admin/*`:

```python
from puma.http import (
    HttpRouteMatch,
    HttpPathMatch,
    HttpRequest,
    compile_route_matches,
)

# Define routes using Gateway API syntax
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

# Compile routes into a matcher
matcher = compile_route_matches(
    matches=routes,
    action="allowed",      # when any route matches
    on_no_match="denied",  # when no routes match
)

# Evaluate against requests
request = HttpRequest(method="GET", raw_path="/api/users")
result = matcher.evaluate(request)
assert result == "allowed"

request = HttpRequest(method="DELETE", raw_path="/api/users")
result = matcher.evaluate(request)
assert result == "denied"  # DELETE not in routes
```

The `compile_route_matches` function is the high-level API. It takes a list of `HttpRouteMatch` configs and produces a `Matcher[HttpRequest, A]` that runs in microseconds.

## How Compilation Works

Gateway API route configuration is declarative. You specify what to match, not how to evaluate it:

```python
from puma.http import HttpRouteMatch, HttpPathMatch, HttpHeaderMatch

route = HttpRouteMatch(
    path=HttpPathMatch(type="PathPrefix", value="/api"),
    method="GET",
    headers=[
        HttpHeaderMatch(
            type="Exact",
            name="content-type",
            value="application/json",
        ),
    ],
)
```

All conditions within a single `HttpRouteMatch` are ANDed together. When you pass multiple `HttpRouteMatch` entries to `compile_route_matches`, they are ORed.

The compiler produces a tree of predicates:

```
Matcher
└── FieldMatcher
    └── Or
        ├── And(path=/api, method=GET, header=content-type)
        └── And(path=/admin, method=POST)
```

At evaluation time, the tree walks first-match-wins until a predicate succeeds.

## Under the Hood: Manual Construction

The Gateway API compiler is syntactic sugar. Here's what it generates:

```python
from puma import Matcher, FieldMatcher, Action, SinglePredicate, And
from puma import ExactMatcher, PrefixMatcher
from puma.http import HttpRequest, PathInput, MethodInput

# Manual construction of the same matcher
matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=And(
                predicates=(
                    SinglePredicate(
                        input=PathInput(),
                        matcher=PrefixMatcher("/api"),
                    ),
                    SinglePredicate(
                        input=MethodInput(),
                        matcher=ExactMatcher("GET"),
                    ),
                )
            ),
            on_match=Action("allowed"),
        ),
        FieldMatcher(
            predicate=SinglePredicate(
                input=PathInput(),
                matcher=PrefixMatcher("/admin"),
            ),
            on_match=Action("allowed"),
        ),
    ),
    on_no_match=Action("denied"),
)
```

This is verbose but explicit. You control the exact tree structure.

**When to use manual construction:**
- Building matchers programmatically from non-Gateway-API configs
- Implementing custom `DataInput` or `InputMatcher` protocols
- Debugging compilation behavior

**When to use the compiler:**
- Standard HTTP routing (99% of use cases)
- Gateway API configurations from Kubernetes
- Less code, same performance

## The HttpRequest Context

`HttpRequest` is a frozen dataclass with parsed query parameters and lowercased headers:

```python
from puma.http import HttpRequest

# Query string parsed from raw_path
request = HttpRequest(
    method="GET",
    raw_path="/search?q=hello&lang=en",
    headers={"Content-Type": "application/json"},
)

assert request.path == "/search"
assert request.query_param("q") == "hello"
assert request.query_param("lang") == "en"

# Headers are case-insensitive
assert request.header("content-type") == "application/json"
assert request.header("Content-Type") == "application/json"
```

The query string is parsed once at construction time (O(n) scan). Every `query_param()` lookup is O(1) dictionary access.

Headers are stored lowercased to avoid repeated string operations during matching.

## Actions: Beyond Strings

The examples use `str` actions for simplicity. In production, actions are often dataclasses or enums:

```python
from dataclasses import dataclass
from enum import Enum

class RouteTarget(Enum):
    API_SERVICE = "api-service"
    ADMIN_SERVICE = "admin-service"

@dataclass(frozen=True)
class RouteAction:
    target: RouteTarget
    weight: int
    timeout_ms: int

matcher = compile_route_matches(
    matches=routes,
    action=RouteAction(
        target=RouteTarget.API_SERVICE,
        weight=100,
        timeout_ms=5000,
    ),
    on_no_match=None,  # return None when no match
)
```

The action type can be any Python object. puma returns it by reference when a match succeeds.

## Validation and Safety

Matchers enforce safety constraints:

- **Depth limit**: Nested matchers cannot exceed 32 levels (prevents stack overflow)
- **Immutability**: All types are `frozen=True` dataclasses (no accidental mutation)
- **Type safety**: Type checkers like `mypy` catch mismatches at analysis time

Check depth limits with `validate()`:

```python
try:
    matcher.validate()
    print("Matcher is valid")
except ValueError as e:
    print(f"Validation failed: {e}")
```

The Gateway API compiler produces valid trees. Manual construction can violate depth limits if you nest `NestedMatcher` recursively.

## Performance Notes

At 200 routing rules, `puma` evaluates worst-case (last rule matches) in 20 microseconds on Apple M1 Max. That's 50,000 requests per second per core.

Python's GIL limits parallelism, but the matcher is thread-safe:

```python
import threading

def worker(matcher, request):
    result = matcher.evaluate(request)
    print(f"Result: {result}")

threads = [
    threading.Thread(target=worker, args=(matcher, request))
    for _ in range(10)
]

for t in threads:
    t.start()
for t in threads:
    t.join()
```

Each thread acquires the GIL during `evaluate()`. If you need true parallelism, consider:

- **Multiple processes** with `multiprocessing` (one matcher per process)
- **puma-crusty** (Rust core via PyO3, releases GIL during evaluation)

For ReDoS protection with untrusted regex input, use `puma-crusty` instead of pure `puma`. See [ReDoS Protection](../performance/redos.md).

## Integration Examples

### FastAPI Middleware

```python
from fastapi import FastAPI, Request, Response
from puma.http import HttpRequest, compile_route_matches

app = FastAPI()
matcher = compile_route_matches(...)

@app.middleware("http")
async def route_matcher_middleware(request: Request, call_next):
    req = HttpRequest(
        method=request.method,
        raw_path=str(request.url.path),
        headers=dict(request.headers),
    )

    action = matcher.evaluate(req)

    if action == "denied":
        return Response("Access denied", status_code=403)

    return await call_next(request)
```

### Flask Before Request

```python
from flask import Flask, request, abort
from puma.http import HttpRequest, compile_route_matches

app = Flask(__name__)
matcher = compile_route_matches(...)

@app.before_request
def check_route():
    req = HttpRequest(
        method=request.method,
        raw_path=request.path,
        headers=dict(request.headers),
    )

    action = matcher.evaluate(req)

    if action == "denied":
        abort(403)
```

## Next Steps

- [Build an HTTP Router](../tutorials/http-router.md) — full routing patterns
- [Predicate Composition](../concepts/predicates.md) — AND/OR/NOT logic
- [Benchmark Results](../performance/benchmarks.md) — performance deep dive
- [Python API Reference](../reference/python.md) — complete type documentation
