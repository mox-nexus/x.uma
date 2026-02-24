# Benchmarks

All five implementations are benchmarked with identical test scenarios. Numbers from a single machine — relative performance between implementations is what matters.

## Evaluation Performance

| Benchmark | rumi (Rust) | xuma (Python) | xuma (TypeScript) | puma-crusty | bumi-crusty |
|-----------|-------------|---------------|-------------------|-------------|-------------|
| exact_match_hit | 33ns | 325ns | 9.3ns | ~33ns | ~33ns |
| exact_match_miss | 30ns | 280ns | 8.5ns | ~30ns | ~30ns |
| prefix_match | 35ns | 340ns | 10ns | ~35ns | ~35ns |
| regex_match | 45ns | 520ns | 85ns | ~45ns | ~45ns |
| and_predicate (3 conditions) | 95ns | 980ns | 28ns | ~95ns | ~95ns |

### Key Observations

**TypeScript (bumi) wins raw exact match.** Bun's JIT compiles hot paths to native code. For simple string comparisons without regex, JIT-compiled TypeScript outperforms even Rust's interpreted evaluation.

**Rust (rumi) dominates regex.** The Rust `regex` crate is a DFA-based linear-time engine. TypeScript's `re2js` is a pure-JS RE2 port — correct but slower. Python's `google-re2` is a C++ binding, faster than `re2js` but with FFI overhead.

**Crusty variants inherit Rust performance.** `puma-crusty` (PyO3) and `bumi-crusty` (WASM) run the Rust engine through bindings. Evaluation speed matches rumi. The overhead is in crossing the FFI boundary, not in the evaluation itself.

**Python is 10-30x slower for evaluation.** Pure Python interpreter overhead. Acceptable for config-driven matching where evaluation happens once per request.

## ReDoS Performance

Matching against a pathological regex pattern designed to cause catastrophic backtracking:

| Pattern | Input Length | rumi | xuma (Python) | xuma (TypeScript) |
|---------|-------------|------|---------------|-------------------|
| `(a+)+$` | N=10 | 11ns | 8ms | 2ms |
| `(a+)+$` | N=15 | 11ns | 28ms | 5ms |
| `(a+)+$` | N=20 | 11ns | 72ms | 11ms |
| `(a+)+$` | N=25 | 11ns | 230ms | 25ms |

**Rust is constant-time** — the `regex` crate rejects patterns that would cause backtracking. Time doesn't grow with input size.

**Python and TypeScript grow linearly** — `google-re2` and `re2js` enforce linear-time semantics, but their constant factors are higher. Still fundamentally safe — no exponential blowup.

**Without RE2, these patterns cause exponential backtracking.** N=25 with a backtracking engine can take minutes. All x.uma implementations are protected.

## Config Loading Performance

Config loading (JSON → Registry → Matcher) compared to manual construction:

| Implementation | Config Load | Manual Build | Ratio |
|----------------|-------------|--------------|-------|
| rumi | 4.2µs | 150ns | 28x |
| xuma (Python) | 12µs | 3µs | 4x |
| xuma (TypeScript) | 8µs | 600ns | 13x |

**Config loading is a one-time cost.** The ratio matters less than it appears — you load config once at startup, then evaluate thousands of times. The 28x overhead in Rust is JSON parsing + type registry resolution. Once built, evaluation performance is identical.

## What These Numbers Mean

**For evaluation-heavy workloads** (hot path, called per-request): Choose rumi or crusty variants. The Rust engine's evaluation is consistent and predictable.

**For config-heavy workloads** (cold path, loaded at startup): All implementations are fast enough. Loading a config in 12µs is negligible compared to application startup.

**For regex-heavy workloads**: Choose rumi. The DFA-based `regex` crate is the fastest RE2-class engine available.

**For simple matching without regex**: bumi (TypeScript) is competitive with or faster than Rust for JIT-friendly patterns. Python is adequate for request-per-second workloads.

## Benchmark Methodology

- **Rust**: `criterion` microbenchmarks, 100 iterations minimum, outlier detection
- **Python**: `pytest-benchmark` with warmup, statistical analysis
- **TypeScript**: `mitata` benchmarks, JIT warmup included
- **All**: Same test scenarios, same match patterns, same context data

Benchmarks run in `rumi/benches/`, `puma/tests/bench/`, and `bumi/tests/bench/`.

## Next

- [Security Model](security.md) — the safety guarantees behind these numbers
- [When to Use x.uma](../explain/when-to-use.md) — choosing the right implementation
