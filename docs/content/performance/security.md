# Security Model

x.uma's security model prevents four classes of attack against matcher engines. Every guarantee is enforced at construction time, not evaluation time.

## Threat Model

Matcher configs can come from untrusted sources — user-provided routing rules, dynamically loaded policy files, configs from external systems. The engine must be safe even when the config is adversarial.

## ReDoS Protection

**Threat:** Regular expression Denial of Service. A crafted regex pattern causes exponential backtracking, consuming CPU indefinitely.

**Mitigation:** The engine uses a linear-time regex engine, and both bindings run the engine:

| Runtime | Regex Engine | Guarantee |
|---------|-------------|-----------|
| rumi (Rust) | `regex` crate (DFA) | Linear time, proven |
| xuma-crust (Python) | Rust `regex` via PyO3 | Same as rumi |
| xuma-crust (TypeScript) | Rust `regex` via WASM | Same as rumi |

No implementation uses a backtracking regex engine. Patterns that would cause catastrophic backtracking in PCRE/Python `re`/JavaScript `RegExp` are either rejected or matched in linear time.

**Pattern length limit:** Regex patterns are capped at 4,096 characters (`MAX_REGEX_PATTERN_LENGTH`). Non-regex patterns are capped at 8,192 characters (`MAX_PATTERN_LENGTH`).

## Depth Limit

**Threat:** Stack overflow from deeply nested matchers. A config with 1,000 levels of nested matchers could exhaust the call stack during evaluation.

**Mitigation:** Maximum nesting depth of 32 levels (`MAX_DEPTH`), validated at construction time. If a config exceeds this limit, `MatcherError::DepthExceeded` is returned and no matcher is constructed.

32 levels is generous — real-world matchers rarely exceed 5 levels. The limit catches misconfigured or adversarial configs.

## Width Limits

**Threat:** Resource exhaustion from extremely wide matchers. A config with millions of field matchers at depth 1 bypasses depth limits but still causes excessive memory and CPU usage.

**Mitigation:** A budget on what one evaluation can reach, and budgets per document that nesting cannot add around, all checked before a matcher exists:

| Limit | Value | Protects |
|-------|-------|----------|
| `MAX_PREDICATES_PER_EVALUATION` | 256 on the worst path | CPU per request: every rule of a list, one tree entry, or one chain of prefixes |
| `MAX_PATTERN_LENGTH` | 8,192 chars | Memory from large string patterns |
| `MAX_DOCUMENT_PREDICATES` | 65,536 per document | Memory, counted across nested matchers and tree entries |
| `MAX_REGEXES` | 64 per document, 1 MiB program and 1 MiB DFA cache each | Regex memory, bounded as count × size |
| `MAX_DOCUMENT_BYTES` | 1 MiB | A document refused by its size before it is parsed |

The per-document budgets are counted as the document is loaded, so a hostile config is refused at the predicate or regex that crosses the budget rather than after everything past it has been built.

## None-to-False

**Threat:** Missing data accidentally matching a rule. If a header doesn't exist, it should not match `ExactMatcher("secret")`.

**Mitigation:** `DataInput::get` returns `Option<MatchingData>`. When it returns `None`, the predicate evaluates to `false` and the `InputMatcher` is never called. The signature enforces it in the engine, and therefore in every binding.

This is a security invariant, not a convenience feature. It ensures fail-closed behavior: missing data means no match.

## Immutability

**Threat:** Race conditions from concurrent access. Matchers shared across threads could produce inconsistent results if modified during evaluation.

**Mitigation:**

- **Rust:** All core types are `Send + Sync`. Matchers are immutable after construction and safe to share via `Arc<Matcher>`.
- **Bindings:** A compiled matcher is an opaque handle with no setters. The Python class is `#[pyclass(frozen)]`; the WebAssembly class exposes only `evaluate` and `trace`.
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

- [Architecture](../explain/architecture.md) — how safety is built into the design
