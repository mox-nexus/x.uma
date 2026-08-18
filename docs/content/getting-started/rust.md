# Rust Quick Start

Build an HTTP route matcher with `rumi` and `rumi-http`.

## Install

> **Not yet on registries.** No release has been cut, so these packages do not
> resolve yet. Until the first release, clone the repo and run `just build`.
> See the [README](https://github.com/mox-nexus/x.uma#install).

```toml
[dependencies]
rumi-core = "0.0.2"
rumi-http = { version = "0.0.2", features = ["registry"] }   # add "gateway" for the compiler
```

`rumi-http` brings in `rumi-core` as a transitive dependency. The lib name is `rumi`, so you write `use rumi::prelude::*`.

The CLI is a separate binary:

```bash
cargo install --path rumi/cli
```

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
                  exact: POST
      onMatch:
        action:
          name: api_write
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: api_write

onNoMatch:
  action:
    name: not_found
    typedConfig:
      "@type": type.googleapis.com/xuma.core.v1.NamedAction
      name: not_found
```

This is canonical protojson — protobuf's own JSON mapping of
`xds.type.matcher.v3.Matcher`. `@type` selects which data input to extract.
`valueMatch` tests the extracted value. See [Config Format](../reference/config.md)
for the full schema.

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

```rust,no_run
use rumi::prelude::*;
use rumi::RegistryBuilder;
use rumi_http::{register_simple, HttpRequest};
use rumi_proto::any_resolver::{AnyResolver, AnyResolverBuilder};
use rumi_proto::convert::convert_matcher;
use rumi_proto::protojson::parse_matcher;
use rumi_proto::xuma;

/// Every type URL a config may name, and the one action type this engine ships.
fn resolver() -> AnyResolver {
    AnyResolverBuilder::new()
        .register::<xuma::http::v1::PathInput>("xuma.http.v1.PathInput")
        .register::<xuma::http::v1::MethodInput>("xuma.http.v1.MethodInput")
        .register::<xuma::core::v1::NamedAction>("xuma.core.v1.NamedAction")
        .build()
}

struct NamedActionFactory;

impl rumi::IntoAction<String> for NamedActionFactory {
    type Config = xuma::core::v1::NamedAction;

    fn from_config(config: Self::Config) -> Result<String, rumi::MatcherError> {
        Ok(config.name)
    }
}

fn main() {
    // Build registry with HTTP inputs, and the action registry NamedAction
    // resolves through.
    let registry = register_simple(RegistryBuilder::new()).build();
    let actions = rumi::ActionRegistryBuilder::new()
        .action::<NamedActionFactory>("xuma.core.v1.NamedAction")
        .build();

    // Load the config: canonical protojson in, runtime Matcher out.
    let yaml = std::fs::read_to_string("routes.yaml").unwrap();
    let resolver = resolver();
    let doc: serde_json::Value = serde_yaml::from_str(&yaml).unwrap();
    let proto = parse_matcher(&resolver, doc).unwrap();
    let config = convert_matcher(&proto, &resolver).unwrap();
    let matcher = registry.load_typed_matcher(config, &actions).unwrap();

    // Evaluate
    let request = HttpRequest::builder()
        .method("GET")
        .path("/api/users")
        .build();
    assert_eq!(matcher.evaluate(&request), Some("api_read".to_string()));
}
```

The registry resolves `@type` URLs to concrete `DataInput` implementations at
load time, through the same `AnyResolver` that decodes them off the wire when a
host's xDS client is the source instead of a file. Unknown type URLs produce an
error listing available types.

## Compiler Shorthand

For type-safe HTTP matching without config files, use the Gateway API compiler:

```rust,no_run
use rumi::prelude::*;
use rumi_http::{compile_route_matches, HttpPathMatch, HttpRouteMatch};

// Declarative config. Note `PathPrefix`, not `Prefix` — these are the Gateway
// API's own names, and `method` is a plain String.
let routes = vec![
    HttpRouteMatch {
        path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
        method: Some("GET".into()),
        ..Default::default()
    },
];

// One call compiles all routes into a matcher. It returns Result: an invalid
// or oversized regex is reported rather than silently dropping the route.
let matcher = compile_route_matches(&routes, "allowed", Some("denied")).unwrap();

// `matcher` evaluates against `HttpMessage`, the indexed ext_proc context.
// See the note below on getting one.
```

`compile_route_matches` produces a `Matcher<HttpMessage, _>`, and `HttpMessage`
is currently only constructible from an ext_proc `ProcessingRequest` — there is
no public builder. So the Gateway API compiler is usable today from inside an
ext_proc filter, but not from a scratch program. If you want to experiment
locally, use the config path above with `HttpRequest`, which does have a
builder.

This requires `rumi-http` with the `gateway` feature. Nothing is enabled by
default — `default = []` — so ask for what you need:

```toml
rumi-http = { version = "0.0.2", features = ["registry", "gateway"] }
```

The `ext-proc` feature is separate and pulls tonic, tokio and hyper. You want it
only if you are writing an Envoy ext_proc filter.

## Claude Code Hooks

rumi also matches Claude Code hook events. Create `hooks.yaml`:

```yaml
matcherList:
  matchers:
    - predicate:
        andMatcher:
          predicate:
            - singlePredicate:
                input:
                  name: event
                  typedConfig:
                    "@type": type.googleapis.com/xuma.claude.v1.EventTypeInput
                valueMatch:
                  exact: PreToolUse
            - singlePredicate:
                input:
                  name: tool
                  typedConfig:
                    "@type": type.googleapis.com/xuma.claude.v1.ToolNameInput
                valueMatch:
                  exact: Bash
            - singlePredicate:
                input:
                  name: command
                  typedConfig:
                    "@type": type.googleapis.com/xuma.claude.v1.ToolArgInput
                    name: command
                valueMatch:
                  contains: "rm -rf"
      onMatch:
        action:
          name: block
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: block

onNoMatch:
  action:
    name: allow
    typedConfig:
      "@type": type.googleapis.com/xuma.core.v1.NamedAction
      name: allow
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
