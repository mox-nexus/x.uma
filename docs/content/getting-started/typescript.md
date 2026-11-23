# TypeScript Quick Start

Build an HTTP route matcher with `xuma-crust`: the Rust engine, compiled to WebAssembly.

## Install

> **Not yet on registries.** No release has been cut, so these packages do not
> resolve yet. Until the first release, clone the repo and run `just build`.
> See the [README](https://github.com/mox-nexus/x.uma#install).

```bash
bun add xuma-crust
```

Requires the Bun runtime.

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
  rules:     2
  inputs:    xuma.http.v1.MethodInput, xuma.http.v1.PathInput
  fallback:  yes
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

## Load in Your App

`fromConfig` takes the same document the CLI read. The package reads protojson,
not YAML: bring your own parser (`bun add js-yaml`).

```typescript
import { strict as assert } from "node:assert";
import init, { HttpMatcher } from "xuma-crust";
import yaml from "js-yaml";

// The WASM module must be initialised before any class is used.
await init();

// fromConfig takes canonical protojson as a string, so the YAML becomes JSON.
const config = yaml.load(await Bun.file("routes.yaml").text());
const matcher = HttpMatcher.fromConfig(JSON.stringify(config));

// Evaluate with a request object
assert.equal(matcher.evaluate({ method: "GET", path: "/api/users" }), "api_read");
assert.equal(matcher.evaluate({ method: "DELETE", path: "/other" }), "not_found");
```

## Integration: Bun HTTP Server

<!-- doc-sample: compile -->
```typescript
import init, { HttpMatcher } from "xuma-crust";
import yaml from "js-yaml";

await init();
const config = yaml.load(await Bun.file("routes.yaml").text());
const matcher = HttpMatcher.fromConfig(JSON.stringify(config));

Bun.serve({
  port: 3000,
  fetch(req) {
    const url = new URL(req.url);
    const action = matcher.evaluate({
      method: req.method,
      path: url.pathname + url.search,
      headers: Object.fromEntries(req.headers),
    });
    if (action === "not_found") {
      return new Response("Not found", { status: 404 });
    }
    return new Response(action ?? "no decision");
  },
});
```

## Safety

- **ReDoS protection** -- regex runs in Rust's `regex` crate: linear time, no backtracking.
- **Immutable** -- a compiled matcher cannot be changed after `fromConfig`.
- **Bounded** -- nesting depth, rule count and pattern length are limited at load, not at evaluation.
- **Missing is not a match** -- a missing header or query parameter makes its predicate `false`. Under `not` that becomes a match, so mark an input `required` when its absence must not decide anything: the result then says `undecided`.

## Next Steps

- [The Matching Pipeline](../concepts/pipeline.md) -- how data flows through the matcher
- [CLI Reference](../reference/cli.md) -- all commands and domains
- [Config Format](../reference/config.md) -- full config schema and type URL tables
- [API Reference](../reference/api.md) -- generated docs for all languages
