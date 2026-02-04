# Roadmap

Current development status and planned phases.

## Status

| Phase | Focus | Status |
|-------|-------|--------|
| 0 | Scaffolding | ✅ Done |
| 1 | Core Traits | ✅ Done |
| 2 | Conformance Fixtures | ✅ Done |
| 2.5 | Extensible MatchingData (`Custom` variant) | ✅ Done |
| 3 | StringMatcher, MatcherTree, RadixTree | ✅ Done |
| 4 | HTTP Domain (ext_proc model) | 🚧 Next |
| 5 | p.uma (Pure Python + HTTP) | Planned |
| 6 | b.uma (Bun/TypeScript + HTTP) | Planned |
| 7 | crusty/p.uma (uniffi→Python) | Planned |
| 8 | crusty/b.uma (uniffi→WASM) | Planned |
| 9 | Benchmarks | Planned |

## Phase 4: HTTP Domain

### Overview

The HTTP domain provides matching for all HTTP-based protocols using two layered APIs:

| Layer | Standard | Purpose |
|-------|----------|---------|
| **User API** | Gateway API `HTTPRouteMatch` | Config-time, DX-friendly YAML/JSON |
| **Data Plane API** | ext_proc `ProcessingRequest` | Runtime context from Envoy |

### Why Two Layers?

**Gateway API HTTPRouteMatch** (CNCF Standard)
- What users write in config files
- Clean YAML/JSON schema
- Adopted by: Istio, Envoy Gateway, Contour, Kong, etc.
- We use it as-is, not reinvent

**ext_proc ProcessingRequest** (Envoy)
- Universal HTTP processing model
- Covers: REST, gRPC, GraphQL, WebSocket handshake
- What matchers actually operate on at runtime

### Architecture

```text
┌─────────────────────────────────────────────────────┐
│              Gateway API HTTPRouteMatch             │
│  ─────────────────────────────────────────────────  │
│  path:                                              │
│    type: PathPrefix                                 │
│    value: /api/v2                                   │
│  headers:                                           │
│  - name: x-api-key                                  │
│    type: Exact                                      │
│    value: secret123                                 │
└─────────────────────┬───────────────────────────────┘
                      │ compile() — config time
                      ▼
┌─────────────────────────────────────────────────────┐
│            rumi Matcher<ProcessingRequest, A>       │
│  ─────────────────────────────────────────────────  │
│  DataInputs: PathInput, HeaderInput, MethodInput    │
│  Predicates: composed with AND/OR/NOT               │
│  Actions: domain-specific (routing, rate limit...)  │
└─────────────────────┬───────────────────────────────┘
                      │ evaluate() — runtime
                      ▼
┌─────────────────────────────────────────────────────┐
│              ext_proc ProcessingRequest             │
│  ─────────────────────────────────────────────────  │
│  request_headers / response_headers                 │
│  request_body / response_body                       │
│  request_trailers / response_trailers               │
└─────────────────────────────────────────────────────┘
```

### Components

#### 1. DataInputs for ProcessingRequest

```rust,ignore
// Extract data from ProcessingRequest for matching
impl DataInput<ProcessingRequest> for PathInput { ... }
impl DataInput<ProcessingRequest> for MethodInput { ... }
impl DataInput<ProcessingRequest> for HeaderInput { ... }
impl DataInput<ProcessingRequest> for QueryParamInput { ... }
```

#### 2. Compiler: HTTPRouteMatch → Matcher

```rust,ignore
// Config time: compile Gateway API config to rumi matcher
let config: HTTPRouteMatch = load_yaml("route.yaml");
let matcher: Matcher<ProcessingRequest, Action> = config.compile();
```

#### 3. Runtime Evaluation

```rust,ignore
// Runtime: fast path evaluation
let action = matcher.evaluate(&processing_request);
```

### Match Schema is Action-Agnostic

The same `HTTPRouteMatch` works for different use cases:

| Use Case | Match | Action |
|----------|-------|--------|
| Routing | path, headers | → backend selection |
| Rate Limiting | path, headers, user | → limit config |
| Feature Flags | headers, % rollout | → enable features |
| Auth Policy | path, method | → require auth |
| Observability | all requests | → add tracing |

x.uma provides the **match vocabulary**. Actions are domain-specific.

### Deliverables

- [ ] `rumi-http` crate with `ProcessingRequest` context
- [ ] DataInputs: `PathInput`, `MethodInput`, `HeaderInput`, `QueryParamInput`
- [ ] Gateway API `HTTPRouteMatch` schema (import or mirror)
- [ ] Compiler: `HTTPRouteMatch` → `Matcher<ProcessingRequest, A>`
- [ ] Conformance tests against Gateway API spec

### Research

See `.claude/research/` for detailed research on:
- Gateway API schema structure
- ext_proc ProcessingRequest fields
- Adoption patterns from Istio, Envoy Gateway

## Contributing

See the [GitHub repository](https://github.com/mox-labs/x.uma) for contribution guidelines.
