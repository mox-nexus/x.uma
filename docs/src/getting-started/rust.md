# Rust Quick Start

Build an HTTP route matcher with `rumi` and `rumi-http`.

## Install

```toml
[dependencies]
rumi-core = "0.0.2"
rumi-http = "0.0.2"
```

`rumi-http` brings in `rumi-core` as a transitive dependency. The lib name is `rumi`, so you write `use rumi::prelude::*`.

The CLI is a separate binary:

```bash
cargo install --path rumi/cli
```

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
      type: and
      predicates:
        - type: single
          input: { type_url: "xuma.http.v1.PathInput", config: {} }
          value_match: { Prefix: "/api" }
        - type: single
          input: { type_url: "xuma.http.v1.MethodInput", config: {} }
          value_match: { Exact: "POST" }
    on_match: { type: action, action: "api_write" }

on_no_match: { type: action, action: "not_found" }
```

The `type_url` selects which data input to extract. `value_match` tests the extracted value. See [Config Format](../reference/config.md) for the full schema.

## Validate with the CLI

```bash
$ rumi check http routes.yaml
Config valid
```

Catches unknown type URLs, invalid regex patterns, and depth limit violations at load time.

## Run with the CLI

```bash
$ rumi run http routes.yaml --method GET --path /api/users
api_read

$ rumi run http routes.yaml --method POST --path /api/items
api_write

$ rumi run http routes.yaml --method DELETE --path /api/users
not_found
```

## Load in Your App

The same config file works programmatically via the Registry API:

```rust,ignore
use rumi::prelude::*;
use rumi_http::{HttpRequest, register_simple};

fn main() {
    // Build registry with HTTP inputs
    let registry = register_simple(RegistryBuilder::new()).build();

    // Load the config
    let yaml = std::fs::read_to_string("routes.yaml").unwrap();
    let config: MatcherConfig<String> = serde_yaml::from_str(&yaml).unwrap();
    let matcher = registry.load_matcher(config).unwrap();

    // Evaluate
    let request = HttpRequest::builder()
        .method("GET")
        .path("/api/users")
        .build();
    assert_eq!(matcher.evaluate(&request), Some("api_read".to_string()));
}
```

The registry resolves `type_url` strings to concrete `DataInput` implementations at load time. Unknown type URLs produce an error listing available types.

## Compiler Shorthand

For type-safe HTTP matching without config files, use the Gateway API compiler:

```rust,ignore
use rumi::prelude::*;
use rumi_http::prelude::*;

// Declarative config
let routes = vec![
    HttpRouteMatch {
        path: Some(HttpPathMatch::Prefix { value: "/api".into() }),
        method: Some(HttpMethod::Get),
        ..Default::default()
    },
];

// One call compiles all routes into a matcher
let matcher = compile_route_matches(&routes, "allowed", "denied").unwrap();

let req = HttpRequest::builder().method("GET").path("/api/users").build();
assert_eq!(matcher.evaluate(&req), Some(&"allowed"));
```

This requires `rumi-http` with the `ext-proc` feature (enabled by default).

## Claude Code Hooks

rumi also matches Claude Code hook events. Create `hooks.yaml`:

```yaml
matchers:
  - predicate:
      type: and
      predicates:
        - type: single
          input: { type_url: "xuma.claude.v1.EventInput", config: {} }
          value_match: { Exact: "PreToolUse" }
        - type: single
          input: { type_url: "xuma.claude.v1.ToolNameInput", config: {} }
          value_match: { Exact: "Bash" }
        - type: single
          input: { type_url: "xuma.claude.v1.ArgumentInput", config: { name: "command" } }
          value_match: { Contains: "rm -rf" }
    on_match: { type: action, action: "block" }

on_no_match: { type: action, action: "allow" }
```

```bash
$ rumi check claude hooks.yaml
Config valid

$ rumi run claude hooks.yaml --event PreToolUse --tool Bash --arg command="rm -rf /"
block

$ rumi run claude hooks.yaml --event PreToolUse --tool Read
allow
```

## Safety

- **ReDoS protection** -- the `regex` crate guarantees linear-time matching. No backtracking.
- **Depth limits** -- nested matchers capped at 32 levels, validated at construction.
- **No unsafe in core** -- all `Send + Sync` is compiler-derived.

## Next Steps

- [The Matching Pipeline](../concepts/pipeline.md) -- how data flows through the matcher
- [CLI Reference](../reference/cli.md) -- all commands and domains
- [Config Format](../reference/config.md) -- full config schema and type URL tables
- [API Reference](../reference/api.md) -- generated docs for all languages
