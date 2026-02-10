# Security Model

x.uma is a trust boundary. Here's what it guarantees.

A matcher processes untrusted input (HTTP requests, gRPC messages, arbitrary data) and makes routing or policy decisions. A single malicious input should never crash the process, hang a worker thread, or consume unbounded resources.

This page documents the security guarantees and failure modes of the x.uma matcher engine.

## Threat Model

x.uma assumes:
- **Untrusted input data** — HTTP paths, headers, query params come from the network
- **Trusted config** — matcher rules come from deployment config (not user input)
- **Optional: untrusted patterns** — in some deployments, regex patterns may come from user config

The matcher must remain safe under all input conditions. Adversarial input may cause a match to fail, but it must never cause unbounded resource consumption or undefined behavior.

## Depth Limits

**Guarantee**: Nested matchers cannot exceed 32 levels.

**Implementation**: `MAX_DEPTH = 32` enforced at config validation time (not runtime).

### Why Depth Limits Matter

xDS matchers support recursion — an `OnMatch` can contain a nested `Matcher`, which can contain more nested matchers:

```protobuf
Matcher {
  matcher_tree {
    on_match {
      matcher {  // level 1
        on_match {
          matcher {  // level 2
            ...
          }
        }
      }
    }
  }
}
```

Without limits, an attacker could craft a config with 10,000 nested levels. In recursive implementations, this causes stack overflow. In iterative implementations, it causes O(n) validation cost per request.

### Enforcement

All variants validate depth at **config load time**:

```rust
// rumi (Rust)
impl Matcher {
    pub fn validate(&self) -> Result<(), MatcherError> {
        self.validate_depth(0, MAX_DEPTH)
    }
}
```

```python
# puma (Python)
def validate_depth(matcher: Matcher, current: int = 0) -> None:
    if current > MAX_DEPTH:
        raise ValueError(f"Matcher depth {current} exceeds MAX_DEPTH={MAX_DEPTH}")
```

A config that exceeds depth limits is **rejected at construction**. Runtime evaluation never sees invalid configs.

## ReDoS Protection

**Guarantee**: Varies by implementation.

Regular Expression Denial of Service (ReDoS) exploits backtracking regex engines with adversarial input. A pattern like `(a+)+$` against input `"a" * N + "X"` causes exponential backtracking.

### Protection by Variant

| Variant | Regex Engine | Complexity | ReDoS Risk |
|---------|-------------|------------|-----------|
| **rumi** | Rust `regex` crate | O(n) linear | **None** — guaranteed safe |
| **puma** | Python `re` module | O(2^n) backtracking | **High** — vulnerable |
| **bumi** | JavaScript `RegExp` | O(2^n) backtracking | **High** — vulnerable |
| **puma-crusty** | Rust `regex` via FFI | O(n) linear | **None** — guaranteed safe |
| **bumi-crusty** | Rust `regex` via WASM | O(n) linear | **None** — guaranteed safe |

### When It Matters

**Trusted patterns** (you control the regex):
- puma and bumi are safe — you won't write exponential patterns
- No ReDoS risk because attacker can't control the pattern

**Untrusted patterns** (regex comes from user config or external source):
- puma and bumi are **unsafe** — attacker can craft malicious patterns
- Must use rumi, puma-crusty, or bumi-crusty for guaranteed linear time

### Example Attack

Pattern: `(a+)+$`
Input: `"aaaaaaaaaaaaaaaaaaaaX"` (N=20)

| Variant | Time | Behavior |
|---------|------|----------|
| rumi | 11 ns | Returns false |
| puma | 72 ms | Returns false after 6.5M backtracking attempts |
| bumi | 11 ms | Returns false after 1M backtracking attempts |
| puma-crusty | 11 ns | Returns false |
| bumi-crusty | 11 ns | Returns false |

At N=25, puma and bumi hang indefinitely. The worker thread never returns.

See [ReDoS Protection](redos.md) for full technical deep dive.

## Fail-Closed Defaults

**Guarantee**: No match never accidentally becomes a match.

The matcher uses **fail-closed semantics**:
- If a predicate evaluates to false, the match fails
- If input data is missing, the predicate evaluates to false
- If no rule matches, the matcher returns `None` (no action)

There is no way for missing data or failed predicates to "leak through" and trigger an unintended action.

### The None → false Invariant

When a `DataInput` returns `None` (missing data), the predicate evaluates to `false`:

```rust
// rumi
impl SinglePredicate {
    fn evaluate(&self, ctx: &Ctx) -> bool {
        let value = self.input.get(ctx);
        if let MatchingData::None = value {
            return false;  // missing data = no match
        }
        self.matcher.matches(&value)
    }
}
```

This prevents bugs where a missing header or query param causes a "default match" that bypasses security rules.

**Example**: A rule matches requests with `x-admin: true`. If the header is missing:
- Predicate evaluates to `false`
- Rule does not match
- Request is not treated as admin

The attacker cannot trigger admin behavior by omitting the header.

## Input Validation

**Guarantee**: Configs are validated at construction, not at runtime.

All variants validate matcher configs when they're built:

```rust
// rumi
let matcher = Matcher::try_from(proto_config)?;  // fails fast on invalid config
```

```python
# puma
matcher = Matcher.from_proto(proto_config)  # raises ValueError on invalid config
validate_depth(matcher)  # enforces MAX_DEPTH
```

Invalid configs are rejected before the matcher is ever used. Runtime evaluation assumes the config is valid and never checks invariants that should have been validated at construction.

This follows the **parse, don't validate** principle: once a `Matcher` object exists, it's known to be valid.

## Type Safety

**Guarantee**: Matchers are type-safe at the domain level.

A `Matcher<HttpMessage, A>` only accepts `HttpMessage` contexts. You cannot pass a `GrpcMessage` to an HTTP matcher. The type system prevents misuse.

```rust
// rumi
let http_matcher: Matcher<HttpMessage, Action> = ...;
let http_req = HttpMessage::from(...);
http_matcher.evaluate(&http_req);  // OK

let grpc_req = GrpcMessage::from(...);
http_matcher.evaluate(&grpc_req);  // compile error: type mismatch
```

In Python and TypeScript, this is enforced at runtime via protocols:

```python
# puma
def evaluate(self, ctx: HttpRequest) -> Action | None: ...
```

The type hint documents the expected context. Passing the wrong type raises `AttributeError` when the matcher tries to access missing fields.

## Thread Safety

**Guarantee**: All matcher types are thread-safe.

Matchers are immutable after construction. All core types implement `Send + Sync` (Rust) or equivalent thread-safety guarantees in Python and TypeScript.

Multiple threads can evaluate the same matcher concurrently without locking:

```rust
// rumi
static MATCHER: Lazy<Matcher<HttpMessage, Action>> = Lazy::new(|| ...);

// Called from multiple threads
fn handle_request(req: HttpRequest) {
    let result = MATCHER.evaluate(&req);  // no lock needed
}
```

This enables zero-overhead concurrent evaluation in multi-threaded servers.

## Resource Bounds

**Guarantee**: Evaluation time is bounded by config size, not input size (except regex).

For non-regex operations, evaluation cost is O(rules) regardless of input size:
- Exact string match: O(1) comparison
- Prefix match: O(1) radix tree lookup
- First-match-wins over N rules: O(N) linear scan in worst case

The only unbounded operation is regex matching, where cost is O(n) in the input length for rumi/crusty variants, and O(2^n) for puma/bumi on adversarial patterns.

**Maximum evaluation time** (trusted patterns, 200 rules):
- rumi: 3.5 microseconds
- bumi: 2.1 microseconds
- puma: 20 microseconds

For untrusted patterns, use a variant with linear-time regex.

## No Unsafe Code (Rust)

**Guarantee**: rumi uses zero unsafe code in the core engine.

All `Send + Sync` implementations are compiler-derived. No manual `unsafe impl`. The type system enforces thread safety without escape hatches.

The only `unsafe` in the entire codebase is in FFI boundary layers (puma-crusty and bumi-crusty), where it's required to cross language boundaries. The core engine is 100% safe Rust.

## Attack Scenarios

### Scenario 1: Stack Overflow via Deep Nesting

**Attack**: Craft a config with 10,000 nested matchers to cause stack overflow.

**Mitigation**: `MAX_DEPTH = 32` enforced at config validation. Invalid config rejected before use.

**Result**: Attack prevented. Config never loads.

### Scenario 2: ReDoS via Malicious Pattern

**Attack**: Inject pattern `(a+)+$` into user-facing config, send input `"a" * 30 + "X"` to hang the service.

**Mitigation**: Use rumi, puma-crusty, or bumi-crusty for untrusted patterns. Linear-time regex prevents exponential backtracking.

**Result**: Attack mitigated if using safe variant. puma/bumi vulnerable if attacker controls patterns.

### Scenario 3: Bypass via Missing Header

**Attack**: Omit the `x-admin` header to bypass a rule that checks `x-admin: true`.

**Mitigation**: None → false invariant. Missing header causes predicate to return `false`, not `true`.

**Result**: Attack prevented. Rule does not match when header is missing.

### Scenario 4: Type Confusion

**Attack**: Pass a `GrpcMessage` to an `HttpMatcher` to trigger undefined behavior.

**Mitigation**: Type system enforces context types. Rust prevents this at compile time. Python/TypeScript fail at runtime with `AttributeError`.

**Result**: Attack prevented. Type mismatch detected before evaluation.

## Security Checklist

Before deploying a matcher in production:

- [ ] Depth limit validated — confirm `MAX_DEPTH` enforcement
- [ ] ReDoS risk assessed — untrusted patterns require rumi/crusty variants
- [ ] Fail-closed defaults confirmed — missing data evaluates to false
- [ ] Config validated at load time — invalid configs rejected
- [ ] Type safety verified — context types match matcher expectations
- [ ] Thread safety confirmed — concurrent evaluation is safe

For untrusted input scenarios, prefer rumi for maximum safety.

## Related Pages

- [ReDoS Protection](redos.md) — Deep dive into regex denial-of-service
- [Benchmark Results](benchmarks.md) — Performance data including ReDoS scenarios
- [Performance Guide](guide.md) — Which variant to use for your threat model
