# HTTP Matching

The HTTP domain provides inputs, config types, and a compiler for matching HTTP requests. It ships with all five implementations.

## Inputs

Four `DataInput` types extract fields from HTTP requests:

| Input | Extracts | Returns |
|-------|----------|---------|
| `PathInput` | Request path | `string` |
| `MethodInput` | HTTP method | `string` |
| `HeaderInput(name)` | Header value | `string` or `null` |
| `QueryParamInput(name)` | Query parameter | `string` or `null` |

`HeaderInput` and `QueryParamInput` return `null` when the field is absent. The None-to-false rule ensures missing fields never match.

## The Gateway API Compiler

The compiler transforms declarative config into matcher trees. It mirrors [Kubernetes Gateway API](https://gateway-api.sigs.k8s.io/) `HTTPRouteMatch` semantics:

- Within a single `HttpRouteMatch`, all conditions are **ANDed**
- Multiple `HttpRouteMatch` entries are **ORed** (first match wins)

### Python

```python
from xuma.http import HttpRouteMatch, HttpPathMatch, HttpHeaderMatch, compile_route_matches

routes = [
    HttpRouteMatch(
        path=HttpPathMatch(type="PathPrefix", value="/api"),
        method="GET",
        headers=[
            HttpHeaderMatch(type="Exact", name="accept", value="application/json"),
        ],
    ),
    HttpRouteMatch(
        path=HttpPathMatch(type="PathPrefix", value="/health"),
    ),
]

matcher = compile_route_matches(routes, "allowed", on_no_match="denied")
```

### TypeScript

```typescript
import { compileRouteMatches } from "xuma/http";
import type { HttpRouteMatch } from "xuma/http";

const routes: HttpRouteMatch[] = [
  {
    path: { type: "PathPrefix", value: "/api" },
    method: "GET",
    headers: [
      { type: "Exact", name: "accept", value: "application/json" },
    ],
  },
  {
    path: { type: "PathPrefix", value: "/health" },
  },
];

const matcher = compileRouteMatches(routes, "allowed", "denied");
```

### Rust

```rust,ignore
use rumi::prelude::*;
use rumi_http::simple::*;

let request = HttpRequest::builder()
    .method("GET")
    .path("/api/users")
    .header("accept", "application/json")
    .build();
```

Rust has two layers. The **simple** module (`HttpRequest`, `SimplePathInput`, etc.) works everywhere, including WASM. The **ext-proc** module (behind the `ext-proc` feature) provides `HttpMessage` for Envoy external processing integration with full k8s Gateway API types.

## Config Types

### HttpPathMatch

| Field | Type | Values |
|-------|------|--------|
| `type` | string | `"Exact"`, `"PathPrefix"`, `"RegularExpression"` |
| `value` | string | The pattern to match |

`PathPrefix` matches if the path starts with the value. `Exact` requires an exact match. `RegularExpression` uses RE2 syntax (linear-time guarantee).

### HttpHeaderMatch

| Field | Type | Values |
|-------|------|--------|
| `type` | string | `"Exact"`, `"RegularExpression"` |
| `name` | string | Header name (case-insensitive) |
| `value` | string | The pattern to match |

### HttpQueryParamMatch

| Field | Type | Values |
|-------|------|--------|
| `type` | string | `"Exact"`, `"RegularExpression"` |
| `name` | string | Query parameter name |
| `value` | string | The pattern to match |

### HttpRouteMatch

| Field | Type | Required |
|-------|------|----------|
| `path` | `HttpPathMatch` | No |
| `method` | string | No |
| `headers` | list of `HttpHeaderMatch` | No |
| `query_params` | list of `HttpQueryParamMatch` | No |

All fields are optional. An empty `HttpRouteMatch` matches every request (catch-all).

## Registry Type URLs

When using the config path (JSON/YAML → Registry → Matcher), these type URLs are registered:

| Type URL | Input |
|----------|-------|
| `xuma.http.v1.PathInput` | Path extraction |
| `xuma.http.v1.MethodInput` | Method extraction |
| `xuma.http.v1.HeaderInput` | Header extraction (config: `{"name": "..."}`) |
| `xuma.http.v1.QueryParamInput` | Query param extraction (config: `{"name": "..."}`) |

## Manual Construction

You can build HTTP matchers manually instead of using the compiler:

```python
from xuma import Matcher, FieldMatcher, SinglePredicate, And, Action
from xuma import PrefixMatcher, ExactMatcher
from xuma.http import HttpRequest, PathInput, MethodInput, HeaderInput

# GET /api/* with JSON accept header
predicate = And((
    SinglePredicate(PathInput(), PrefixMatcher("/api")),
    SinglePredicate(MethodInput(), ExactMatcher("GET")),
    SinglePredicate(HeaderInput("accept"), ExactMatcher("application/json")),
))

matcher = Matcher(
    matcher_list=(
        FieldMatcher(predicate=predicate, on_match=Action("api_json")),
    ),
    on_no_match=Action("not_found"),
)

request = HttpRequest(method="GET", raw_path="/api/users",
                      headers={"accept": "application/json"})
assert matcher.evaluate(request) == "api_json"
```

The compiler is syntactic sugar. Manual construction gives full control over predicate trees and nesting.

## Rust: Simple vs Ext-Proc

| Feature | Simple | Ext-Proc |
|---------|--------|----------|
| Context type | `HttpRequest` | `HttpMessage` |
| Dependencies | None | k8s-gateway-api, k8s-openapi, envoy types |
| WASM compatible | Yes | No |
| Use case | Testing, bindings, simple routing | Production Envoy integration |

The simple module is always available. The ext-proc module requires `features = ["ext-proc"]` (enabled by default in `rumi-http`).

## Next

- [Build an HTTP Router](../tutorials/http-router.md) — step-by-step tutorial
- [The Matching Pipeline](../concepts/pipeline.md) — how HTTP inputs flow through the engine
- [Config Format](../reference/config.md) — JSON/YAML config for HTTP matchers
