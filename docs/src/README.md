# x.uma

> Cross-Platform Unified Matcher API

**Alpha status** — API is under active development and will change.

x.uma implements the [xDS Unified Matcher API](https://github.com/cncf/xds) across multiple languages. Match structured data (HTTP requests, events, messages) against rule trees with first-match-wins semantics.

## Implementations

| Package | Language | Status |
|---------|----------|--------|
| **rumi** | Rust | Production-ready core (195 tests) |
| **puma** | Python 3.12+ | Alpha v0.1.0 (194 tests) |
| **bumi** | TypeScript/Bun | Alpha v0.1.0 (202 tests) |
| **puma-crusty** | Python (PyO3 bindings) | Alpha (37 tests) |
| **bumi-crusty** | TypeScript (wasm-bindgen) | Alpha (36 tests) |

All implementations pass the same conformance test suite. **Total: 268 tests** across 5 variants.

## Quick Example

The same pattern in Rust, Python, and TypeScript.

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

### TypeScript

```typescript
import { Matcher, FieldMatcher, SinglePredicate, Action } from "bumi";
import { PrefixMatcher } from "bumi";
import { HttpRequest, PathInput } from "bumi/http";

const matcher = new Matcher([
    new FieldMatcher(
        new SinglePredicate(
            new PathInput(),
            new PrefixMatcher("/api")
        ),
        new Action("api_handler")
    ),
], new Action("default"));

const request = new HttpRequest("GET", "/api/users");
const action = matcher.evaluate(request); // "api_handler"
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

**Performance.** Sub-microsecond evaluation. Linear-time regex (Rust implementations). Zero-copy where possible.

**Security.** ReDoS protection via linear-time regex. Depth limits (max 32 levels). Fail-closed validation.

## Documentation

| Section | What you'll learn |
|---------|-------------------|
| **Getting Started** | |
| [Choose Your Implementation](getting-started/choose.md) | Which variant fits your use case |
| [Rust Quick Start](getting-started/rust.md) | Get rumi running in 5 minutes |
| [Python Quick Start](getting-started/python.md) | Get puma running in 5 minutes |
| [TypeScript Quick Start](getting-started/typescript.md) | Get bumi running in 5 minutes |
| **Tutorials** | |
| [Build an HTTP Router](tutorials/http-router.md) | Step-by-step routing example |
| **Core Concepts** | |
| [The Matching Pipeline](concepts/pipeline.md) | How evaluation works |
| [Type Erasure and Ports](concepts/type-erasure.md) | Why DataInput/InputMatcher split exists |
| [Predicate Composition](concepts/predicates.md) | AND/OR/NOT logic trees |
| [First-Match-Wins Semantics](concepts/semantics.md) | Evaluation order and fallback |
| **Performance & Security** | |
| [Performance Guide](performance/guide.md) | Optimization techniques |
| [Benchmark Results](performance/benchmarks.md) | Cross-language numbers |
| [Security Model](performance/security.md) | Threat model and mitigations |
| [ReDoS Protection](performance/redos.md) | Linear-time regex guarantees |
| **Understanding x.uma** | |
| [Architecture](explain/architecture.md) | Why it's built this way |
| [Why ACES](explain/why-aces.md) | Design philosophy deep dive |
| [When to Use x.uma](explain/when-to-use.md) | x.uma vs OPA vs Cedar vs Zanzibar |
| [Policy Landscape](explain/policy-landscape.md) | Where x.uma fits in the ecosystem |
| **Reference** | |
| [Proto API](reference/proto.md) | xDS protocol definitions |
| [Rust API](reference/rust.md) | rumi API reference |
| [Python API](reference/python.md) | puma API reference |
| [TypeScript API](reference/typescript.md) | bumi API reference |
| [HTTP Domain](reference/http.md) | HTTP matching across languages |

## Status

x.uma is alpha software. The API is under active development and will change.

| Phase | Focus | Status |
|-------|-------|--------|
| 5 | puma (Python) | ✅ Done (v0.1.0) |
| 6 | bumi (TypeScript) | ✅ Done (v0.1.0) |
| 7 | puma-crusty (PyO3 bindings) | ✅ Done |
| 8 | bumi-crusty (WASM bindings) | ✅ Done |
| 9 | Cross-language benchmarks | 🚧 In Progress |
| 10 | Semantic matching (cosine similarity) | Planned |
| 11 | RE2 migration (linear-time regex natively) | Planned |

See [Roadmap](development/roadmap.md) for details.
