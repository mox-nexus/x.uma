# rumi-core

Rust implementation of the xDS Unified Matcher API.

Part of the [x.uma](https://github.com/mox-nexus/x.uma) matcher engine — also available as [xuma](https://pypi.org/project/xuma/) (Python) and [xuma](https://www.npmjs.com/package/xuma) (TypeScript).

## Installation

```bash
cargo add rumi-core
```

In code, the crate is imported as `rumi`:

```rust
use rumi::prelude::*;
```

## Example

```rust
use rumi::prelude::*;
use rumi::{MatcherConfig, RegistryBuilder};

// Build from config (JSON/YAML → registry → matcher)
let json = serde_json::json!({
    "matchers": [{
        "predicate": {
            "type": "single",
            "input": { "type_url": "my.StringInput", "config": { "key": "method" } },
            "value_match": { "Exact": "GET" }
        },
        "on_match": { "type": "action", "action": "matched" }
    }],
    "on_no_match": { "type": "action", "action": "fallback" }
});

let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
// ... register inputs, build registry, load matcher, evaluate
```

## Features

| Feature | Description |
|---------|-------------|
| `registry` | Config-driven matcher construction via `RegistryBuilder` |
| `claude` | Claude Code hook matching domain |
| `serde` | Serialization support |

## Architecture

rumi uses hexagonal architecture (ports & adapters):

- **`DataInput<Ctx>`** — domain-specific extraction port (generic over context)
- **`InputMatcher`** — domain-agnostic matching port (non-generic, shareable)
- **`Matcher<Ctx, A>`** — evaluates predicate trees, returns first-match action

Type erasure at the data level (`MatchingData`) means the same `ExactMatcher` works across HTTP, Claude hooks, or any custom domain.

## Extension Crates

| Crate | Description |
|-------|-------------|
| `rumi-http` | HTTP route matching (Gateway API compiler) |
| `rumi-test` | Conformance test utilities |
| `rumi-proto` | Protobuf types + xDS Matcher loading |

## Security

`RegexMatcher` uses the `regex` crate (linear-time, RE2 semantics, ReDoS-safe). Matcher depth is validated at construction (max 32 levels).

## License

MIT OR Apache-2.0
