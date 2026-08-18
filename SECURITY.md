# Security

## Reporting

Report vulnerabilities through [GitHub Security
Advisories](https://github.com/mox-nexus/x.uma/security/advisories/new). Please
do not open a public issue for anything exploitable.

Nothing is published to crates.io, PyPI or npm yet, so there is no released
version to patch. That changes at 0.1.0 and this section will change with it.

## What x.uma is, for threat-modelling purposes

x.uma is an embedded matcher engine. It **never opens a socket** — no xDS
client, no subscription, no transport of its own. Config reaches it either from
a file a human wrote or from a host's own xDS client, and the host owns the
transport entirely.

That means the attack surface is narrow and specific:

- **Config is untrusted input.** It may arrive from a control plane, so it must
  be treated as adversarial even though it looks like local configuration.
- **Matched data is untrusted input.** Headers, paths, tool arguments.
- **The engine can gate agent actions.** `rumi run claude` decides whether a
  Claude Code tool call proceeds, so a matcher that fails open is an agent
  safety problem, not only a correctness one.

## The resource-limit model

This is the part consumers must reason about, because it determines what a
hostile config can cost you.

**Limits belong to the type that holds the resource, not to the config loader.**
Every constructor that compiles a pattern enforces its own limit, so the
registry, both domain compilers, the FFI bindings and direct construction all
inherit the same guarantees. A limit enforced only in a loader is advisory to
every other caller — that was a real defect, and it let a domain compiler accept
an 8 MB pattern against an 8 KB limit.

| Limit | Value | Bounds | Enforced by |
|---|---|---|---|
| `MAX_DEPTH` | 32 | nested matcher depth | `Matcher::validate()` |
| `MAX_FIELD_MATCHERS` | 256 | rules in one matcher | `Matcher::validate()` |
| `MAX_PREDICATES_PER_COMPOUND` | 256 | children of one `and`/`or` | `Predicate::validate()` |
| `MAX_PATTERN_LENGTH` | 8192 | a literal match pattern | `StringMatchSpec` constructor |
| `MAX_REGEX_PATTERN_LENGTH` | 4096 | a regex pattern's source | `StringMatchSpec` constructor |
| repetition product (TypeScript) | 1000 | nested `{n}` counts, matching RE2's own `kMaxRepeat` | the regex budget |
| config nesting | 128 | JSON/YAML object depth before a matcher exists | both parsers, and the protojson walk |

The two width limits used to be enforced in the config loader alone, which meant
a matcher produced by a domain compiler carried neither — including the compiler
that gates agent tool calls. Both compilers now call `validate()` before
returning, so every construction route inherits the same guarantees.

**Empty identifiers are rejected at construction.** A header name, query
parameter name, tool argument name or map key says *where* to read a value.
Empty, it reads nothing, so the predicate is false and a rule keyed on it stops
firing — a deny rule that never denies. proto3 cannot express "required", so the
schema cannot carry this; the constructor does.

**Regex is linear-time in every implementation** — Rust's `regex` crate,
`google-re2` in Python, `re2js` in TypeScript. None can backtrack, so evaluation
cost is bounded by input length.

**Compile cost is bounded separately**, because pattern *length* does not bound
it. Compiled program size grows as the product of nested repetition counts:
twenty characters of `((a{100}){100}){100}` cost 282 ms and 286 MB before that
was bounded. Rust and Python get this from their engines; TypeScript needs an
explicit budget because `re2js` implements neither of RE2's compile-time guards.

### Known gaps

Stated plainly, because a limit table that omits what it does not cover reads as
a stronger guarantee than it is.

- **There is no aggregate budget.** Every limit is per-item. 256 field matchers
  each carrying a legal 13-byte regex is ~3.3 KB of config and about 2.9 s of
  CPU, with every declared limit respected.
- **Evaluation is recursive**, with `MAX_DEPTH` holding the line rather than an
  explicit stack. What actually prevents config-borne stack exhaustion today is
  the parsers' own 128-level recursion limit, not `MAX_DEPTH`. Measured
  2026-08-18: `serde_json` and `serde_yaml` both accept 128 levels and reject
  129, so neither front end can hand a deeper document to anything downstream.
- **An empty rule list compiles to a catch-all.** Its polarity depends entirely
  on how the caller assigns actions.

## Controls that are load-bearing

Named so they are not removed by someone who does not know what they hold up.

1. `serde`'s 128-level recursion limit — the real defence against config-borne
   stack exhaustion. Changing deserializer, or calling
   `disable_recursion_limit()`, removes it silently.
2. `serde_yaml` rejects YAML alias bombs. Whatever replaces it must be re-tested
   for this; the crate is archived upstream and a migration is planned.
3. Rust `regex`'s 10 MB size limit — the only cap on a single compiled regex.
4. `google-re2` rejecting nested counted repetition, which is why Python is
   immune to the compile bomb. Do not swap it for a pure-Python engine.
5. `AnyResolver` is a closed-world allowlist with decoders monomorphised at
   registration. There is no reflective type lookup, so the classic
   polymorphic-deserialization attack is absent by construction.
6. `#[serde(deny_unknown_fields)]` on `HookMatch` and `ArgumentMatch`. Without
   it, a typo'd field in a deny rule silently produces a catch-all.
7. Zero `unsafe` in the entire tree.
8. No `panic = "abort"` in any profile, so PyO3's `catch_unwind` works and Rust
   panics surface as `PanicException` rather than unwinding into CPython.

## Scope

The docs site has no URL-seeded state, so its playground is self-inflicted-only.
**If a share-link or rule-export feature ships, this needs re-auditing** — the
graph renderer parses config without going through the loader and inherits no
limits.

`cargo audit` runs on every push. Note that a clean audit is not the same as
safe: it flags neither an archived-but-unadvised dependency nor a pre-1.0 crate
with 1,121 downloads sitting on a default feature path.
