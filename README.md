# x.uma

[![Docs](https://img.shields.io/badge/docs-x.uma-4da6ff)](https://mox-nexus.github.io/x.uma/)
[![CI](https://github.com/mox-nexus/x.uma/actions/workflows/ci.yml/badge.svg)](https://github.com/mox-nexus/x.uma/actions/workflows/ci.yml)
[![Docs build](https://github.com/mox-nexus/x.uma/actions/workflows/docs.yml/badge.svg)](https://github.com/mox-nexus/x.uma/actions/workflows/docs.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE-MIT)

> **Alpha (0.0.2)** — API is stabilizing. Expect breaking changes before 1.0.

Match structured data against rule trees. Write the rules once; evaluate them in Rust, Python, or TypeScript and get the same answer every time.

## Install

> **Not yet on registries.** No release has been cut, so `rumi-core` and
> `xuma-crust` do not resolve on crates.io, PyPI, or npm yet. Build from source
> until the first release lands.

```bash
git clone https://github.com/mox-nexus/x.uma
cd x.uma
just build          # the engine
just test           # the suites
just docs-dev       # docs + playground on localhost:6200
```

Once released, the runtimes install as:

**Rust** — the engine:
```bash
cargo add rumi-core rumi-http
cargo install rumi-cli   # binary: rumi
```

**Python** — the engine, bound through PyO3:
```bash
uv add xuma-crust
```

**TypeScript** — the engine, compiled to WebAssembly:
```bash
bun add xuma-crust
```

## HTTP route matching — same rules, three languages

**routes.yaml** (one config, all runtimes — canonical protojson):
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
onNoMatch:
  action:
    name: not_found
    typedConfig:
      "@type": type.googleapis.com/xuma.core.v1.NamedAction
      name: not_found
```

**Rust:**
```rust,ignore
use rumi::prelude::*;
use rumi_http::{HttpRequest, register_simple};
use rumi_proto::{any_resolver::AnyResolverBuilder, convert::convert_matcher, protojson::parse_matcher};

let resolver = AnyResolverBuilder::new()
    .register::<rumi_proto::xuma::http::v1::PathInput>("xuma.http.v1.PathInput")
    .register::<rumi_proto::xuma::http::v1::MethodInput>("xuma.http.v1.MethodInput")
    .register::<rumi_proto::xuma::core::v1::NamedAction>("xuma.core.v1.NamedAction")
    .build();

let registry = register_simple(RegistryBuilder::new()).build();
let doc: serde_json::Value = serde_yaml::from_str(&yaml).unwrap();
let proto = parse_matcher(&resolver, doc).unwrap();
let config = convert_matcher(&proto, &resolver).unwrap();
let matcher = registry.load_typed_matcher(config, &actions).unwrap();

let req = HttpRequest::builder().method("GET").path("/api/users").build();
assert_eq!(matcher.evaluate(&req), Some("api_read".to_string()));
```

**Python:**
```python
import json

import yaml
from xuma_crust import HttpMatcher

with open("routes.yaml") as f:
    matcher = HttpMatcher.from_config(json.dumps(yaml.safe_load(f)))

assert matcher.evaluate("GET", "/api/users") == "api_read"
assert matcher.evaluate("DELETE", "/other") == "not_found"
```

**TypeScript:**
```typescript
import { strict as assert } from "node:assert";
import init, { HttpMatcher } from "xuma-crust";
import yaml from "js-yaml";

await init();
const config = yaml.load(await Bun.file("routes.yaml").text());
const matcher = HttpMatcher.fromConfig(JSON.stringify(config));

assert.equal(matcher.evaluate({ method: "GET", path: "/api/users" }), "api_read");
assert.equal(matcher.evaluate({ method: "DELETE", path: "/other" }), "not_found");
```

## How it works

```
Context → DataInput → MatchingData → InputMatcher → bool
           domain-      erased         domain-
           specific                    agnostic
```

`DataInput` extracts a value from your context (an HTTP request, a hook event). `InputMatcher` tests that value. The two halves are separate: an `ExactMatcher` doesn't know whether it's checking an HTTP path or a Claude tool name — it matches data. This is why the same config file works in every runtime: there is one engine, and the bindings hand it the same document.

## Domains

| Domain | Context | Use case |
|--------|---------|----------|
| **HTTP** | Method, path, headers, query | Route matching, gateway policy |
| **Claude** | Hook events, tool names, args | Claude Code hook policies |

**Claude Code hooks** — match on event type, tool, or argument:
```bash
$ rumi run claude hooks.yaml --event PreToolUse --tool Bash --arg command="rm -rf /"
block

$ rumi run claude hooks.yaml --event PreToolUse --tool Read
allow
```

## Guarantees

| Guarantee | How |
|-----------|-----|
| **No ReDoS** | One regex engine everywhere: Rust's `regex` crate, linear time, no backtracking |
| **Bounded depth** | Max 32 nesting levels, validated at config load — not at evaluation time |
| **Fail-closed** | Missing data from `DataInput` → predicate returns `false`. Never matches by accident. |
| **Thread-safe** | `Send + Sync` in Rust; a compiled matcher is immutable in every binding |

The engine and both bindings run the same conformance fixtures in `spec/tests/`.
A binding that disagrees with the engine fails its own build.

## Docs

- **[Playground](https://mox-nexus.github.io/x.uma/playground/)** — build a matcher and watch it evaluate
- **[Getting Started](https://mox-nexus.github.io/x.uma/docs/rust/)** — Rust, Python, TypeScript quick starts
- **[How-to guides](https://mox-nexus.github.io/x.uma/docs/route-by-header/)** — routing, custom inputs, debugging a match
- **[Architecture](https://mox-nexus.github.io/x.uma/docs/architecture/)** — why type erasure sits at the data level
- **[Config Format](https://mox-nexus.github.io/x.uma/docs/config/)** — full schema and type URL reference

## License

MIT OR Apache-2.0
