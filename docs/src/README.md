# x.uma

> Cross-Platform Unified Matcher API

**Alpha status** — API is under active development and will change.

x.uma implements the [xDS Unified Matcher API](https://github.com/cncf/xds) across multiple languages. Match structured data (HTTP requests, events, messages) against rule trees with first-match-wins semantics.

## Implementations

| Package | Language | Status |
|---------|----------|--------|
| **rumi** | Rust | Production-ready core |
| **puma** | Python 3.12+ | Alpha (v0.1.0) |
| **bumi** | TypeScript/Bun | Planned |
| **puma-crusty** | Python (Rust bindings) | Planned |
| **bumi-crusty** | TypeScript (WASM bindings) | Planned |

All implementations pass the same conformance test suite.

## Quick Example

The same pattern in Rust and Python.

### Rust

```rust,ignore
use rumi::prelude::*;
use rumi_http::{HttpMessage, PathInput, PrefixMatcher};

let matcher = Matcher::new(
    vec![
        FieldMatcher::new(
            Predicate::Single(SinglePredicate::new(
                Box::new(PathInput),
                Box::new(PrefixMatcher::new("/api")),
            )),
            OnMatch::Action("api_handler"),
        ),
    ],
    Some(OnMatch::Action("default")),
);

// ProcessingRequest -> HttpMessage -> evaluate
let action = matcher.evaluate(&http_message);
```

### Python

```python
from puma import Matcher, FieldMatcher, SinglePredicate, Action
from puma import PrefixMatcher
from puma.http import HttpRequest, PathInput

matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(
                input=PathInput(),
                matcher=PrefixMatcher("/api")
            ),
            on_match=Action("api_handler")
        ),
    ),
    on_no_match=Action("default")
)

request = HttpRequest(method="GET", raw_path="/api/users")
action = matcher.evaluate(request)  # "api_handler"
```

## Architecture

x.uma follows hexagonal architecture (ports and adapters). The core is domain-agnostic. Domains plug in at the edges.

```text
┌─────────────────────────────────┐
│       Domain Adapters           │
│   HTTP  CloudEvent  Custom      │
└───────────────┬─────────────────┘
                │
┌───────────────▼─────────────────┐
│           PORTS                 │
│  DataInput[Ctx] → MatchingValue │
│  InputMatcher → bool            │
└───────────────┬─────────────────┘
                │
┌───────────────▼─────────────────┐
│           CORE                  │
│  Matcher, Predicate, Actions    │
│    (domain-agnostic)            │
└─────────────────────────────────┘
```

**DataInput** extracts values from your context (HTTP request, event, custom type).

**InputMatcher** matches the extracted value (exact, prefix, regex, etc).

**Matcher** composes predicates with first-match-wins semantics.

The same `ExactMatcher` works for HTTP headers, event types, or your custom domain. This is the key design insight from Envoy's matcher architecture.

## Why x.uma?

**Domain-agnostic core.** The same matcher logic works for HTTP routing, event filtering, access control, or your custom domain. Add a new domain by implementing `DataInput` for your context type.

**Type-safe composition.** Predicates compose with AND/OR/NOT. Matchers nest. Actions are exclusive (action XOR nested matcher, never both).

**Battle-tested semantics.** Built on the same xDS protocol Envoy uses at Google scale. First-match-wins, nested matcher failure propagation, depth limits — all enforced by design.

**Multi-language.** Same API across Rust, Python, and TypeScript. Write matchers once, run anywhere.

## Documentation

| Section | What you'll learn |
|---------|-------------------|
| [Quick Start](learn/quick-start.md) | Get running in 5 minutes (Rust and Python) |
| [Concepts](learn/concepts.md) | Core abstractions and terminology |
| [Architecture](explain/architecture.md) | Why it's built this way |
| [Proto API](reference/proto.md) | xDS protocol definitions |
| [Rust API](reference/rust.md) | rumi API reference |
| [Python API](reference/python.md) | puma API reference |

## Status

x.uma is alpha software. The API is under active development and will change.

| Phase | Focus | Status |
|-------|-------|--------|
| 5 | Python (puma) | ✅ Done (v0.1.0) |
| 6 | TypeScript (bumi) | Next |
| 7 | Python Rust bindings (puma-crusty) | Planned |
| 8 | TypeScript WASM bindings (bumi-crusty) | Planned |

See [Roadmap](development/roadmap.md) for details.
