# Why ACES

The design principles behind x.uma.

## The Problem

Matchers are everywhere: routing, filtering, access control, feature flags. But most implementations are:

- **Tightly coupled** to one domain (HTTP only, or gRPC only)
- **Hard to extend** without forking
- **Inconsistent** across languages

## ACES Principles

### Adaptable

New domains plug in without touching core.

```text
Adding HTTP support? → Create rumi-domains/http.rs
Adding Claude hooks? → Create rumi-domains/claude.rs
Core unchanged.
```

### Composable

Matchers nest. Predicates AND/OR/NOT. Trees recurse.

```rust
Predicate::And(vec![
    Predicate::single(PathInput, PrefixMatcher::new("/api/")),
    Predicate::Or(vec![
        Predicate::single(MethodInput, ExactMatcher::new("GET")),
        Predicate::single(MethodInput, ExactMatcher::new("POST")),
    ]),
])
```

### Extensible

`TypedExtensionConfig` is the extension seam. Register new types at startup, use them in configs.

```yaml
input:
  "@type": "type.googleapis.com/xuma.http.v1.HeaderInput"
  header_name: "authorization"
```

### Sustainable

Core is stable. Growth happens at edges.

- **v1.0** → Core traits locked
- **v1.x** → New domains, new matchers, same core
- **No rewrites** → Hexagonal architecture pays off

## Inspired By

- **Envoy** — Production-proven matcher implementation at Google scale
- **xDS** — Protocol that powers service mesh configuration
- **Hexagonal Architecture** — Ports and adapters pattern
