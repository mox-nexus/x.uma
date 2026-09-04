# xuma-crust

WASM-backed bindings for the [x.uma](https://github.com/mox-nexus/x.uma) matcher
engine, built with `wasm-bindgen` over the Rust core.

Same config format, and the same conformance suite (`spec/tests/07_protojson/`)
as the pure-TypeScript `xuma` package. Reach for this when evaluation is hot enough to pay for loading a WASM
module.

```bash
bun add xuma-crust
```

## Use

The module must be initialised before any class is touched — this is a WASM
package, so the first call is `init()`, not a constructor.

`fromConfig` takes canonical protojson as a **string**: the same document every
x.uma implementation reads. If your config is YAML, parse it and re-serialise.

```typescript
import init, { HttpMatcher } from "xuma-crust";

await init();

const CONFIG = JSON.stringify({
  matcherList: {
    matchers: [{
      predicate: {
        singlePredicate: {
          input: {
            name: "path",
            typedConfig: { "@type": "type.googleapis.com/xuma.http.v1.PathInput" },
          },
          valueMatch: { prefix: "/api" },
        },
      },
      onMatch: {
        action: {
          name: "api",
          typedConfig: {
            "@type": "type.googleapis.com/xuma.core.v1.NamedAction",
            name: "api",
          },
        },
      },
    }],
  },
});

const matcher = HttpMatcher.fromConfig(CONFIG);

matcher.evaluate({ method: "GET", path: "/api/users" }); // "api"
matcher.evaluate({ method: "GET", path: "/other" });     // undefined
```

`HookMatcher` compiles Claude Code hook rules the same way, and `TestMatcher`
runs conformance fixtures.

## A rule with no conditions is refused

An empty rule matches every request, which in an allowlist is a total bypass.
Both the compilers and these bindings reject one unless you say the catch-all is
intended — `matchAll: true` on a `HookMatcher` rule. See `DECISIONS.md` D-050.

## Security

Regex goes through Rust's `regex` crate: linear time, no backtracking, so a
pattern cannot be turned into a denial of service. Pattern length, matcher depth
and matcher width are bounded at construction.

Documentation: https://mox-nexus.github.io/x.uma/

License: MIT OR Apache-2.0
