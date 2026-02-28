# TypeScript Quick Start

Build an HTTP route matcher with `xuma` (pure TypeScript) or `xuma-crust` (WASM-backed).

## Install

```bash
# Pure TypeScript
bun add xuma

# WASM-backed (faster, same API surface)
bun add xuma-crust
```

Requires Bun runtime. `xuma` uses `re2js` for linear-time regex.

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

The pure TypeScript implementation loads the same config:

```typescript
import { RegistryBuilder, registerHttp, type MatcherConfig } from "xuma";
import { HttpRequest } from "xuma/http";
import { parse } from "yaml";

// Build registry with HTTP inputs
const builder = new RegistryBuilder();
registerHttp(builder);
const registry = builder.build();

// Load config
const yaml = await Bun.file("routes.yaml").text();
const config: MatcherConfig = parse(yaml);
const matcher = registry.loadMatcher(config);

// Evaluate
const request = new HttpRequest("GET", "/api/users");
console.assert(matcher.evaluate(request) === "api_read");
```

## Load in Your App (xuma-crust)

The WASM-backed bindings use the same config format:

```typescript
import { loadHttpMatcher, type HttpMatcher } from "xuma-crust";

// Load config and build matcher in one call
const matcher: HttpMatcher = loadHttpMatcher("routes.yaml");

// Evaluate with method + path
console.assert(matcher.evaluate("GET", "/api/users") === "api_read");
console.assert(matcher.evaluate("DELETE", "/other") === "not_found");
```

`xuma-crust` is 3-10x faster than pure TypeScript for evaluation.

## Compiler Shorthand

For type-safe HTTP matching without config files:

```typescript
import { compileRouteMatches, HttpRequest } from "xuma/http";
import type { HttpRouteMatch } from "xuma/http";

const routes: HttpRouteMatch[] = [
  {
    path: { type: "PathPrefix", value: "/api" },
    method: "GET",
  },
  {
    path: { type: "PathPrefix", value: "/admin" },
    method: "POST",
  },
];

const matcher = compileRouteMatches(routes, "allowed", "denied");

console.assert(matcher.evaluate(new HttpRequest("GET", "/api/users")) === "allowed");
console.assert(matcher.evaluate(new HttpRequest("DELETE", "/api/users")) === "denied");
```

Within a single `HttpRouteMatch`, all conditions are ANDed. Multiple routes are ORed. First match wins.

## Integration: Bun HTTP Server

```typescript
import { compileRouteMatches, HttpRequest } from "xuma/http";

const matcher = compileRouteMatches(
  [{ path: { type: "PathPrefix", value: "/api" }, method: "GET" }],
  "allowed",
  "denied",
);

Bun.serve({
  port: 3000,
  fetch(req) {
    const url = new URL(req.url);
    const request = new HttpRequest(
      req.method,
      url.pathname + url.search,
      Object.fromEntries(req.headers),
    );
    if (matcher.evaluate(request) === "denied") {
      return new Response("Not found", { status: 404 });
    }
    return new Response("OK");
  },
});
```

## Safety

- **ReDoS protection** -- `re2js` guarantees linear-time regex matching.
- **Immutable** -- all types use `readonly` fields.
- **Depth limits** -- nested matchers capped at 32 levels.
- **Fail-closed** -- missing data from `DataInput` returns `null`, which makes the predicate evaluate to `false`.

## Next Steps

- [The Matching Pipeline](../concepts/pipeline.md) -- how data flows through the matcher
- [CLI Reference](../reference/cli.md) -- all commands and domains
- [Config Format](../reference/config.md) -- full config schema and type URL tables
- [API Reference](../reference/api.md) -- generated docs for all languages
