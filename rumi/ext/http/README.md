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
    FieldMatcherIr, MatcherIr, OnMatchIr, PredicateIr, RegistryBuilder,
    SinglePredicateIr, StringMatchSpec, TypedExtensionConfig, ValueMatchIr,
};
use rumi_http::{register, HttpMessageBuilder};

let registry = register(RegistryBuilder::new()).build();

let config = MatcherIr::list(vec![FieldMatcherIr {
        predicate: PredicateIr::Single(SinglePredicateIr {
            input: TypedExtensionConfig {
                type_url: "xuma.http.v1.PathInput".into(),
                config: serde_json::json!({}),
            },
            matcher: ValueMatchIr::BuiltIn {
                spec: StringMatchSpec::Prefix("/api".into()),
                ignore_case: false,
            },
        }),
        on_match: OnMatchIr::Action {
            action: "api_read".to_string(),
        },
}]);

let matcher = registry.load_matcher(config).unwrap();

let req = HttpMessageBuilder::new().method("GET").path("/api/users?page=2").build();
assert_eq!(matcher.evaluate(&req), Some("api_read".to_string()));
# }
```

## Features

`default = []`. Nothing is enabled unless you ask for it.

| Feature | Adds |
|---|---|
| `registry` | config-file loading via `rumi-core`'s registry |
| `gateway` | Gateway API types and `compile_route_matches` |
| `ext-proc` | Envoy `ext_proc` data-plane types, and `From<ProcessingRequest>` |

`HttpMessage`, its `DataInput`s and `HttpMessageBuilder` need no feature: they
have no dependencies. `gateway` and `ext-proc` are **siblings** over them, not a
chain. An
ext_proc filter has no use for Kubernetes config types, and a Gateway API user
has no use for a data plane.

`ext-proc` is not a default. It pulls 101 crates against 7 — tokio, tonic,
hyper, h2 — into a library that never makes a network call, and every consumer
in this repo already opted out of it. If you are writing an ext_proc filter you
want it; otherwise you do not.

## One context

`HttpMessage` is a request as it arrives: the query string inside `:path`, and
`:authority` and `:scheme` as pseudo-headers. The registry, the Gateway API
compiler and the `ext_proc` conversion all target it. A second context,
`HttpRequest`, had its own registry over the same `xuma.http.v1.*` type URLs,
and once gave `PathInput` two meanings depending on the caller; it was retired
before first publish (D-059).
