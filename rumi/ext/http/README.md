# rumi-http

The HTTP matching domain for [rumi](https://crates.io/crates/rumi-core).

Match on method, path, headers and query parameters — either from a config file
through the registry, or by compiling Gateway API `HttpRouteMatch` values
directly.

Configs are canonical protojson, read through `rumi-proto`. This example builds
the same matcher directly, so it stays self-contained:

```rust
# #[cfg(feature = "registry")] {
use rumi::prelude::*;
use rumi::{
    FieldMatcherConfig, MatcherConfig, OnMatchConfig, PredicateConfig, RegistryBuilder,
    SinglePredicateConfig, StringMatchSpec, TypedConfig, ValueMatchConfig,
};
use rumi_http::{register_simple, HttpRequest};

let registry = register_simple(RegistryBuilder::new()).build();

let config = MatcherConfig::list(vec![FieldMatcherConfig {
        predicate: PredicateConfig::Single(SinglePredicateConfig {
            input: TypedConfig {
                type_url: "xuma.http.v1.PathInput".into(),
                config: serde_json::json!({}),
            },
            matcher: ValueMatchConfig::BuiltIn {
                spec: StringMatchSpec::Prefix("/api".into()),
                ignore_case: false,
            },
        }),
        on_match: OnMatchConfig::Action {
            action: "api_read".to_string(),
        },
}]);

let matcher = registry.load_matcher(config).unwrap();

let req = HttpRequest::builder().method("GET").path("/api/users").build();
assert_eq!(matcher.evaluate(&req), Some("api_read".to_string()));
# }
```

## Features

`default = []`. Nothing is enabled unless you ask for it.

| Feature | Adds |
|---|---|
| `message` | `HttpMessage`, its `DataInput`s, and `HttpMessageBuilder`. No dependencies. |
| `registry` | config-file loading via `rumi-core`'s registry |
| `gateway` | Gateway API types and `compile_route_matches` |
| `ext-proc` | Envoy `ext_proc` data-plane types, and `From<ProcessingRequest>` |
| `proto` | xDS proto config loading |

`gateway` and `ext-proc` are **siblings** over `message`, not a chain. An
ext_proc filter has no use for Kubernetes config types, and a Gateway API user
has no use for a data plane.

`ext-proc` is not a default. It pulls 101 crates against 7 — tokio, tonic,
hyper, h2 — into a library that never makes a network call, and every consumer
in this repo already opted out of it. If you are writing an ext_proc filter you
want it; otherwise you do not.

## Two contexts

`HttpMessage` is the indexed context the Gateway API compiler targets. Build one
with `HttpMessageBuilder` — it needs no data plane. `From<ProcessingRequest>` is
an additional adapter under `ext-proc`.

`HttpRequest` is a simpler struct that existed because `HttpMessage` used to be
un-constructible without ext_proc. That reason is gone, and the two are not
interchangeable — they differ in whether a missing method can be `None`, and in
whether the raw path is available. Collapsing them is a decision owed before
first publish.
