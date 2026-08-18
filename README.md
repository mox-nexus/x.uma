# x.uma

[![Docs](https://img.shields.io/badge/docs-x.uma-4da6ff)](https://mox-nexus.github.io/x.uma/)
[![CI](https://github.com/mox-nexus/x.uma/actions/workflows/docs.yml/badge.svg)](https://github.com/mox-nexus/x.uma/actions/workflows/docs.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE-MIT)

> **Alpha (0.0.2)** — API is stabilizing. Expect breaking changes before 1.0.

Match structured data against rule trees. Write the rules once; evaluate them in Rust, Python, or TypeScript and get the same answer every time.

## Install

> **Not yet on registries.** No release has been cut, so `rumi-core`, `xuma`, and
> `xuma-crust` do not resolve on crates.io, PyPI, or npm yet. Build from source
> until the first release lands.

```bash
git clone https://github.com/mox-nexus/x.uma
cd x.uma
just build          # all runtimes
just test           # conformance suite, every implementation
just docs-dev       # docs + playground on localhost:6200
```

Once released, the runtimes install as:

**Rust** — reference implementation, lowest latency:
```bash
cargo add rumi-core rumi-http
cargo install rumi-cli   # binary: rumi
```

**Python** — no-build or Rust-backed:
```bash
uv add xuma          # no build step (google-re2 ships wheels)
uv add xuma-crust    # Rust-backed via PyO3, 10–100x faster evaluation
```

**TypeScript** — pure TypeScript or WASM-backed:
```bash
bun add xuma         # pure TypeScript (re2js for regex)
bun add xuma-crust   # Rust-backed via WASM, 3–10x faster evaluation
```

## HTTP route matching — same rules, three languages

**routes.yaml** (one config, all runtimes):
```yaml
matchers:
  - predicate:
      type: and
      predicates:
        - type: single
          input: { type_url: "xuma.http.v1.PathInput" }
          value_match: { Prefix: "/api" }
        - type: single
          input: { type_url: "xuma.http.v1.MethodInput" }
          value_match: { Exact: "GET" }
    on_match: { type: action, action: "api_read" }
on_no_match: { type: action, action: "not_found" }
```

**Rust:**
```rust,ignore
use rumi::prelude::*;
use rumi_http::{HttpRequest, register_simple};

let registry = register_simple(RegistryBuilder::new()).build();
let config: MatcherConfig<String> = serde_yaml::from_str(&yaml).unwrap();
let matcher = registry.load_matcher(config).unwrap();

let req = HttpRequest::builder().method("GET").path("/api/users").build();
assert_eq!(matcher.evaluate(&req), Some("api_read".to_string()));
```

**Python:**
```python
from xuma.http import HttpRequest, HttpRouteMatch, HttpPathMatch, compile_route_matches

matcher = compile_route_matches(
    [HttpRouteMatch(path=HttpPathMatch(type="PathPrefix", value="/api"), method="GET")],
    action="api_read",
    on_no_match="not_found",
)

matcher.evaluate(HttpRequest(method="GET", raw_path="/api/users"))   # "api_read"
matcher.evaluate(HttpRequest(method="DELETE", raw_path="/other"))     # "not_found"
```

**TypeScript:**
```typescript
import { compileRouteMatches, HttpRequest } from "xuma/http";

const matcher = compileRouteMatches(
    [{ path: { type: "PathPrefix", value: "/api" }, method: "GET" }],
    "api_read",
    "not_found",
);

matcher.evaluate(new HttpRequest("GET", "/api/users"));   // "api_read"
matcher.evaluate(new HttpRequest("DELETE", "/other"));     // "not_found"
```

## How it works

```
Context → DataInput → MatchingData → InputMatcher → bool
           domain-      erased         domain-
           specific                    agnostic
```

`DataInput` extracts a value from your context (an HTTP request, a hook event). `InputMatcher` tests that value. The two halves are separate: an `ExactMatcher` doesn't know whether it's checking an HTTP path or a Claude tool name — it matches data. This is why the same config file works across all five implementations.

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
| **No ReDoS** | RE2-class engines everywhere: Rust `regex` crate, `google-re2` (Python), `re2js` (TypeScript) |
| **Bounded depth** | Max 32 nesting levels, validated at config load — not at evaluation time |
| **Fail-closed** | Missing data from `DataInput` → predicate returns `false`. Never matches by accident. |
| **Thread-safe** | `Send + Sync` (Rust) / immutable frozen types (Python, TypeScript) |

All five implementations run the same 27 conformance fixtures in `spec/tests/`.
Suites at last run: 274 Rust, 294 Python, 258 TypeScript.

## Docs

- **[Playground](https://mox-nexus.github.io/x.uma/playground/)** — build a matcher and watch it evaluate
- **[Getting Started](https://mox-nexus.github.io/x.uma/docs/rust/)** — Rust, Python, TypeScript quick starts
- **[How-to guides](https://mox-nexus.github.io/x.uma/docs/route-by-header/)** — routing, custom inputs, debugging a match
- **[Architecture](https://mox-nexus.github.io/x.uma/docs/architecture/)** — why type erasure sits at the data level
- **[Config Format](https://mox-nexus.github.io/x.uma/docs/config/)** — full schema and type URL reference

## License

MIT OR Apache-2.0
