<!--
Provenance. Produced 2026-08-16 by a `ci-scaffolds:mudge` subagent against commit
`e90429e`. It was never written to disk; `PLAN.md` folded a summary into Phase S
and the source was lost when the session ended. Recovered 2026-08-17 from the
session's subagent transcript and committed so the falsifying tests below have a
home.

Read it as evidence, not as truth. At least one finding is wrong: F-04 states
there is no `SessionIdInput`. There is — `rumi/core/src/claude/inputs.rs:48`,
registered at `claude/mod.rs:72`, present at the reviewed commit. The underlying
defect is real (`claude/config.rs` has no `session_id` field); the prescribed fix
is smaller than written.

Findings are numbered `F-01..F-06`, `S-1..S-5`, `L-1..L-4` here. `PLAN.md` uses
its own `F1..F18` and `S1..S3`. They do not correspond. See PLAN.md §4's note on
cross-referencing.
-->

# Security review: x.uma pre-publication

**Scope reviewed:** `rumi` Rust workspace (core, ext/http, proto, cli, crusts/python, crusts/wasm), `puma` (Python), `bumi` (TypeScript), `docs/experience` (SvelteKit site), CI workflows, dependency graphs for all five published artifacts. Commit `e90429e`.

**Threat model as given, accepted:** config arrives from a control plane over the network (untrusted); matched data is untrusted; the engine gates Claude Code tool calls, so a matcher that fails open is an agent-safety problem.

**Out of scope:** the actual Claude Code hook runner (not in this repo), `spec/tests` fixture correctness, `rumi/proto` runtime behaviour (design review only — the crate compiles in-workspace but has no consumer wired up).

Findings are ranked by exploitability under that threat model, not by generic severity.

---

## F-01: A 20-character regex in a route config costs 276 ms and 325 MB in bumi

**File:** `/Users/yza.vyas/mox/packages/x.uma/bumi/src/string-matchers.ts:91`

```typescript
this.compiled = RE2JS.compile(pattern);
```

**Claim:** `re2js` implements neither of C++ RE2's two compile-time guards — the `max_mem` program budget (8 MB default) nor the nested-repetition product limit. Compiled program size grows as the product of nested `{n}` counts, with no ceiling. `MAX_REGEX_PATTERN_LENGTH = 4096` (`/Users/yza.vyas/mox/packages/x.uma/bumi/src/registry.ts:57`) bounds pattern *length*; the cost is driven by compiled *program size*. Wrong axis, zero protection.

**Falsifying test** (run from `/Users/yza.vyas/mox/packages/x.uma/bumi`):

```javascript
import { RE2JS } from "re2js";
for (const p of ["a{100}", "(a{100}){100}", "((a{100}){100}){100}"]) {
  const t = Date.now(); RE2JS.compile(p);
  console.log(`len=${p.length} ${Date.now()-t}ms rss=${Math.round(process.memoryUsage().rss/1048576)}MB`);
}
```

**Expected pre-fix** — measured on this machine:

```
len=6  "a{100}"               COMPILED in 2ms   rss=52MB
len=13 "(a{100}){100}"        COMPILED in 8ms   rss=60MB
len=20 "((a{100}){100}){100}" COMPILED in 276ms rss=325MB
```

Cost is linear in the repetition product, ~0.3 µs and ~0.22 KB per NFA instruction, with no ceiling. One more nesting level (`(((a{100}){100}){100}){10}`, 26 chars) measures 3.25 s / 2.19 GB. Extrapolating the measured law, 27 characters reaches ~32 s and ~22 GB — an OOM kill.

**Expected post-fix:** rejected at construction with a `MatcherError`, in under a millisecond.

**Blast radius:** a compromised or hostile control plane pushes one route config and OOM-kills every bumi process consuming it. In a multi-tenant gateway accepting tenant-supplied `RegularExpression` matches, any tenant kills the shared process. Match time is fine — RE2's lazy DFA holds at ~28× baseline and linear. This is compile and memory, not evaluation.

**puma is immune and proves the fix is correct.** `google-re2` rejects all three patterns in under a millisecond with `invalid repetition size: {100}`. The Rust `regex` crate rejects the equivalent with `Compiled regex exceeds size limit of 10485760 bytes`. bumi is the only one of the three without a program-size guard.

**Fix:** `re2js` exposes no `max_mem` option, so bound it yourself in `RegexMatcher`'s constructor — reject a `{n}` whose operand already contains a `{m}`, mirroring RE2's own rule. Put the check in the constructor, not the config loader (see F-02).

**Why this fix:** class elimination. A rejected pattern class cannot be written, at any call site.

**Residual:** a program-size budget is more precise than a syntactic nesting rule and would also catch large alternations. The syntactic rule is the cheap correct-today fix; note the gap.

---

## F-02: Every resource limit is enforced in the config loader, not in the constructor it protects

This is the architectural root. F-01 and F-03 are instances. It reproduces independently in all three implementations, which is the tell that it is structural rather than an oversight.

**Rust** — `/Users/yza.vyas/mox/packages/x.uma/rumi/core/src/registry.rs:611`

```rust
fn check_pattern_length(spec: &crate::StringMatchSpec) -> Result<(), MatcherError> {
```

`check_pattern_length` is a **private method on `Registry`**, called from exactly one place (`registry.rs:581`). The public constructor it is supposed to protect — `/Users/yza.vyas/mox/packages/x.uma/rumi/core/src/string_match.rs:56` `StringMatchSpec::to_input_matcher()` — checks nothing.

**Falsifying test** (ran it, passes):

```rust
let huge = "A".repeat(MAX_PATTERN_LENGTH * 1024);  // 8 MB
let m = HookMatch { tool_name: Some(StringMatch::Contains(huge)), ..Default::default() };
assert!(m.compile("x").is_ok());
```

```
F-02 CONFIRMED: HookMatch::compile accepted 8388608 byte pattern (limit is 8192)
F-02b CONFIRMED: StringMatchSpec::to_input_matcher accepted 65536 byte regex (limit 4096)
```

**Confirmed bypass paths, by language:**

| Language | Limit enforced at | Bypassed by |
|---|---|---|
| Rust | `Registry::load_*` (`registry.rs:399,444,529,541,581`) | `StringMatchSpec::to_input_matcher` (`string_match.rs:56`), `compile_hook_matches` (`claude/compiler.rs:109`), `compile_route_matches` (`ext/http/src/compiler.rs:127`) — `grep MAX_ rumi/ext/http/src/*.rs` returns **nothing** |
| Python | `_compile_built_in` (`/Users/yza.vyas/mox/packages/x.uma/puma/src/xuma/_registry.py:375`) | `RegexMatcher(...)` direct; HTTP gateway (`_gateway.py:113,126,139`) — measured: accepted a 40,960-byte regex against a 4,096 limit |
| TypeScript | `compileBuiltIn` (`/Users/yza.vyas/mox/packages/x.uma/bumi/src/registry.ts:343`) | `RegexMatcher` direct; gateway (`gateway.ts:94,106,118`); and `configToGraph` (`docs/experience/src/lib/playground/graph/config-to-graph.ts:42`) calls `parseMatcherConfig` without `loadMatcher`, so the graph renderer inherits no limits at all |

**Blast radius:** every limit the project documents as a security control — `MAX_PATTERN_LENGTH`, `MAX_REGEX_PATTERN_LENGTH`, `MAX_FIELD_MATCHERS`, `MAX_PREDICATES_PER_COMPOUND` — is advisory on any path that isn't the JSON/YAML registry loader. The domain compilers are the documented "door handle" from CLAUDE.md, and they are the door with no lock.

**Fix:** move each check into the constructor of the thing being limited.

```rust
// rumi/core/src/string_match.rs — to_input_matcher()
Self::Regex(v) => {
    if v.len() > crate::MAX_REGEX_PATTERN_LENGTH {
        return Err(MatcherError::PatternTooLong { len: v.len(), max: crate::MAX_REGEX_PATTERN_LENGTH });
    }
    // ...
}
```

Then delete `Registry::check_pattern_length` — the loader inherits the guarantee. Same move for `RegexMatcher.__init__` in puma and `RegexMatcher`'s constructor in bumi. No new dependencies; this is within the no-`thiserror` policy since `MatcherError::PatternTooLong` already exists (`lib.rs:267`).

**Why this fix:** complete mediation. One chokepoint owns the decision, and no caller can route around it.

**Pattern to watch:** the same shape produced F-01 and will produce the next one. The rule is *the type that holds the resource owns the limit on that resource.*

---

## F-03: 3,328 bytes of config buys 2.9 seconds of CPU, with every declared limit respected

**File:** `/Users/yza.vyas/mox/packages/x.uma/rumi/core/src/registry.rs:399` (`MAX_FIELD_MATCHERS` check) combined with `registry.rs:614-620` (per-pattern length check)

**Claim:** the limits are per-item, and nothing bounds the aggregate. 256 field matchers (exactly `MAX_FIELD_MATCHERS`) each carrying a 13-byte regex (far under the 4,096 limit) compiles to seconds of CPU.

**Falsifying test** (ran it):

```rust
let pattern = "(a{500}){500}";  // 13 bytes
for _ in 0..256 { held.push(StringMatchSpec::Regex(pattern.into()).to_input_matcher().unwrap()); }
```

**Expected pre-fix:**

```
F-05: 256 regexes x 13 bytes = 3328 bytes of config
F-05: compiled in 2.862036917s      (release build; 30.5s in debug)
```

~860× amplification from config bytes to milliseconds of CPU, and the memory ceiling is 256 × the regex crate's 10 MB `size_limit` = 2.5 GB. Every limit in `lib.rs` is satisfied.

**Blast radius:** a control plane pushing configs on an update loop stalls the data plane. Config reload is typically synchronous on a hot path.

**Fix:** add an aggregate budget to the loader — track cumulative compiled regex size across a `load_matcher` call and fail past a threshold. `regex::RegexBuilder::size_limit()` lets you set a much smaller per-pattern budget than the 10 MB default; something like 64 KB is generous for routing patterns and cuts the ceiling by 160×.

**Residual:** an aggregate budget is a policy number that needs a default. Pick it from the benchmark suite you already have (`rumi/core/benches/redos.rs`), not from intuition.

---

## F-04: Both FFI crusts accept `session_id`, count it as a constraint, then silently discard it

**Files:** `/Users/yza.vyas/mox/packages/x.uma/rumi/crusts/python/src/convert.rs:32-101` and `/Users/yza.vyas/mox/packages/x.uma/rumi/crusts/wasm/src/convert.rs:91-111`

```rust
let is_empty = py_match.event.is_none()
    && py_match.tool_name.is_none()
    && py_match.arguments.is_empty()
    && py_match.session_id.is_none()      // <-- counted here
    && py_match.cwd.is_none()
    && py_match.git_branch.is_none();

if is_empty && !py_match.match_all { return Err(/* V-BYPASS-1 */); }
// ...
Ok(HookMatch {
    event, tool_name, arguments, cwd, git_branch,   // <-- session_id is NOT here
})
```

`HookMatch` (`/Users/yza.vyas/mox/packages/x.uma/rumi/core/src/claude/config.rs:38-49`) has no `session_id` field. There is no `SessionIdInput` in `claude/inputs.rs`, though `HookContext::session_id()` exists at `claude/context.rs:189`.

**Claim:** a rule constraining only `session_id` satisfies the V-BYPASS-1 empty-match guard, then converts to an all-`None` `HookMatch`, which is a catch-all.

**Mechanism:** `HookMatch::to_predicate()` (`claude/compiler.rs:51-82`) collects predicates from the five fields it knows about. All are `None`, so `predicates` is empty. `Predicate::from_all(vec![], catch_all())` (`predicate.rs:201-207`) returns the `catch_all()` branch — `PrefixMatcher::new("")`, which matches every string.

**Falsifying test** (ran it, passes):

```rust
let m = HookMatch { event: None, tool_name: None, arguments: None, cwd: None, git_branch: None };
let matcher = m.compile("ALLOW").unwrap();
assert_eq!(matcher.evaluate(&HookContext::pre_tool_use("Bash").with_arg("command", "rm -rf /")), Some("ALLOW"));
```

```
F-01 CONFIRMED: all-None HookMatch matches every context
```

**Expected pre-fix:** `HookMatcher.compile([HookMatch(session_id="sess-abc")], "allow", fallback="deny")` returns `"allow"` for every tool call in every session.

**Expected post-fix:** either the rule constrains by session as written, or `compile()` raises `ValueError`.

**Blast radius:** in an allowlist gate, total bypass — every tool call in every session is permitted by a rule the operator believes is scoped to one session. The V-BYPASS-1 control was built specifically to prevent accidental catch-alls, and this walks straight through it. Present in both the PyPI wheel and the npm package.

**Fix, smallest first:** reject the field until it is implemented.

```rust
if py_match.session_id.is_some() {
    return Err(PyValueError::new_err(
        "session_id matching is not implemented — remove the field",
    ));
}
```

Better: add `session_id: Option<StringMatch>` to `HookMatch` and a `SessionIdInput` alongside `CwdInput`. `HookContext` already carries the data.

**Why this fix:** fail-safe defaults. A constraint the engine cannot enforce must be an error, never a silently-dropped no-op.

**Pattern:** the FFI structs and the core struct drifted, and nothing in the type system links them. `convert_hook_match` constructs `HookMatch` with named fields, so adding a field to `HookMatch` would break the build and force the conversion to be updated — but *removing* a field from the FFI struct's meaning does not. A round-trip test (`PyHookMatch` → `HookMatch` → assert every non-`None` input field is represented) would catch this class.

---

## F-05: An empty rule list compiles to a catch-all

**Files:** `/Users/yza.vyas/mox/packages/x.uma/rumi/core/src/claude/compiler.rs:119` and `/Users/yza.vyas/mox/packages/x.uma/rumi/ext/http/src/compiler.rs:138`

```rust
let or_pred = Predicate::from_any(predicates, catch_all());
```

**Claim:** with zero rules, `from_any` returns `catch_all()`. The polarity of the failure depends entirely on how the caller assigns actions, and the library gives no signal about which one they picked.

- `compile_hook_matches(&[], "deny", Some("allow"))` → denies everything. Fail-closed, safe.
- `compile_hook_matches(&[], "allow", Some("deny"))` → **allows everything**. Fail-open.

The repo's own tests document the behaviour as intended (`claude/compiler.rs:553` `e2e_empty_rules_matches_everything`, `ext/http/src/compiler.rs:534`), so this is a deliberate design choice — but it is the choice that makes an allowlist with a config-load failure into a total bypass.

**Falsifying test:** `assert_eq!(compile_hook_matches::<&str>(&[], "allow", Some("deny")).unwrap().evaluate(&HookContext::pre_tool_use("Bash")), Some("allow"))`.

**Blast radius:** any consumer whose rule list can become empty — YAML file missing, control-plane push returning nothing, a filter that removed everything — silently converts an allowlist gate into open access.

**Fix:** the crusts already solved this correctly for the single-rule case with `match_all`. Extend the same ceremony to the list:

```rust
pub fn compile_hook_matches<A>(matches: &[HookMatch], action: A, on_no_match: Option<A>)
    -> Result<Matcher<HookContext, A>, MatcherError>
{
    if matches.is_empty() {
        return Err(MatcherError::InvalidConfig {
            source: "empty rule list compiles to a catch-all — use compile_catch_all() to confirm".into(),
        });
    }
    // ...
}
```

**Why this fix:** psychological acceptability. The safe path stays the default; the catch-all requires explicit ceremony. Note the crusts' `match_all` flag is exactly this pattern already — it is the right instinct, applied one level too shallow.

---

## F-06: `MAX_DEPTH` is enforced on the config path only; the compilers never call `validate()`

**File:** `/Users/yza.vyas/mox/packages/x.uma/rumi/core/src/matcher.rs:278-287`

```rust
pub fn validate(&self) -> Result<(), MatcherError> {
    let depth = self.depth();
    if depth > MAX_DEPTH { return Err(MatcherError::DepthExceeded { depth, max: MAX_DEPTH }); }
    Ok(())
}
```

`validate()` checks depth and nothing else — `MAX_FIELD_MATCHERS` and `MAX_PREDICATES_PER_COMPOUND` are not re-checked here, only in `Registry::load_*`. `grep -rn validate rumi/ext/http/src/` returns nothing; `compile_hook_matches` does not call it either.

**Falsifying test** (ran it, passes):

```
F-04: built matcher of depth 202 (MAX_DEPTH=32)
F-04 CONFIRMED: evaluate() ran on an over-depth tree; validate() is opt-in
```

**But the config-borne version of this attack is already blocked — and not by anything in rumi.** I tested it:

```
json depth=50 bytes=7756 -> REJECTED (recursion limit exceeded at line 1 column 4615)
yaml depth=5000 -> REJECTED (recursion limit exceeded at line 1 column 4640)
```

`serde`'s 128-level recursion guard rejects deeply-nested `MatcherConfig` long before `MAX_DEPTH` is consulted. Measured stack exhaustion in `Predicate::evaluate` needs ~100,000 levels of nesting (50,000 survives; 100,000 aborts with `fatal runtime error: stack overflow`) — unreachable through any deserializer.

**So: no finding for config-borne stack exhaustion. It is defended.** The finding is that **the team does not know what is defending it.** `MAX_DEPTH = 32` is documented as the protection; the actual load-bearing control is a `serde` default that nobody wrote down. If someone calls `serde_json::Deserializer::disable_recursion_limit()`, switches deserializers, or adds a binary/proto config path (which `rumi/proto` is heading toward — prost has no equivalent guard), the protection vanishes with no test failing.

**Fix:** two parts.
1. Call `validate()` at the end of `compile_hook_matches` and `compile_route_matches` — the crusts already do this correctly (`crusts/python/src/matcher.rs:74`, `crusts/wasm/src/matcher.rs:73`); the pure-Rust paths should match.
2. Add a regression test asserting a 200-deep config is rejected, so the day the serde guard stops applying, CI says so.

**Residual:** the iterative-evaluation rewrite stays deferred, and that is the right call given the above. Document *why* it is safe to defer — "serde caps nesting at 128, well under the ~100k needed to overflow" — so the next person does not remove the deferral for the wrong reason, or keep it for the wrong reason.

---

## Supply chain

**S-1 (blocker, not security but irreversible).** Two publishable crates depend on `publish = false` crates: `rumi-http` → `rumi-proto` (`/Users/yza.vyas/mox/packages/x.uma/rumi/ext/http/Cargo.toml:21`) and `rumi-cli` → `rumi-test` (`rumi/cli/Cargo.toml:19`). `cargo publish` resolves optional deps too. Verified by dry-run: `no matching package named 'rumi-proto' found`. `.github/workflows/release.yml:97-119` publishes all three in sequence, so the run publishes `rumi-core 0.0.2`, then hard-fails — and crates.io versions cannot be re-uploaded. Fix before the release workflow is ever triggered.

**S-2 (fix during the freeze window).** `rumi-http` `default = ["ext-proc"]` (`rumi/ext/http/Cargo.toml:12-16`) pulls **101 crates vs 7** with `--no-default-features` — tokio, tonic, axum, hyper, h2, prost, chrono, plus duplicated majors (`http` 0.2 and 1.5). The heaviest root is `envoy-grpc-ext-proc v0.1.2`: **1,121 total downloads**, published 2025-10-02, single third-party maintainer, pre-1.0. Making that a *default* dependency of a published library is the sharpest supply-chain edge in the repo. Every in-repo consumer already opts out (`cli:18`, `crusts/python:18`, `crusts/wasm:23`) — the project never uses its own default. Also, `ext-proc` conflates two concerns: `k8s-gateway-api` supplies the config types the compiler needs, `envoy-grpc-ext-proc` supplies data-plane types. Split them, set `default = []`. Feature defaults are effectively frozen after first publish.

**S-3.** `serde_yaml` is archived upstream (March 2024) and is a **non-optional runtime dependency of three published artifacts**: `rumi-cli` (`cli/Cargo.toml:21`), the PyPI wheel (`crusts/python/Cargo.toml:21`), and the npm package (`crusts/wasm/Cargo.toml:21`). No RustSec advisory covers it, which is exactly why `cargo audit` is clean and why clean-audit is not the same as safe. It parses untrusted YAML config and will never receive a fix. Note `serde_yml` carries `RUSTSEC-2025-0068`, so it is not the safe swap — `serde_yaml_ng` or `serde_norway`.

**S-4.** Every CI action is a floating tag, none SHA-pinned — including in the workflows that hold `CARGO_REGISTRY_TOKEN` and `NPM_TOKEN`. `dtolnay/rust-toolchain@stable` is a *branch*, the weakest form. PyPI correctly uses OIDC trusted publishing; npm uses a long-lived `NPM_TOKEN` (`release.yml:167-168`) and crates.io uses a static token, both of which now support trusted publishing. `release.yml:11-12` grants `contents: write` at workflow scope where only the `tag` job needs it.

**S-5 (minor).** No `LICENSE` files in any of the five package roots — manifests declare `MIT OR Apache-2.0` but the files live only at repo root, outside each package. `rumi/ext/http/README.md` does not exist, so its crates.io page ships bare. `google-re2` has **no musllinux wheels**, so Alpine consumers of `xuma` build RE2 + abseil from source; worth a README note.

---

## Low

**L-1.** `docs/experience` playground: `configToGraph` (`.../graph/config-to-graph.ts:42`) calls `parseMatcherConfig` without `loadMatcher`, so the graph path enforces neither `MAX_FIELD_MATCHERS` nor `MAX_DEPTH` (another F-02 instance). ELK runs on the main thread via elkjs's fake worker — there is no Web Worker anywhere in the site. Pasting ~5,000 field matchers locks the tab. The result badge correctly reports `TooManyFieldMatchers` while the graph pane happily builds 10,001 nodes. Self-inflicted only: there is no share-link feature, so no delivery channel. **If a share-link is ever added, re-audit** — this becomes remotely triggerable and `ResultBadge.svelte:28` becomes third-party-reachable.

**L-2.** `StringMatcher::regex_ignore_case` (`rumi/core/src/input_matcher.rs:373`) builds `format!("(?i){pattern}")`. A pattern beginning `(?-i)` neutralizes the flag. Config-author-controlled, no privilege crossing — noted so it is not mistaken for a guarantee.

**L-3.** `trace_string_match` (`claude/compiler.rs:236-240`) uses `.is_ok_and(...)`, so an invalid regex traces as "did not match" while `compile()` returns `Err`. Trace is the tool an operator reaches for to answer "why didn't my deny rule fire?" and it gives a different answer than the compiler. Surface the compile error in the trace.

**L-4.** `puma/src/xuma/_string_matchers.py:144` catches only `re2.error`; a non-`str` pattern escapes as a raw `TypeError`, outside the `MatcherError` contract.

---

## Controls that are correct — do not weaken these

Named explicitly so nobody removes them without knowing what they hold up.

1. **`serde`'s 128-level recursion limit** is what actually stops config-borne stack exhaustion, not `MAX_DEPTH`. Measured: depth-50 configs already rejected; ~100,000 needed to overflow. Any change to the deserializer, or a `disable_recursion_limit()` call, removes this silently.
2. **`serde_yaml` rejects YAML alias bombs** — a 325-byte billion-laughs payload returns `repetition limit exceeded` in 30 ms. Whatever replaces serde_yaml (S-3) must be re-tested for this.
3. **Rust `regex` crate's 10 MB `size_limit`** rejects `(((a{100}){100}){100}){100}` with `Compiled regex exceeds size limit`. This is the only thing capping F-03's per-regex ceiling.
4. **`google-re2` rejects nested counted repetition** — this is why puma is immune to F-01. Do not swap it for a pure-Python engine.
5. **`AnyResolver` is a closed-world allowlist** (`rumi/proto/src/any_resolver.rs:112-119`). Decoders are monomorphized at registration; an unregistered `type_url` yields `UnknownTypeUrl`. There is no reflective type lookup, so an attacker cannot steer polymorphic deserialization — the classic failure of this pattern is absent by construction. Design review only, as flagged; the runtime behaviour is untested because no consumer is wired up.
6. **`#[serde(deny_unknown_fields)]` on `HookMatch` and `ArgumentMatch`** (`claude/config.rs:37,53`). Without it, a typo'd field name in a deny rule silently produces an all-`None` catch-all — exactly F-04's failure, reachable from YAML.
7. **Zero `unsafe`** across the entire `rumi` tree — verified, the grep returns nothing but comments.
8. **The crusts call `matcher.validate()`** (`crusts/python/src/matcher.rs:74`, `crusts/wasm/src/matcher.rs:73`). They are currently the *only* paths that enforce depth on compiler output.
9. **No `panic = "abort"` in any profile**, so PyO3's automatic `catch_unwind` around `#[pymethods]` works — Rust panics surface as `pyo3_runtime.PanicException`, not an unwind into CPython. Setting `panic = "abort"` for wheel size would break this.
10. **No XSS in the docs site.** The sink grep (`{@html`, `innerHTML`, `eval`, `new Function`, …) returns zero hits across `docs/experience/src`. SVG labels use `textContent` (`graph/draw.ts:64`), not string concat. `rehype-raw` processes only build-time globbed repo markdown (`load-content.ts:11-15`) behind a hardcoded slug manifest — it does mean raw HTML in a docs `.md` is equivalent to committing JavaScript, which is a docs-PR review obligation, not a runtime hole. No URL-seeded state anywhere, which is the property that makes the whole playground safe.
11. **No regex metacharacter injection.** Prefix/suffix/contains/exact use native string ops in all three languages; `grep` for `"^" +`, `f"^`, `` `^${ `` across puma and bumi returns zero hits.
12. **`cargo audit` is clean** across 200 crates — accurate, and worth noting that it flags neither S-2 nor S-3, which are the two dependencies that actually warrant attention.

---

## Verdict

**DO NOT SHIP in current form.** Three things must close first.

**Before first release:**

1. **F-01** — the bumi regex compile bound. This is remotely triggerable by the exact channel the threat model names, with a 20-character payload, and it is the only one of the three implementations without a guard. Ship a regression fixture asserting `((a{100}){100}){100}` is rejected by all three.
2. **F-02** — move the length checks into the constructors. This is a small mechanical change that closes F-01's whole class and stops the domain compilers from being an unlocked door. Doing it now costs an afternoon; doing it after publish means a breaking change to a frozen config format across five implementations.
3. **F-04** — `session_id` silently dropped. Trivially fixed by rejecting the field, and it is a total agent-gate bypass in the allowlist polarity. This one ships in the wheel and the npm package.
4. **S-1** — the `publish = false` dependency edges. Not a vulnerability, but triggering `release.yml` today burns `rumi-core 0.0.2` on crates.io permanently and then fails.
5. **S-2** — `rumi-http` default features. Feature defaults are frozen after first publish; a 1,121-download pre-1.0 crate should not be a default dependency of a published library.

**Can follow in 0.0.3:** F-03 (aggregate regex budget), F-05 (empty-list ceremony), F-06 (call `validate()` in the compilers plus the regression test), S-3 (serde_yaml migration), S-4 (SHA-pin actions, trusted publishing), all of L.

**What would change the verdict to SHIP WITH FIXES:** items 1–5 closed with the falsifying tests above committed as regression fixtures, and the F-06 comment documenting *why* the iterative rewrite is safe to defer.

**On the claim in CLAUDE.md that ReDoS protection is handled** — it holds for evaluation in all three implementations, which is the harder half and was done right. It does not hold for compilation in bumi. The arch-guild constraint as written ("Use Rust `regex` crate only") is about match-time complexity; nobody wrote down the compile-time budget, and re2js is the one engine of the three that does not supply one by default. That is the gap between the documented control and the code.

If there is context I am missing — a planned refactor that already covers F-02, a reason the `session_id` field is intentionally inert, or a deployment constraint that makes the bumi path unreachable — tell me and I will re-run against it.
