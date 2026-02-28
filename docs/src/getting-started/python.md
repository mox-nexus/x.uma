# Python Quick Start

Build an HTTP route matcher with `xuma` (pure Python) or `xuma-crust` (Rust-backed).

## Install

```bash
# Pure Python
uv add xuma

# Rust-backed (faster, same API surface)
uv add xuma-crust
```

Requires Python 3.12+. `xuma` uses `google-re2` for linear-time regex.

## Write a Config

Create `routes.yaml`:

```yaml
matchers:
  - predicate:
      type: and
      predicates:
        - type: single
          input: { type_url: "xuma.http.v1.PathInput", config: {} }
          value_match: { Prefix: "/api" }
        - type: single
          input: { type_url: "xuma.http.v1.MethodInput", config: {} }
          value_match: { Exact: "GET" }
    on_match: { type: action, action: "api_read" }

  - predicate:
      type: single
      input: { type_url: "xuma.http.v1.PathInput", config: {} }
      value_match: { Exact: "/health" }
    on_match: { type: action, action: "health" }

on_no_match: { type: action, action: "not_found" }
```

## Validate with the CLI

```bash
$ rumi check http routes.yaml
Config valid
```

## Run with the CLI

```bash
$ rumi run http routes.yaml --method GET --path /api/users
api_read

$ rumi run http routes.yaml --method GET --path /health
health

$ rumi run http routes.yaml --method DELETE --path /other
not_found
```

## Load in Your App (xuma)

The pure Python implementation loads the same config:

```python
import yaml
from xuma import Registry, RegistryBuilder
from xuma.http import HttpRequest, register_http

# Build registry with HTTP inputs
builder = RegistryBuilder()
register_http(builder)
registry = builder.build()

# Load config
with open("routes.yaml") as f:
    config = yaml.safe_load(f)
matcher = registry.load_matcher(config)

# Evaluate
request = HttpRequest(method="GET", raw_path="/api/users")
assert matcher.evaluate(request) == "api_read"
```

## Load in Your App (xuma-crust)

The Rust-backed bindings use the same config format:

```python
from xuma_crust import load_http_matcher, HttpMatcher

# Load config and build matcher in one call
matcher: HttpMatcher = load_http_matcher("routes.yaml")

# Evaluate with method + path
assert matcher.evaluate("GET", "/api/users") == "api_read"
assert matcher.evaluate("DELETE", "/other") == "not_found"
```

`xuma-crust` is 10-100x faster than pure Python for evaluation.

## Compiler Shorthand

For type-safe HTTP matching without config files:

```python
from xuma.http import (
    HttpRouteMatch,
    HttpPathMatch,
    HttpRequest,
    compile_route_matches,
)

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

matcher = compile_route_matches(
    matches=routes,
    action="allowed",
    on_no_match="denied",
)

assert matcher.evaluate(HttpRequest(method="GET", raw_path="/api/users")) == "allowed"
assert matcher.evaluate(HttpRequest(method="DELETE", raw_path="/api/users")) == "denied"
```

Within a single `HttpRouteMatch`, all conditions are ANDed. Multiple routes are ORed. First match wins.

## Safety

- **ReDoS protection** -- `google-re2` guarantees linear-time regex matching.
- **Immutable** -- all types are `frozen=True` dataclasses.
- **Depth limits** -- nested matchers capped at 32 levels.
- **Fail-closed** -- missing headers or query params return `None` from `DataInput`, which makes the predicate evaluate to `False`.

## Next Steps

- [The Matching Pipeline](../concepts/pipeline.md) -- how data flows through the matcher
- [CLI Reference](../reference/cli.md) -- all commands and domains
- [Config Format](../reference/config.md) -- full config schema and type URL tables
- [API Reference](../reference/api.md) -- generated docs for all languages
