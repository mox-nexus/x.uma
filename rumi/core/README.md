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

#[derive(Debug)]
struct MethodInput;

impl DataInput<&'static str> for MethodInput {
    fn get(&self, ctx: &&'static str) -> MatchingData {
        MatchingData::String((*ctx).to_string())
    }
}

let matcher = Matcher::list(
    vec![FieldMatcher::new(
        Predicate::Single(SinglePredicate::new(
            Box::new(MethodInput),
            Box::new(ExactMatcher::new("GET")),
        )),
        OnMatch::Action("matched"),
    )],
    Some(OnMatch::Action("fallback")),
);
matcher.validate().unwrap();

assert_eq!(matcher.evaluate(&"GET"), Some("matched"));
assert_eq!(matcher.evaluate(&"POST"), Some("fallback"));
```

## Loading from a config file

Configs are written in canonical protojson — protobuf's own JSON mapping of
`xds.type.matcher.v3.Matcher`. Reading one needs the generated xDS types, so it
lives in `rumi-proto` rather than here:

```toml
rumi-proto = "0.0.2"
```

```rust,ignore
use rumi_proto::convert::load_proto_matcher;
use rumi_proto::protojson::parse_matcher_str;

let proto = parse_matcher_str(&resolver, config_json)?;
let matcher = load_proto_matcher(&registry, &actions, &resolver, &proto)?;
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
