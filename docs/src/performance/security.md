# Security Model

x.uma's security model prevents four classes of attack against matcher engines. Every guarantee is enforced at construction time, not evaluation time.

## Threat Model

Matcher configs can come from untrusted sources — user-provided routing rules, dynamically loaded policy files, configs from external systems. The engine must be safe even when the config is adversarial.

## ReDoS Protection

**Threat:** Regular expression Denial of Service. A crafted regex pattern causes exponential backtracking, consuming CPU indefinitely.

**Mitigation:** All implementations use RE2-class linear-time regex engines:

| Implementation | Regex Engine | Guarantee |
|----------------|-------------|-----------|
| rumi (Rust) | `regex` crate (DFA) | Linear time, proven |
| xuma (Python) | `google-re2` (C++ RE2 binding) | Linear time, Google RE2 |
| xuma (TypeScript) | `re2js` (pure JS RE2 port) | Linear time, RE2 semantics |
| puma-crusty | Rust `regex` via PyO3 | Same as rumi |
| bumi-crusty | Rust `regex` via WASM | Same as rumi |

No implementation uses a backtracking regex engine. Patterns that would cause catastrophic backtracking in PCRE/Python `re`/JavaScript `RegExp` are either rejected or matched in linear time.

**Pattern length limit:** Regex patterns are capped at 4,096 characters (`MAX_REGEX_PATTERN_LENGTH`). Non-regex patterns are capped at 8,192 characters (`MAX_PATTERN_LENGTH`).

## Depth Limit

**Threat:** Stack overflow from deeply nested matchers. A config with 1,000 levels of nested matchers could exhaust the call stack during evaluation.

**Mitigation:** Maximum nesting depth of 32 levels (`MAX_DEPTH`), validated at construction time. If a config exceeds this limit, `MatcherError::DepthExceeded` is returned and no matcher is constructed.

32 levels is generous — real-world matchers rarely exceed 5 levels. The limit catches misconfigured or adversarial configs.

## Width Limits

**Threat:** Resource exhaustion from extremely wide matchers. A config with millions of field matchers at depth 1 bypasses depth limits but still causes excessive memory and CPU usage.

**Mitigation:** Three width limits, all validated at construction time:

| Limit | Value | Protects |
|-------|-------|----------|
| `MAX_FIELD_MATCHERS` | 256 per `Matcher` | Memory from wide matcher lists |
| `MAX_PREDICATES_PER_COMPOUND` | 256 per AND/OR | CPU from wide predicate trees |
| `MAX_PATTERN_LENGTH` | 8,192 chars | Memory from large string patterns |

## None-to-False

**Threat:** Missing data accidentally matching a rule. If a header doesn't exist, it should not match `ExactMatcher("secret")`.

**Mitigation:** When `DataInput.get()` returns `None`/`null`, the predicate evaluates to `false`. The `InputMatcher` is never called. This is enforced in all five implementations.

This is a security invariant, not a convenience feature. It ensures fail-closed behavior: missing data means no match.

## Immutability

**Threat:** Race conditions from concurrent access. Matchers shared across threads could produce inconsistent results if modified during evaluation.

**Mitigation:**

- **Rust:** All core types are `Send + Sync`. Matchers are immutable after construction and safe to share via `Arc<Matcher>`.
- **Python:** All types use `@dataclass(frozen=True)`. Fields cannot be reassigned after construction.
- **TypeScript:** All types use `readonly` fields.
- **Registry:** Immutable after `.build()`. The builder produces the registry, then the builder is consumed. No runtime registration.

## Construction-Time Validation

All validation happens when the matcher is built, not when it's evaluated. If a `Matcher` object exists, it's guaranteed to be:

- Within depth limits
- Within width limits
- Free of invalid regex patterns
- Free of unknown type URLs (config path)
- Structurally sound (OnMatch exclusivity enforced by the type system)

This follows the "parse, don't validate" principle. The construction boundary is the trust boundary.

## Error Messages

`MatcherError` variants include actionable context:

- `UnknownTypeUrl` lists all registered type URLs
- `DepthExceeded` shows actual vs maximum depth
- `PatternTooLong` shows actual vs maximum length
- `InvalidPattern` includes the regex compilation error

Self-correcting error messages help operators fix configs without guessing.

## What Is NOT Protected

- **Semantic correctness:** x.uma doesn't verify that your rules do what you intend. First-match-wins means rule order matters — a too-broad rule early in the list can shadow specific rules.
- **Action interpretation:** The engine returns actions without interpreting them. Whether `"allow"` means permit is your responsibility.
- **Context injection:** x.uma trusts the context you provide. If your `DataInput` produces unsafe values from user input, the engine cannot protect you.
- **Side effects:** Evaluation is pure (no I/O, no state mutation). But the code that acts on the result is outside x.uma's scope.

## Next

- [Benchmarks](benchmarks.md) — concrete performance numbers
- [xDS Semantics](../explain/xds-semantics.md) — the protocol behind the guarantees
- [Architecture](../explain/architecture.md) — how safety is built into the design
