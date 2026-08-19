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
    FieldMatcherConfig, MatcherConfig, OnMatchConfig, PredicateConfig, RegistryBuilder,
    SinglePredicateConfig, StringMatchSpec, TypedConfig, ValueMatchConfig,
};
use rumi_kv::{register, KvContext};

let registry = register(RegistryBuilder::new()).build();

let config = MatcherConfig::list(vec![FieldMatcherConfig {
        predicate: PredicateConfig::Single(SinglePredicateConfig {
            input: TypedConfig {
                type_url: "xuma.kv.v1.MapInput".into(),
                config: serde_json::json!({ "key": "role" }),
            },
            matcher: ValueMatchConfig::BuiltIn {
                spec: StringMatchSpec::Exact("admin".into()),
                ignore_case: false,
            },
        }),
        on_match: OnMatchConfig::Action {
            action: "allow".to_string(),
        },
}]);

let matcher = registry.load_matcher(config).unwrap();

let ctx = KvContext::new().with("role", "admin");
assert_eq!(matcher.evaluate(&ctx), Some("allow".to_string()));
# }
```

## Why this is its own crate

It used to live inside `rumi-test` alongside the conformance suite's YAML
fixture loader. That crate is `publish = false`, because a fixture loader has no
business in a published artifact — which meant `rumi-cli` could not be published
either, despite only ever wanting the domain.

The concept was never "test". It is a general-purpose domain, a peer of
`rumi-http`, and the misleading name is what hid the release blocker.

## Type URLs

| Type URL | Reads |
|---|---|
| `xuma.kv.v1.MapInput` | one key from the context map |

The URL still says `test`. Type URLs are part of the config schema and freeze at
first publish, so renaming them belongs with the schema migration rather than
with a crate split — see `PLAN.md` SF8.
