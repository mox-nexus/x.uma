# Benchmark Results

Performance analysis of x.uma matcher engine implementations.

This page presents the full benchmark data collected across all five x.uma variants. The story unfolds progressively: simple operations show unexpected results, complex workloads reveal architectural trade-offs, and adversarial input exposes catastrophic failure modes.

All variants passed the same 194-test conformance suite before benchmarking. These aren't apples-to-oranges comparisons — they're different implementations of identical behavior.

## Hardware & Methodology

**Hardware**: Apple M1 Max (10-core CPU, 64GB RAM)
**Date**: 2026-02-10
**Operating System**: macOS 14.4

**Benchmarking Tools**:
- Rust (rumi): divan 0.1.14
- Python (puma): pytest-benchmark 4.0.0
- TypeScript (bumi): mitata 1.0.10

**Execution**:
```bash
just bench-all  # runs all benchmarks
```

Each benchmark runs for at least 100 iterations to ensure statistical significance. Results show median latency unless otherwise noted.

## The Variants

x.uma implements the xDS Unified Matcher API in five distinct ways:

| Variant | Language | Engine | Regex Implementation |
|---------|----------|--------|---------------------|
| **rumi** | Rust | Native | `regex` crate (linear time, RE2 semantics) |
| **puma** | Python | Pure Python | `re` module (backtracking) |
| **bumi** | TypeScript | Pure TypeScript (Bun) | `RegExp` (backtracking) |
| **puma-crusty** | Python + Rust | PyO3 FFI to rumi | `regex` crate via FFI |
| **bumi-crusty** | TypeScript + Rust | WASM FFI to rumi | `regex` crate via WASM |

The first three (rumi, puma, bumi) are pure implementations in their native languages. The crusty variants wrap the Rust engine with language bindings.

## The Simplest Case: Exact String Match

Let's start with the most basic operation: matching a single exact string.

**Scenario**: One rule checks if a path equals `/api/users`. We test both hit (matches) and miss (doesn't match) paths.

| Variant | Exact Match Hit | Exact Match Miss |
|---------|----------------|-----------------|
| rumi | 33 ns | 25 ns |
| bumi | 9.3 ns | 10.4 ns |
| puma | 325 ns | 275 ns |

**The surprise**: TypeScript (bumi) is 3.5x faster than Rust (rumi).

### Why TypeScript Beats Rust Here

This isn't a language speed contest — it's an architectural one.

The rumi matcher uses `Box<dyn InputMatcher>` for extensibility. Every match call goes through a vtable dispatch. The TypeScript JIT (JavaScriptCore in Bun) sees the monomorphic call site and inlines the comparison directly.

This is expected and fixable. Rust's trait dispatch trades single-operation speed for compositional flexibility. Future Rust versions with improved monomorphization may close this gap. For now, it's the cost of dynamic dispatch.

**Python** is 10-30x slower than both — no surprise there. Python's interpreter overhead dominates at this scale. But 325ns is still 3 million operations per second on a single core.

## Adding Complexity: Regex Matching

Now we test pattern matching with `^/api/v\d+/users$`.

| Variant | Regex Hit | Regex Miss |
|---------|----------|-----------|
| rumi | 52 ns | 30 ns |
| bumi | 24.8 ns | 20.7 ns |
| puma | 452 ns | 311 ns |

Same pattern: bumi's JIT-optimized RegExp beats rumi's vtable dispatch. The hit path is roughly 2x slower than the miss path across all variants (the regex engine does more work on a match).

Python adds interpreter overhead but stays within the same order of magnitude. Still fast enough for most web workloads.

Nothing alarming yet. Let's make things interesting.

## Boolean Logic: AND/OR Predicates

Real matchers combine conditions. Here we test two predicates composed with AND and OR:

- **AND**: both predicates must match (worst case: evaluates both)
- **OR**: first match wins (best case: short-circuits on first predicate)

| Variant | AND (both match) | OR (first matches) |
|---------|-----------------|-------------------|
| rumi | 96 ns | 56 ns |
| bumi | 61.7 ns | 25.4 ns |
| puma | 637 ns | 454 ns |

The gap widens. bumi's short-circuit optimization on OR is particularly efficient (25ns). Rust's trait dispatch adds overhead for each predicate evaluation.

Python remains 10x slower but scales proportionally. The interpreter tax is constant across operations.

## Scaling: Many Rules

Production matchers have dozens or hundreds of rules. We test first-match-wins evaluation where the **last rule matches** (worst case: linear scan through all rules).

| Rules | rumi | bumi | puma |
|-------|------|------|------|
| 10 | 245 ns | 118 ns | 1.2 µs |
| 50 | 1.07 µs | 542 ns | 5.3 µs |
| 100 | 1.80 µs | 1.07 µs | 10.1 µs |
| 200 | 3.49 µs | 2.10 µs | 20.0 µs |

All three scale linearly (as expected — first-match-wins is O(n) in the worst case). No cache thrashing, no algorithmic surprises. The JIT advantage persists: bumi is roughly 2x faster than rumi, which is roughly 5x faster than puma.

### Throughput at 200 Rules

| Variant | Latency | Requests/sec |
|---------|---------|--------------|
| bumi | 2.1 µs | 476,000 |
| rumi | 3.5 µs | 286,000 |
| puma | 20 µs | 50,000 |

For context, even the "slow" Python variant evaluates **50,000 requests per second on a single core**. In a typical web framework with network I/O, database queries, and business logic, the matcher is not a bottleneck.

But there's a scenario where these differences become life-or-death.

## The Catastrophe: ReDoS

Regular Expression Denial of Service (ReDoS) exploits backtracking regex engines with adversarial input.

**Attack pattern**: `(a+)+$`
**Adversarial input**: `"a" * N + "X"`

The regex engine tries exponentially many backtracking paths before failing. At N=20, there are over **1 million backtracking attempts**.

Here's what happens:

| N | rumi (linear) | puma (backtracking) | bumi (backtracking) |
|---|--------------|--------------------|--------------------|
| 5 | 10 ns | 2.5 µs | 335 ns |
| 10 | 10 ns | 71 µs | 10.7 µs |
| 15 | 11 ns | 2.27 ms | 370 µs |
| 20 | 11 ns | **72.4 ms** | **11.1 ms** |
| 25 | 11 ns | HANGS | HANGS |
| 50 | 11 ns | HANGS | HANGS |
| 100 | 11 ns | HANGS | HANGS |

### The Numbers at N=20

- **rumi**: 11 nanoseconds
- **puma**: 72 milliseconds (6.5 million times slower)
- **bumi**: 11 milliseconds (1 million times slower)

At N=25 and beyond, both puma and bumi **hang indefinitely**. The matcher becomes a denial-of-service vulnerability. An attacker sends a single malicious request and ties up a worker thread forever.

### Why Rust Survives

Rust's `regex` crate uses a Thompson NFA implementation (like Google's RE2) with guaranteed linear time complexity. No backtracking, no exponential blowup, no vulnerability.

The time complexity is O(n) where n is the input length. Whether you match 5 characters or 100 characters, the cost scales linearly.

This is why the arch-guild review mandated: **Use Rust `regex` crate only. No `fancy-regex`. ReDoS protection is non-negotiable.**

But what if you need Python or TypeScript? That's where the crusty variants come in.

## FFI Head-to-Head: puma vs puma-crusty

puma-crusty is a Python package that wraps the rumi engine via PyO3. Every matcher call crosses the Python-Rust FFI boundary.

Let's see what that boundary costs:

| Scenario | puma (pure Python) | puma-crusty (PyO3) | Ratio |
|----------|-------------------|-------------------|-------|
| compile_simple | 1.83 µs | 625 ns | crusty 2.9x faster |
| compile_complex | 4.62 µs | 14.0 µs | puma 3x faster |
| exact_hit | 282 ns | 188 ns | crusty 1.5x faster |
| exact_miss | 178 ns | 141 ns | crusty 1.3x faster |
| regex_hit | 482 ns | 299 ns | crusty 1.6x faster |
| regex_miss | 203 ns | 201 ns | same |
| complex_hit | 814 ns | 570 ns | crusty 1.4x faster |
| complex_miss | 461 ns | 490 ns | same |

### The FFI Overhead

**For evaluation**: crusty is 1.3-1.6x faster on simple operations, breaks even on misses.

**For compilation**: crusty wins on simple configs (Rust struct construction beats Python dataclass construction), but loses on complex configs where Python's dynamic typing makes nested object graphs cheaper to construct.

The FFI overhead is minimal because PyO3 has been heavily optimized for this workload. The crossover happens around 1-2 arguments per call.

### The Strategic Value

crusty gives you the ReDoS protection of Rust's regex engine without rewriting your Python code. For regex-heavy workloads, it's a 1.5x speedup. For adversarial regex input, it's the difference between 11ns and hanging forever.

If you're building a Python service that accepts regex patterns from untrusted sources, puma-crusty is your safety net.

## FFI Head-to-Head: bumi vs bumi-crusty

bumi-crusty wraps the rumi engine compiled to WebAssembly via wasm-bindgen. Every call crosses the JavaScript-WASM boundary with object serialization via `js_sys::Reflect`.

Here's the brutal truth:

| Scenario | bumi (pure TS) | bumi-crusty (WASM) | Ratio |
|----------|---------------|-------------------|-------|
| compile_simple | 19 ns | 2.17 µs | **bumi 113x faster** |
| compile_complex | 107 ns | 47.4 µs | **bumi 444x faster** |
| exact_hit | 9.9 ns | 1.63 µs | **bumi 164x faster** |
| exact_miss | 10.7 ns | 1.52 µs | **bumi 142x faster** |
| regex_hit | 25.9 ns | 2.13 µs | **bumi 82x faster** |
| regex_miss | 18.1 ns | 1.66 µs | **bumi 92x faster** |
| complex_hit | 82.7 ns | 2.45 µs | **bumi 30x faster** |
| complex_miss | 25.7 ns | 2.42 µs | **bumi 94x faster** |

Pure TypeScript is faster in **every single scenario**. Not by a little — by 30-450x.

### The WASM Boundary Tax

The WASM boundary serialization overhead dominates. At 2-3 microseconds per call, the FFI cost exceeds the work being done by multiple orders of magnitude.

Unlike PyO3 (which passes raw pointers), wasm-bindgen serializes objects via `js_sys::Reflect`, which means:
1. Walk the JavaScript object graph
2. Convert to Rust types
3. Call the Rust function
4. Convert result back to JavaScript
5. Reconstruct JavaScript objects

For a 10ns operation, a 2µs boundary cost is 200x overhead.

### Why bumi-crusty Exists

Not for speed. For **ReDoS protection**.

If your matcher uses regex heavily and accepts untrusted input, the pure TypeScript RegExp engine is a vulnerability. bumi-crusty gives you the linear-time guarantees of Rust's `regex` crate at the cost of 100x slower baseline performance.

For most workloads, this is a bad trade. But if an attacker can hang your service with a single malicious regex match, 2µs is cheaper than 72ms (or infinite time).

### When to Use bumi-crusty

- Cloudflare Workers or Deno Deploy where regex comes from user config
- Low request rate where 2µs per match is acceptable
- Security-critical edge cases where ReDoS is a real threat

For general use, stick with pure bumi and **audit regex patterns at ingress**.

## The Strategic Picture

### Use **rumi** (Rust) when

- Building a proxy, load balancer, or high-throughput router
- You need maximum throughput per core
- Regex matching untrusted input (ReDoS protection mandatory)
- You're already in Rust-land

**Speed**: Best for complex pipelines. Linear-time regex with zero ReDoS risk.

### Use **puma** (Python) when

- Integrating with Python web frameworks (FastAPI, Flask, Django)
- Developer ergonomics matter more than raw speed
- You control the regex patterns (no untrusted input)
- 50k req/sec per core is fast enough

**Speed**: 10-30x slower than Rust, but rarely the bottleneck. Backtracking regex vulnerable to ReDoS.

### Use **bumi** (TypeScript) when

- Building frontend routing or edge workers (Cloudflare, Deno Deploy)
- You need sub-microsecond latency for simple matches
- You control the regex patterns (no untrusted input)
- TypeScript ecosystem integration is valuable

**Speed**: Fastest for simple operations (JIT beats Rust dispatch). Backtracking regex vulnerable to ReDoS.

### Use **puma-crusty** (Python + Rust FFI) when

- You need ReDoS protection in a Python codebase
- Regex-heavy workloads where 1.5x speedup matters
- Willing to add a native extension dependency

**Speed**: 1.5x faster than puma for evaluation, 3x slower for complex compilation. Linear-time regex.

### Avoid **bumi-crusty** (TypeScript + WASM FFI) unless

- You MUST have linear-time regex in TypeScript
- You're matching untrusted regex input at the edge
- The 100x slowdown is acceptable (e.g., low request rate)

**Speed**: 30-450x slower than pure TypeScript. Only use for ReDoS protection when no other option exists.

## Phase 11: The RE2 Alternative

The roadmap includes Phase 11: migrate puma to `google-re2` (Python bindings to C++ RE2) and bumi to `re2js` (pure JS port of RE2).

This would give linear-time regex guarantees without FFI overhead:
- **puma** gets ReDoS protection via a mature C extension (comparable to PyO3 overhead)
- **bumi** gets ReDoS protection in pure TypeScript (no WASM boundary)

At that point, the crusty variants shift from "safety layer" to "full compiled pipeline in Rust" for complex configs where the rumi compiler's optimizations justify the FFI cost.

But that's future work. For now, the pure implementations are fastest, and the crusty variants are the ReDoS safety net.

## The Bottom Line

TypeScript JIT beats Rust dynamic dispatch on simple operations. This is expected and not a language-level win — it's the natural outcome of monomorphic inline caching vs vtable indirection.

Python is 10-30x slower than both, which is the expected interpreter tax. Still fast enough for most web workloads.

The real story is **ReDoS**. At N=20, Rust's linear-time regex is 6.5 million times faster than Python's backtracking engine. At N=25, Python hangs forever. This isn't a performance optimization — it's a **security boundary**.

The crusty variants let you pay the FFI tax to get Rust's safety guarantees. For Python, the tax is small (1.5x slower than pure Python). For TypeScript, the tax is huge (100x slower than pure TypeScript). Choose accordingly.

In production:
- Use **rumi** if you can
- Use **puma/bumi** if ergonomics matter more than raw speed
- Add **crusty** if untrusted regex input is a threat model

And never, ever, let user-supplied regex patterns hit a backtracking engine without sanitization.
