# Performance Guide

Which x.uma variant is fastest? It depends on your workload.

The TypeScript implementation (bumi) evaluates an exact string match in 9.3 nanoseconds. The Rust implementation (rumi) takes 33 nanoseconds for the same operation — 3.5x slower. Yet for adversarial regex input at N=20, Rust takes 11 nanoseconds while TypeScript takes 11 milliseconds. That's 1 million times faster.

This guide helps you choose the right variant for your performance requirements and security constraints.

## The Variants

x.uma implements the xDS Unified Matcher API in five ways:

| Variant | Language | When to use |
|---------|----------|-------------|
| **rumi** | Rust (native) | High-throughput routing, untrusted regex, production proxies |
| **puma** | Python (pure) | Web frameworks, dev ergonomics, trusted patterns |
| **bumi** | TypeScript (pure) | Edge workers, frontend routing, sub-microsecond latency |
| **puma-crusty** | Python + Rust FFI | ReDoS protection in Python without full rewrite |
| **bumi-crusty** | TypeScript + WASM | ReDoS protection in TypeScript (100x slower, use sparingly) |

All five variants pass the same 194-test conformance suite. They implement identical behavior with different performance characteristics.

## Decision Matrix

### Use **rumi** (Rust) when

- **Building infrastructure** — proxies, load balancers, API gateways where throughput per core matters
- **Untrusted regex** — patterns come from configuration files, user input, or external sources
- **Maximum throughput** — need to handle 100k+ requests per second per core
- **Already in Rust** — integrating with existing Rust services

**Performance**: 33ns for exact match, 52ns for regex match, 1.8µs for 100 rules. Linear-time regex with zero ReDoS risk.

### Use **puma** (Python) when

- **Python ecosystem** — integrating with FastAPI, Flask, Django, or Python ETL pipelines
- **Dev speed matters** — prototype quickly, optimize later if needed
- **Trusted patterns** — you control all regex patterns (no user input)
- **50k req/sec is enough** — Python's interpreter tax is acceptable for your load

**Performance**: 325ns for exact match, 452ns for regex match, 10.1µs for 100 rules. Backtracking regex vulnerable to ReDoS on adversarial input.

### Use **bumi** (TypeScript) when

- **Edge deployment** — Cloudflare Workers, Deno Deploy, or frontend routing
- **Sub-microsecond latency** — need single-digit nanosecond overhead
- **Trusted patterns** — you control all regex patterns (no user input)
- **TypeScript integration** — leveraging existing TS tooling and type safety

**Performance**: 9.3ns for exact match, 24.8ns for regex match, 1.07µs for 100 rules. Fastest for simple operations. Backtracking regex vulnerable to ReDoS.

### Use **puma-crusty** (Python + Rust FFI) when

- **ReDoS is a threat** — regex patterns come from untrusted sources
- **Python codebase** — can't rewrite everything in Rust
- **Regex-heavy** — matching logic dominates request processing time
- **Native deps OK** — deployment can handle binary wheels

**Performance**: 1.3-1.6x faster than pure Python for evaluation. 2.9x faster for simple config compilation. Zero ReDoS risk.

**Trade-off**: FFI overhead minimal (PyO3 is highly optimized), but adds native dependency to your deployment.

### Avoid **bumi-crusty** (TypeScript + WASM) unless

- **Must have linear-time regex** — ReDoS is critical threat model
- **TypeScript required** — can't switch to rumi or puma-crusty
- **Low request rate** — can tolerate 100x slowdown (2-3µs per match)
- **Security > speed** — preventing denial-of-service is worth the cost

**Performance**: 30-450x slower than pure TypeScript across all operations. WASM boundary serialization overhead dominates.

**Use case**: Edge workers matching untrusted regex where a single malicious request could hang the isolate. Not for general use.

## Throughput Summary

Operations per second on a single core (M1 Max):

| Variant | Exact Match | Regex Match | 100 Rules |
|---------|-------------|-------------|-----------|
| rumi | 30M ops/sec | 19M ops/sec | 555k ops/sec |
| bumi | 107M ops/sec | 40M ops/sec | 934k ops/sec |
| puma | 3M ops/sec | 2.2M ops/sec | 99k ops/sec |
| puma-crusty | 5.3M ops/sec | 3.3M ops/sec | 1.75M ops/sec |
| bumi-crusty | 613k ops/sec | 469k ops/sec | 408k ops/sec |

For HTTP routing in a web framework, even the "slow" Python variant handles 50,000 requests per second per core. This is not a bottleneck for most applications.

The real differentiator is ReDoS protection.

## ReDoS: The Security Boundary

At N=20 characters, a malicious regex input causes:
- **rumi**: 11 nanoseconds (linear time)
- **puma**: 72 milliseconds (6.5 million times slower)
- **bumi**: 11 milliseconds (1 million times slower)

At N=25, both puma and bumi hang indefinitely. A single malicious request ties up a worker thread forever.

This is why rumi mandates the Rust `regex` crate with linear-time guarantees. The arch-guild review marked this as non-negotiable: **ReDoS protection is a security requirement, not a performance optimization.**

See [ReDoS Protection](redos.md) for the full technical deep dive and mitigation strategies.

## When to Switch Variants

Start with the variant that matches your ecosystem:
- **Rust project** → rumi
- **Python project** → puma
- **TypeScript project** → bumi

Switch to a crusty variant if:
1. Regex patterns come from untrusted sources (user input, external config)
2. An attacker could craft malicious patterns to exploit backtracking
3. You can't migrate the entire codebase to Rust

The FFI overhead is minimal for Python (PyO3 is fast) but significant for TypeScript (WASM boundary is expensive). Measure before switching.

## Future: Phase 11 (RE2 Migration)

The roadmap includes migrating puma to `google-re2` (Python bindings to C++ RE2) and bumi to `re2js` (pure JS port of RE2).

This would provide linear-time regex without FFI overhead:
- **puma**: ReDoS protection via mature C extension
- **bumi**: ReDoS protection in pure TypeScript

At that point, the crusty variants shift from "safety layer" to "full compiled pipeline" for complex configs where the rumi compiler's optimizations justify the FFI cost.

Until then, use crusty for ReDoS protection or stick with pure implementations if patterns are trusted.

## Benchmarking Your Workload

The numbers in this guide come from micro-benchmarks on isolated operations. Your production performance depends on:
- **Network I/O** — often dominates request latency
- **Upstream latency** — database, external APIs, other services
- **Rule complexity** — 10 rules vs 1000 rules changes the picture
- **Match rate** — first rule hits vs last rule hits vs no matches

Run benchmarks on representative production configs before making architecture decisions.

All benchmarks use: `just bench-all`

Full data and methodology: [Benchmark Results](benchmarks.md)

## Summary

| Workload | Recommendation |
|----------|----------------|
| High-throughput infrastructure | rumi |
| Python web frameworks (trusted patterns) | puma |
| Edge workers (trusted patterns) | bumi |
| Python + untrusted regex | puma-crusty |
| TypeScript + untrusted regex (rare) | bumi-crusty |
| Any + adversarial regex | rumi or crusty variant |

The fastest variant is the one that matches your ecosystem and security requirements. Start there, measure, then optimize if needed.
