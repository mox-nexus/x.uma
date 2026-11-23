# rumi-kv

The key-value matching domain for [rumi](https://crates.io/crates/rumi-core):
match on a `String -> String` map.

It is the simplest possible domain, and the one `rumi` the CLI uses by default:

```bash
rumi run rules.yaml --context role=admin org=acme
```

Configs are canonical protojson, read through `rumi-proto`. This example builds
the same matcher directly, so it stays self-contained:

```rust
# #[cfg(feature = "registry")] {
use rumi::prelude::*;
use rumi::{
    FieldMatcherIr, MatcherIr, OnMatchIr, PredicateIr, RegistryBuilder,
    SinglePredicateIr, StringMatchSpec, TypedExtensionConfig, ValueMatchIr,
};
use rumi_kv::{register, KvContext};

let registry = register(RegistryBuilder::new()).build();

let config = MatcherIr::list(vec![FieldMatcherIr {
        predicate: PredicateIr::Single(SinglePredicateIr {
            input: TypedExtensionConfig {
                type_url: "xuma.core.v1.MapInput".into(),
                config: serde_json::json!({ "key": "role" }),
            },
            matcher: ValueMatchIr::BuiltIn {
                spec: StringMatchSpec::Exact("admin".into()),
                ignore_case: false,
            },
        }),
        on_match: OnMatchIr::Action {
            action: "allow".to_string(),
        },
}]);

let matcher = registry.load_matcher(config).unwrap();

let ctx = KvContext::new().with("role", "admin");
assert_eq!(matcher.evaluate(&ctx), Some("allow".to_string()));
# }
```

## Why this is its own crate

It is not part of `rumi-test`, the conformance suite's YAML fixture loader. That
crate is `publish = false`, because a fixture loader has no business in a
published artifact, and `rumi-cli` needs only the domain.

It is a general-purpose domain, a peer of `rumi-http`.

## Type URLs

| Type URL | Reads |
|---|---|
| `xuma.core.v1.MapInput` | one key from the context map |

The URL is in `xuma.core`, not a `kv` package: the proto message is defined in
the core schema (D-069). The Rust `MapInput` and the context it reads are
defined here.
