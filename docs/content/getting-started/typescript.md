# TypeScript Quick Start

Build an HTTP route matcher with `xuma` (pure TypeScript) or `xuma-crust` (WASM-backed).

## Install

> **Not yet on registries.** No release has been cut, so these packages do not
> resolve yet. Until the first release, clone the repo and run `just build`.
> See the [README](https://github.com/mox-nexus/x.uma#install).

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
matcherList:
  matchers:
    - predicate:
        andMatcher:
          predicate:
            - singlePredicate:
                input:
                  name: path
                  typedConfig:
                    "@type": type.googleapis.com/xuma.http.v1.PathInput
                valueMatch:
                  prefix: /api
            - singlePredicate:
                input:
                  name: method
                  typedConfig:
                    "@type": type.googleapis.com/xuma.http.v1.MethodInput
                valueMatch:
                  exact: GET
      onMatch:
        action:
          name: api_read
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: api_read

    - predicate:
        singlePredicate:
          input:
            name: path
            typedConfig:
              "@type": type.googleapis.com/xuma.http.v1.PathInput
          valueMatch:
            exact: /health
      onMatch:
        action:
          name: health
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: health

onNoMatch:
  action:
    name: not_found
    typedConfig:
      "@type": type.googleapis.com/xuma.core.v1.NamedAction
      name: not_found
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
import { parseProtojson, RegistryBuilder } from "xuma";
import { HttpRequest, register } from "xuma/http";
import { parse } from "yaml";

// Build registry with HTTP inputs
const builder = new RegistryBuilder();
register(builder);
const registry = builder.build();

// Load config: canonical protojson in, runtime Matcher out
const yaml = await Bun.file("routes.yaml").text();
const config = parseProtojson(parse(yaml));
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
