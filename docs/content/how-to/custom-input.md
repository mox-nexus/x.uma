# Add a custom input

The built-in inputs read strings, integers, booleans, and bytes. When you need to
match on something they do not reach, you add a `DataInput` rather than changing
the engine.

## What an input is

A `DataInput` extracts one value from your context and returns it as
`MatchingData`, the erased type every matcher understands. That erasure is what
lets one `ExactMatcher` work across HTTP, Claude hooks, and your own domain
without knowing anything about them.

```rust
pub trait DataInput<Ctx>: Send + Sync + Debug {
    fn get(&self, ctx: &Ctx) -> MatchingData;
}
```

`Ctx` is yours. `MatchingData` is fixed.

## Write the input

```rust
use rumi::prelude::*;

#[derive(Debug)]
struct RegionInput;

impl DataInput<MyRequest> for RegionInput {
    fn get(&self, ctx: &MyRequest) -> MatchingData {
        match ctx.region() {
            Some(r) => MatchingData::String(r.to_string()),
            None => MatchingData::None,
        }
    }

    fn data_type(&self) -> &'static str {
        "string"
    }
}
```

Return `MatchingData::None` when the value is absent. The predicate will evaluate
to `false`, which is what you want. Do not panic, and do not substitute a
sentinel like the empty string, because a rule matching `""` would then fire on
missing data.

## Register it

Inputs reach configs through the registry, keyed by type URL:

```rust
let registry = RegistryBuilder::new()
    .input::<RegionInput>("myapp.v1.RegionInput")
    .build();
```

The registry is immutable once built. Register everything before calling
`build()`, and the compiler will stop you from adding more afterwards, because
`build` consumes the builder.

## Use it

```yaml
input:
  type_url: myapp.v1.RegionInput
value_match: { Exact: "eu-west-1" }
```

## Matching on something that is not a scalar

When the value has no scalar representation, implement `CustomMatchData` and
wrap it:

```rust
MatchingData::Custom(Arc::new(GeoLocation { lat, lon }))
```

`Arc`, not `Box`. Identity comparison relies on it, and the trait carries the
`Send + Sync + Debug` bounds the FFI boundary needs. You then pair it with an
`InputMatcher` that knows how to compare two `GeoLocation` values, which is where
proximity matching or similarity scoring would live.

## The trade-off

A custom input is a new extension point in your config surface. Every consumer of
that config now needs the same input registered, or loading fails. Keep custom
inputs few and name their type URLs carefully, because the config format is
frozen across all five implementations.

## Next

- [Share one config across languages](share-config.md) once the input exists in each runtime.
