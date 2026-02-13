# Choose Your Implementation

You have five choices. Which one do you need?

x.uma runs in Rust, Python, and TypeScript. Each has a **pure implementation** (native to that language) and a **crusty implementation** (Rust core accessed via FFI). The pure variants are faster for typical workloads. The crusty variants protect against ReDoS attacks when matching untrusted regex patterns.

## Quick Decision Tree

Start here:

```
What language are you using?
├─ Rust → rumi (always)
├─ Python → Do you match untrusted regex input?
│   ├─ Yes → puma-crusty (ReDoS protection)
│   └─ No → puma (1.5x faster than crusty)
└─ TypeScript → Do you match untrusted regex input?
    ├─ Yes → bumi-crusty (ReDoS protection, accepts 100x slowdown)
    └─ No → bumi (100x faster than crusty)
```

## The Five Variants

| Variant | Language | Regex Engine | ReDoS Safe? | FFI Overhead |
|---------|----------|--------------|-------------|--------------|
| **rumi** | Rust | `regex` crate (linear) | Yes | None |
| **puma** | Python | `re` module (backtracking) | No | None |
| **bumi** | TypeScript/Bun | `RegExp` (backtracking) | No | None |
| **puma-crusty** | Python → Rust | `regex` crate via PyO3 | Yes | Minimal (1.3-1.6x) |
| **bumi-crusty** | TypeScript → Rust | `regex` crate via WASM | Yes | Heavy (30-450x) |

All five pass the same 194-test conformance suite. They implement identical behavior with different performance characteristics.

## Performance Spectrum

At 200 routing rules (worst case: last rule matches):

| Variant | Latency | Throughput/core |
|---------|---------|-----------------|
| bumi | 2.1 µs | 475k req/sec |
| rumi | 3.5 µs | 285k req/sec |
| puma | 20 µs | 50k req/sec |
| puma-crusty | 13 µs | 77k req/sec |
| bumi-crusty | 240 µs | 4k req/sec |

These numbers tell half the story. The other half is ReDoS.

## The ReDoS Problem

Regular Expression Denial of Service exploits backtracking regex engines. An attacker sends input designed to trigger exponential backtracking. At N=20 characters, the pathological pattern `(a+)+$` causes:

- **rumi**: 11 nanoseconds (linear-time Thompson NFA)
- **bumi**: 11 milliseconds (backtracking engine)
- **puma**: 72 milliseconds (backtracking engine)

At N=25, both puma and bumi hang indefinitely. One malicious request ties up a worker thread forever.

Rust's `regex` crate (based on RE2 semantics) guarantees linear time. No backtracking, no exponential blowup, no vulnerability. The crusty variants give you this protection in Python and TypeScript.

## When to Use Each

### rumi (Rust)

**Use when:**
- Building a proxy, load balancer, or high-throughput router
- Maximum throughput per core matters
- Matching untrusted regex patterns (ReDoS mandatory)
- Already writing Rust

**Don't use when:**
- Your team doesn't know Rust
- Python or TypeScript would integrate better with existing stack

**Install:**
```toml
[dependencies]
rumi = "0.1"
rumi-http = "0.1"
```

### puma (Python)

**Use when:**
- Integrating with Python web frameworks (FastAPI, Flask, Django)
- Developer ergonomics matter more than raw speed
- You control all regex patterns (no untrusted input)
- 50k requests per second per core is fast enough

**Don't use when:**
- Users can supply regex patterns
- You need to route more than 50k req/sec on a single core

**Install:**
```bash
uv add puma
```

### bumi (TypeScript/Bun)

**Use when:**
- Building frontend routing or edge workers (Cloudflare Workers, Deno Deploy)
- Sub-microsecond latency matters for simple matches
- You control all regex patterns (no untrusted input)
- TypeScript ecosystem integration is valuable

**Don't use when:**
- Users can supply regex patterns
- Running on Node.js (bumi requires Bun runtime)

**Install:**
```bash
bun add @x.uma/bumi
```

### puma-crusty (Python + Rust FFI)

**Use when:**
- Need ReDoS protection in an existing Python codebase
- Regex-heavy workloads where 1.5x speedup matters
- Willing to add a native extension dependency
- Python wheels for your platform exist

**Don't use when:**
- Deployment platform doesn't support native extensions
- Pure Python is fast enough and you control regex patterns

**Install:**
```bash
uv add puma-crusty
```

### bumi-crusty (TypeScript + WASM FFI)

**Use when:**
- MUST have linear-time regex in TypeScript (threat model requires it)
- Matching untrusted regex input at the edge
- The 100x slowdown is acceptable (low request rate, high security requirement)

**Don't use when:**
- You control the regex patterns (use pure bumi instead)
- Performance matters (pure bumi is faster in every scenario)

**Install:**
```bash
bun add @x.uma/bumi-crusty
```

The WASM boundary serialization overhead dominates at 2-3 microseconds per call. This variant exists for threat scenarios, not performance optimization.

## Mixing Variants

You can use different variants in different parts of your system:

- **Edge router** (untrusted traffic): rumi or puma-crusty
- **Internal service mesh** (trusted traffic): puma or bumi
- **Config validation** (build time): any variant, performance doesn't matter

The conformance test suite guarantees identical behavior. A matcher compiled with puma will make the same decisions as the same matcher in rumi.

## Next Steps

Choose your language:

- [Rust Quick Start](rust.md)
- [Python Quick Start](python.md)
- [TypeScript Quick Start](typescript.md)

Or dive deeper:

- [Benchmark Results](../performance/benchmarks.md) — full performance analysis
- [ReDoS Protection](../performance/redos.md) — threat model and mitigations
- [Architecture](../explain/architecture.md) — how x.uma works under the hood
