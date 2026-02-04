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

The HTTP domain uses Envoy's ext_proc `ProcessingRequest`/`ProcessingResponse` as the universal HTTP processing model.

This covers all HTTP-based protocols:
- REST
- gRPC
- GraphQL
- WebSocket (handshake only)

### Architecture

```text
┌─────────────────────────────────────────┐
│         User-Facing Config API          │  ← DX-friendly match schema
│      (Gateway API inspired)             │
└────────────────────┬────────────────────┘
                     │ compiles to
┌────────────────────▼────────────────────┐
│           x.uma Matchers                │  ← rumi engine
│   DataInput + Predicate + OnMatch       │
└────────────────────┬────────────────────┘
                     │ operates on
┌────────────────────▼────────────────────┐
│        ProcessingRequest Context        │  ← ext_proc protocol
└─────────────────────────────────────────┘
```

### Design Principles

1. **Match schema is action-agnostic** — same match syntax works for routing, rate limiting, feature flags, auth policies
2. **ext_proc as universal model** — covers all HTTP protocols at the transport layer
3. **Gateway API inspiration** — familiar, DX-friendly configuration

## Contributing

See the [GitHub repository](https://github.com/mox-labs/x.uma) for contribution guidelines.
