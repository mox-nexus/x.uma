# Architecture

Why x.uma is built the way it is.

## Design Philosophy: ACES

**A**daptable · **C**omposable · **E**xtensible · **S**ustainable

```text
┌─────────────────────────────────────┐
│         Domain Adapters             │
│   xuma.http  xuma.claude  xuma.grpc │
└───────────────┬─────────────────────┘
                │
┌───────────────▼─────────────────────┐
│              PORTS                  │
│     DataInput       ActionPort      │
│   (extract data)  (emit result)     │
└───────────────┬─────────────────────┘
                │
┌───────────────▼─────────────────────┐
│              CORE                   │
│           rumi engine               │
│     Matcher · Predicate · Tree      │
│       (pure, domain-agnostic)       │
└─────────────────────────────────────┘
```

## The Extension Seam

`TypedExtensionConfig` from xDS is the architectural seam:

```protobuf
message TypedExtensionConfig {
  string name = 1;                       // adapter identifier
  google.protobuf.Any typed_config = 2;  // adapter config
}
```

Every `input` and `action` is a port. Adapters are concrete registered types.

## Why Type Erasure at Data Level

Key insight from the spike phase: erase types at the **data level**, not the predicate level.

```rust
// MatchingData — the erased type
pub enum MatchingData { None, String(String), Int(i64), ... }

// DataInput — domain-specific, returns erased type
pub trait DataInput<Ctx> {
    fn get(&self, ctx: &Ctx) -> MatchingData;
}

// InputMatcher — domain-agnostic, NON-GENERIC
pub trait InputMatcher {
    fn matches(&self, value: &MatchingData) -> bool;
}
```

**Why this works:**
- `InputMatcher` is non-generic → same `ExactMatcher` works everywhere
- No GATs or complex lifetimes
- Battle-tested at Google scale (Envoy uses this approach)

## Crate Structure

```text
rumi/
├── rumi-core/      # Pure engine, no_std + alloc
├── rumi-proto/     # Proto types + registry
├── rumi-domains/   # Feature-gated adapters
└── rumi/           # Facade crate
```

Dependencies point inward. Core knows nothing about domains.
