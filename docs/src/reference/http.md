# HTTP Domain Reference

HTTP request matching across all three x.uma implementations.

## Overview

All three implementations provide identical HTTP matching capabilities: Gateway API-style config types, a compiler that turns config into matchers, and DataInputs for HTTP contexts.

| Implementation | Package | Context Type | Compiler |
|----------------|---------|-------------|----------|
| **rumi** (Rust) | `rumi-http` | `HttpMessage` | `compile_route_matches()` |
| **puma** (Python) | `puma.http` | `HttpRequest` | `compile_route_matches()` |
| **bumi** (TypeScript) | `bumi/http` | `HttpRequest` | `compileRouteMatches()` |

## Architecture

Every implementation follows the same two-layer pattern:

```
Config layer:    HttpRouteMatch (human-friendly)
                        |
                   compile()
                        |
Engine layer:    Matcher<HttpContext, A>
                        |
                   evaluate()
                        |
Result:          A | null
```

**Rust has an extra layer** for data plane use: the config layer produces `Matcher<HttpMessage, A>`, where `HttpMessage` is an indexed view over Envoy's `ext_proc ProcessingRequest`. Python and TypeScript use a simpler `HttpRequest` context.

## Quick Start

All three languages use the same Gateway API config types:

**Rust:**
```rust
use rumi_http::prelude::*;

let route = HttpRouteMatch {
    path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
    method: Some("GET".into()),
    ..Default::default()
};
let matcher = route.compile("api_backend");
```

**Python:**
```python
from puma.http import HttpRouteMatch, compile_route_match

route = HttpRouteMatch(
    path=HttpPathMatch(type="PathPrefix", value="/api"),
    method="GET",
)
matcher = compile_route_match(route, "api_backend")
```

**TypeScript:**
```typescript
import { compileRouteMatch } from "bumi/http";

const matcher = compileRouteMatch(
    { path: { type: "PathPrefix", value: "/api" }, method: "GET" },
    "api_backend",
);
```

## DataInputs

Each implementation provides the same set of field extractors:

| Input | Extracts | Returns `null` when |
|-------|----------|-------------------|
| `PathInput` | URL path (without query string) | Path missing |
| `MethodInput` | HTTP method | Method missing |
| `HeaderInput(name)` | Header value by name (case-insensitive) | Header not present |
| `QueryParamInput(name)` | Query parameter value by name | Parameter not present |

**Rust-only** (for Envoy `ext_proc`):
| Input | Extracts |
|-------|----------|
| `SchemeInput` | `:scheme` pseudo-header |
| `AuthorityInput` | `:authority` pseudo-header |

## Gateway API Config Types

These types mirror the [Gateway API HTTPRouteMatch spec](https://gateway-api.sigs.k8s.io/reference/spec/#gateway.networking.k8s.io/v1.HTTPRouteMatch). All conditions within a single `HttpRouteMatch` are ANDed. Multiple `HttpRouteMatch` entries are ORed.

### `HttpPathMatch`

| Type | Match Logic |
|------|-------------|
| `Exact` | Path equals value exactly |
| `PathPrefix` | Path starts with value |
| `RegularExpression` | Path matches regex pattern |

### `HttpHeaderMatch`

| Type | Match Logic |
|------|-------------|
| `Exact` | Header value equals string exactly |
| `RegularExpression` | Header value matches regex pattern |

Header name lookup is always case-insensitive.

### `HttpQueryParamMatch`

| Type | Match Logic |
|------|-------------|
| `Exact` | Query param value equals string exactly |
| `RegularExpression` | Query param value matches regex pattern |

### `HttpRouteMatch`

Combines path, method, headers, and query parameters. All specified conditions must match (AND semantics).

**Rust:**
```rust
HttpRouteMatch {
    path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
    method: Some("GET".into()),
    headers: Some(vec![
        HttpHeaderMatch::Exact {
            name: "x-version".into(),
            value: "2".into(),
        },
    ]),
    query_params: None,
}
```

**Python:**
```python
HttpRouteMatch(
    path=HttpPathMatch(type="PathPrefix", value="/api"),
    method="GET",
    headers=[HttpHeaderMatch(type="Exact", name="x-version", value="2")],
)
```

**TypeScript:**
```typescript
{
    path: { type: "PathPrefix", value: "/api" },
    method: "GET",
    headers: [{ type: "Exact", name: "x-version", value: "2" }],
}
```

## Compiler Functions

### Single Route

Compile one `HttpRouteMatch` into a `Matcher`:

| Language | Function | Signature |
|----------|----------|-----------|
| Rust | `route_match.compile(action)` | `HttpRouteMatch → Matcher<HttpMessage, A>` |
| Python | `compile_route_match(route, action)` | `HttpRouteMatch → Matcher[HttpRequest, A]` |
| TypeScript | `compileRouteMatch(route, action)` | `HttpRouteMatch → Matcher<HttpRequest, A>` |

### Multiple Routes

Compile multiple `HttpRouteMatch` entries into a single `Matcher` with OR semantics:

| Language | Function |
|----------|----------|
| Rust | `compile_route_matches(&matches, action, fallback)` |
| Python | `compile_route_matches(matches, action, on_no_match)` |
| TypeScript | `compileRouteMatches(matches, action, onNoMatch)` |

**Empty matches array** creates a catch-all matcher (matches everything).

**Example (TypeScript):**
```typescript
import { compileRouteMatches, type HttpRouteMatch } from "bumi/http";

const apiRoute: HttpRouteMatch = {
    path: { type: "PathPrefix", value: "/api" },
    method: "GET",
};

const adminRoute: HttpRouteMatch = {
    headers: [{ type: "Exact", name: "x-admin", value: "true" }],
};

const matcher = compileRouteMatches(
    [apiRoute, adminRoute],
    "allowed",
    "denied",
);
```

## Context Types

### `HttpRequest` (Python, TypeScript)

Simplified HTTP request for application-level matching.

```python
# Python
from puma.http import HttpRequest

req = HttpRequest(
    method="GET",
    raw_path="/api/users?role=admin",
    headers={"content-type": "application/json"},
)

req.path              # "/api/users" (parsed from raw_path)
req.query_params      # {"role": "admin"} (parsed from raw_path)
req.header("Content-Type")  # "application/json" (case-insensitive)
```

```typescript
// TypeScript
import { HttpRequest } from "bumi/http";

const req = new HttpRequest(
    "GET",
    "/api/users?role=admin",
    { "Content-Type": "application/json" },
);

req.path;                    // "/api/users"
req.queryParam("role");      // "admin"
req.header("content-type");  // "application/json" (case-insensitive)
```

### `HttpMessage` (Rust only)

Indexed view over Envoy `ProcessingRequest` for data plane matching. Provides O(1) header and query parameter lookups via `HashMap`.

```rust
use rumi_http::{HttpMessage, SimpleHttpRequest};

// For testing: SimpleHttpRequest builder
let req = SimpleHttpRequest::builder()
    .method("GET")
    .path("/api/users")
    .header("content-type", "application/json")
    .build();

let message = HttpMessage::from(&req);
```

## Dependencies

| Implementation | Runtime Dependencies |
|----------------|---------------------|
| rumi-http | `k8s-gateway-api`, `envoy-grpc-ext-proc` |
| puma.http | None (zero dependencies) |
| bumi/http | None (zero runtime dependencies) |

## See Also

- [Rust API Reference](rust.md) — rumi core types
- [Python API Reference](python.md) — puma core + HTTP types
- [TypeScript API Reference](typescript.md) — bumi core + HTTP types
- [Build an HTTP Router](../tutorials/http-router.md) — Step-by-step tutorial
- [Gateway API HTTPRoute Spec](https://gateway-api.sigs.k8s.io/reference/spec/#gateway.networking.k8s.io/v1.HTTPRouteMatch)
