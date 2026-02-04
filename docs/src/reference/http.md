# HTTP Domain Reference

`rumi-http` provides HTTP request matching for Envoy ext_proc.

## Architecture

```
Gateway API HttpRouteMatch (config)
        ↓ compile()
rumi Matcher<ProcessingRequest, A>
        ↓ evaluate()
ext_proc ProcessingRequest (runtime)
```

**Two layers:**
- **User API**: Gateway API `HttpRouteMatch` — human-friendly config
- **Data Plane API**: Envoy `ProcessingRequest` — wire protocol

## Quick Start

```rust
use rumi_http::prelude::*;

// Config time: compile Gateway API match to rumi matcher
let route_match = HttpRouteMatch {
    path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
    method: Some("GET".into()),
    ..Default::default()
};
let matcher = route_match.compile("api_backend");

// Runtime: evaluate against ext_proc ProcessingRequest
let result = matcher.evaluate(&processing_request);
assert_eq!(result, Some("api_backend"));
```

## DataInputs for ProcessingRequest

| Input | Extracts | Example |
|-------|----------|---------|
| `PathInput` | `:path` pseudo-header (without query) | `/api/users` |
| `MethodInput` | `:method` pseudo-header | `GET` |
| `HeaderInput::new("x-api-key")` | Named header | `Bearer ...` |
| `QueryParamInput::new("page")` | Query parameter | `1` |
| `SchemeInput` | `:scheme` pseudo-header | `https` |
| `AuthorityInput` | `:authority` pseudo-header | `api.example.com` |

## Gateway API Match Types

### Path Matching

```rust
// Exact match
HttpPathMatch::Exact { value: "/api/v1/users".into() }

// Prefix match
HttpPathMatch::PathPrefix { value: "/api".into() }

// Regex match
HttpPathMatch::RegularExpression { value: r"^/api/v\d+/.*".into() }
```

### Header Matching

```rust
// Exact header value
HttpHeaderMatch::Exact {
    name: "content-type".into(),
    value: "application/json".into()
}

// Regex header value
HttpHeaderMatch::RegularExpression {
    name: "authorization".into(),
    value: r"^Bearer .+".into()
}
```

### Query Parameter Matching

```rust
// Exact param value
HttpQueryParamMatch::Exact {
    name: "version".into(),
    value: "2".into()
}

// Regex param value
HttpQueryParamMatch::RegularExpression {
    name: "id".into(),
    value: r"^\d+$".into()
}
```

## Match Semantics

Per Gateway API spec:
- **Within a match**: All conditions are ANDed
- **Multiple matches**: ORed together

```rust
// This matches: GET /api/* with x-version header
let route_match = HttpRouteMatch {
    path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
    method: Some("GET".into()),
    headers: Some(vec![
        HttpHeaderMatch::Exact {
            name: "x-version".into(),
            value: "2".into()
        }
    ]),
    ..Default::default()
};
```

## Compiling Multiple Matches

```rust
use rumi_http::compile_route_matches;

let matches = vec![
    HttpRouteMatch {
        path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
        ..Default::default()
    },
    HttpRouteMatch {
        path: Some(HttpPathMatch::PathPrefix { value: "/health".into() }),
        ..Default::default()
    },
];

// Multiple matches are ORed
let matcher = compile_route_matches(&matches, "backend", None);
```

## Testing with SimpleHttpRequest

For unit tests without constructing full `ProcessingRequest`:

```rust
use rumi_http::prelude::*;

let request = HttpRequest::builder()
    .method("POST")
    .path("/api/users")
    .header("Content-Type", "application/json")
    .query_param("page", "1")
    .build();

// Use Simple*Input types for matching
let method_input = SimpleMethodInput;
assert_eq!(method_input.get(&request), MatchingData::String("POST".into()));
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `k8s-gateway-api` | Gateway API types |
| `envoy-grpc-ext-proc` | ext_proc ProcessingRequest types |

## See Also

- [Gateway API HTTPRoute Spec](https://gateway-api.sigs.k8s.io/reference/spec/#gateway.networking.k8s.io/v1.HTTPRouteMatch)
- [Envoy ext_proc Documentation](https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_filters/ext_proc_filter)
