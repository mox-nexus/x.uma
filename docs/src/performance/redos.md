# ReDoS Protection

At N=20, Python takes 72 milliseconds. At N=25, it hangs forever.

Regular Expression Denial of Service (ReDoS) exploits the exponential time complexity of backtracking regex engines. A single malicious input can hang a worker thread indefinitely, turning a matcher into a denial-of-service vulnerability.

This page explains the attack, shows the data, and documents which x.uma variants are safe.

## The Attack Pattern

Consider this regex: `(a+)+$`

Match it against this input: `"a" * N + "X"`

The pattern says "one or more a's, repeated one or more times, followed by end of string." The input is N a's followed by a single X (which makes the match fail).

A backtracking regex engine explores every possible grouping:
- Group 1: `[aaa]`, Group 2: `[aa]`, Group 3: `[a]` — fail, try next
- Group 1: `[aa]`, Group 2: `[aaa]`, Group 3: `[a]` — fail, try next
- Group 1: `[aa]`, Group 2: `[aa]`, Group 3: `[aa]` — fail, try next
- ... and so on ...

The number of backtracking attempts grows exponentially: **O(2^N)**.

At N=10, there are ~1,000 attempts. At N=20, over **1 million**. At N=30, over **1 billion**.

## The Data

Here's what happens when you run this attack against x.uma variants:

| N | rumi (linear) | puma (backtracking) | bumi (backtracking) |
|---|--------------|--------------------|--------------------|
| 5 | 10 ns | 2.5 µs | 335 ns |
| 10 | 10 ns | 71 µs | 10.7 µs |
| 15 | 11 ns | 2.27 ms | 370 µs |
| 20 | 11 ns | **72.4 ms** | **11.1 ms** |
| 25 | 11 ns | HANGS | HANGS |
| 30 | 11 ns | HANGS | HANGS |
| 50 | 11 ns | HANGS | HANGS |
| 100 | 11 ns | HANGS | HANGS |

### Visualizing the Catastrophe

From N=5 to N=20, puma's time grows by **28,960x**:
- N=5: 2.5 microseconds
- N=10: 71 microseconds (28x slower)
- N=15: 2.27 milliseconds (32x slower)
- N=20: 72.4 milliseconds (32x slower)

Each +5 increase in N multiplies the time by 30x. This is the exponential blowup.

At N=25, the benchmark times out. The regex engine enters an effectively infinite loop, trying 33 million+ backtracking paths. The worker thread never returns.

### Why Rust Survives

Rust's `regex` crate uses a **Thompson NFA** implementation (like Google's RE2). It compiles the regex to a finite automaton and simulates it in linear time.

Time complexity: **O(n)** where n is the input length.

There is no backtracking. The engine walks the input once, tracking all possible states in parallel. At N=5 or N=100, the cost scales linearly:

| N | rumi time |
|---|-----------|
| 5 | 10 ns |
| 20 | 11 ns |
| 50 | 11 ns |
| 100 | 11 ns |

The tiny variation (10-11ns) is CPU cache noise. The actual time is constant because the NFA has a fixed number of states regardless of input length.

## How Backtracking Works

A backtracking regex engine tries to match by exploring possible paths:

1. **Try a match** — attempt the first branch
2. **On failure, backtrack** — undo the last decision and try the next branch
3. **Repeat** until success or exhaustion

For patterns with nested quantifiers like `(a+)+`, the number of paths explodes:

```
Input: aaaaaX  (N=5)
Pattern: (a+)+$

Attempt 1: [aaaaa] — fail (X doesn't match $)
Attempt 2: [aaaa] [a] — fail
Attempt 3: [aaa] [aa] — fail
Attempt 4: [aaa] [a] [a] — fail
Attempt 5: [aa] [aaa] — fail
Attempt 6: [aa] [aa] [a] — fail
... 26 more attempts ...
```

At N=5, there are **32 paths** (2^5). At N=20, there are **1,048,576 paths** (2^20).

## Why Thompson NFA Doesn't Backtrack

A Thompson NFA tracks **all possible states simultaneously**. Instead of trying paths sequentially, it evaluates them in parallel:

```
State set at position 0: {start}
State set at position 1: {group1, group2}
State set at position 2: {group1, group2}
...
State set at position N: {group1, group2}
State set at position N+1: {} — no valid transitions, match fails
```

The number of states is fixed (determined by the regex structure, not the input length). Walking N characters takes O(n) time.

There's no backtracking because the engine never "commits" to a single path. It explores all paths at once in a single forward pass.

## Which Variants Are Safe?

| Variant | Regex Engine | Time Complexity | Safe? |
|---------|-------------|-----------------|-------|
| **rumi** | Rust `regex` crate | O(n) linear | Yes |
| **puma** | Python `re` module | O(2^n) backtracking | No |
| **bumi** | JavaScript `RegExp` | O(2^n) backtracking | No |
| **puma-crusty** | Rust `regex` via PyO3 | O(n) linear | Yes |
| **bumi-crusty** | Rust `regex` via WASM | O(n) linear | Yes |

The **crusty variants** wrap the Rust engine with language bindings. They inherit the linear-time guarantees despite running in Python or TypeScript.

### FFI Cost vs Safety

puma-crusty adds minimal FFI overhead (1.5x slower than pure Python for simple matches) but gains infinite safety on adversarial input:

| Scenario | puma (pure) | puma-crusty (FFI) | Difference |
|----------|-------------|------------------|------------|
| N=5 (normal) | 2.5 µs | 10 ns | crusty 250x faster |
| N=20 (attack) | 72 ms | 11 ns | crusty 6.5M times faster |
| N=25 (critical) | HANGS | 11 ns | crusty prevents DoS |

bumi-crusty adds significant FFI overhead (100x slower than pure TypeScript) but prevents the hang:

| Scenario | bumi (pure) | bumi-crusty (FFI) | Difference |
|----------|-------------|------------------|------------|
| N=5 (normal) | 335 ns | 10 ns | crusty 33x faster |
| N=20 (attack) | 11 ms | 11 ns | crusty 1M times faster |
| N=25 (critical) | HANGS | 11 ns | crusty prevents DoS |

For adversarial input, the FFI overhead is irrelevant. The choice is between 11ns and infinite time.

## When ReDoS Matters

### Trusted Patterns (You Control the Regex)

**Safe to use**: puma, bumi

If you write the regex patterns and deploy them with your code, you won't accidentally write exponential patterns. Developers rarely write `(a+)+` unless they're explicitly testing ReDoS.

Example trusted scenarios:
- HTTP routing rules in your application config
- Policy rules deployed via CI/CD
- Hardcoded matchers in your codebase

In these cases, the pure Python or TypeScript implementations are safe and fast.

### Untrusted Patterns (Regex Comes from External Source)

**Unsafe to use**: puma, bumi
**Must use**: rumi, puma-crusty, or bumi-crusty

If regex patterns come from user input, external config, or any source you don't control, an attacker can inject malicious patterns.

Example untrusted scenarios:
- User-facing "advanced search" with regex support
- Multi-tenant SaaS where customers define their own routing rules
- Plugin systems where third-party code supplies patterns

In these cases, backtracking engines are a **critical vulnerability**. An attacker sends a single request with a malicious pattern and hangs your service.

### Untrusted Input (Data Comes from Network)

**Safe to use**: any variant (if patterns are trusted)

If the input data is untrusted but the patterns are trusted, ReDoS is not a concern. The attacker can't craft a regex — they can only send strings to match against your patterns.

Standard regex patterns like `^/api/v\d+/users$` are safe even on adversarial input because they don't have nested quantifiers.

Example:
- HTTP path matching in a web server (patterns are your routing rules)
- Header validation in a proxy (patterns are your security rules)

The key distinction: **who controls the pattern?**

## Mitigation Strategies

### 1. Use a Safe Variant

The most reliable mitigation is to use rumi, puma-crusty, or bumi-crusty for any matcher that processes untrusted patterns.

**Cost**: FFI overhead (minimal for Python, significant for TypeScript)
**Benefit**: Guaranteed linear time, zero ReDoS risk

### 2. Audit Patterns at Ingress

If you must use puma or bumi with untrusted patterns, validate patterns before they reach the matcher:

```python
import re

DANGEROUS_PATTERNS = [
    r'\([^)]+\)\+\+',  # (x++)
    r'\([^)]+\+\)\+',  # (x+)+
    r'\*\+',           # *+
    r'\+\+',           # ++
]

def is_safe_pattern(pattern: str) -> bool:
    for dangerous in DANGEROUS_PATTERNS:
        if re.search(dangerous, pattern):
            return False
    return True

# At ingress
if not is_safe_pattern(user_pattern):
    raise ValueError("Pattern contains exponential quantifiers")
```

**Cost**: Heuristic may reject valid patterns, maintenance burden
**Benefit**: No FFI overhead, works with pure implementations

This is fragile. Attackers may find patterns your heuristics miss. Use as a defense-in-depth measure, not the primary mitigation.

### 3. Timeout Wrappers

Wrap matcher evaluation in a timeout:

```python
import signal

def with_timeout(func, timeout_ms):
    def handler(signum, frame):
        raise TimeoutError("Matcher exceeded timeout")

    signal.signal(signal.SIGALRM, handler)
    signal.setitimer(signal.ITIMER_REAL, timeout_ms / 1000.0)
    try:
        return func()
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)

# Usage
result = with_timeout(lambda: matcher.evaluate(req), timeout_ms=100)
```

**Cost**: Adds signal overhead, doesn't prevent resource exhaustion (CPU burns for 100ms)
**Benefit**: Prevents infinite hangs, allows graceful degradation

This limits the blast radius but doesn't prevent the attack. The CPU still burns cycles on backtracking before the timeout fires.

### 4. Phase 11: RE2 Migration

The roadmap includes migrating puma to `google-re2` (Python bindings to C++ RE2) and bumi to `re2js` (pure JavaScript port of RE2).

This would give linear-time regex without FFI overhead:
- **puma**: ReDoS protection via mature C extension (similar perf to PyO3)
- **bumi**: ReDoS protection in pure TypeScript (no WASM boundary)

At that point, all five variants would have linear-time regex. The crusty variants would shift from "safety layer" to "full Rust pipeline" for complex configs.

**Timeline**: Phase 11 is planned but not yet scheduled.

## Real-World Impact

ReDoS is not theoretical. Production systems have been taken down by this attack:

- **Stack Overflow** (2016): ReDoS in regex parsing caused 10-minute outage
- **Cloudflare** (2019): ReDoS in WAF regex caused global outage
- **npm** (multiple incidents): Package name validation regex caused service degradation

The pattern is always the same:
1. Trusted system uses a backtracking regex
2. Input grows slightly larger than expected
3. CPU pins at 100% for seconds/minutes
4. Worker threads hang, queue backs up, service dies

The fix is always the same: migrate to linear-time regex.

## The Bottom Line

ReDoS is a **security boundary**, not a performance optimization.

For trusted patterns, any variant works. For untrusted patterns, backtracking engines are a **critical vulnerability**.

The data is unambiguous:
- At N=20: rumi is 6.5 million times faster than puma
- At N=25: puma hangs forever, rumi returns in 11 nanoseconds

If an attacker can control regex patterns in your matcher, you must use:
- rumi (Rust native)
- puma-crusty (Python + Rust FFI)
- bumi-crusty (TypeScript + WASM FFI)

If you control all patterns, use the pure implementation for your language. The performance is better and the risk is zero.

And never, ever, let user-supplied regex patterns hit a backtracking engine without sanitization.

## Related Pages

- [Security Model](security.md) — Full security guarantees and attack scenarios
- [Benchmark Results](benchmarks.md) — Complete ReDoS benchmark data
- [Performance Guide](guide.md) — Which variant to use for your threat model
