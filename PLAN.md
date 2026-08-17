# Release plan — x.uma 0.1.0

**Written for an agent arriving with no context.** Read this file top to bottom
before touching anything. It is self-contained: every claim in it was verified
against the code on 2026-08-16, and every task tells you how to prove you are
done.

Target fidelity: **tarmac**. Production-hardened, observable, documented, no
known silent failures. Not "it works on my machine", not "tests pass".

---

## 0. Load these before you touch anything

Not after. Not by grepping their files. **Invoke them with the Skill tool.**

The session that produced this plan did deep buf/xDS/codegen work with
`protocol-mastery` unloaded, and burned hours rediscovering two things that skill
already states plainly: that this project commits generated code, and that
`buf.gen.yaml` v2 needs `clean: true`. Do not repeat that.

**Always, at session start, in this order:**

| # | Skill | Why |
|---|---|---|
| 1 | `ci-scaffolds:crafting` | Compound value, pit of success, complete the work, fidelity levels. The standard this plan is written to. |
| 2 | `maintainer` | The project's own skill. Architecture, conventions, where things live. |
| 3 | `rust-mastery` | x.uma-specific Rust judgment. **Read its §6 ladder** (enum → consume `self` → runtime check) before designing any invariant. |

**When the work touches these areas, load before starting, not partway through:**

| Skill | Load when |
|---|---|
| `protocol-mastery` | **Phase C and anything proto.** buf codegen, xDS, `TypedExtensionConfig`, `Any` resolution, wire format, ECDS. |
| `ci-scaffolds:problem-solving` | Stuck twice on the same thing. Do not attempt a third time first. |
| `ci-scaffolds:whitehat` | Phase S, and any change to limits, regex, or the FFI boundary. |
| `guild-arch:trust-boundaries` | Phase S design questions, before writing the fix. |
| `frontend-design` | Phase H, docs-site work only. |

**Also read, they are not skills but they are context:**
`CLAUDE.md` (project), `DECISIONS.md` (D-001 to D-025, newest first),
`scratch/phase-12/prior-art.md` (2025 design conversations recovered from memex).

`CLAUDE.md:359` tells you to read `scratch/next-session.md`. **That file does not
exist.** Fixing that stale instruction is task A1.

---

## 0.5 Principles in force

These are not decoration. Each one was violated during the session that produced
this plan, and each violation cost real work.

**From the constitution:**
- **Quality is the invariant.** Time, scope and fidelity flex. Quality never
  trades. Under pressure, scope down: fewer things right, not more things wrong.
- **Design for the next collaborator.** Here that is explicitly both a human and
  an agent.
- **Not done when it works. Done when it's right.**

**From `ci-scaffolds:crafting`:**
- **Compound value.** Every change makes the next easier, or it is a loan.
- **Pit of success.** Structure it so the wrong thing is hard, not so the docs
  warn against it. Documentation and willpower both fail.
- **Complete the work.** If artifacts of the old state remain, it is not done.
  `grep` for the old name; zero hits means done.
- **Think in invariants.** Can the wrong thing even be expressed?
- **Evidence before fix.** Read, check runtime data, hypothesize, *then* write.

**Earned in this project, the hard way:**
- **What CI does not check is not true.** Every false claim found — Phase 12
  "done", `keep_matching` "enforced", `just audit`, `check-no-std`, four how-to
  pages — was outside CI's reach. This is the single most load-bearing rule here.
- **Generated beats written. Checked beats asserted.** Where neither is possible,
  cite `file:line` so drift is greppable.
- **One source, or it will disagree with itself.** `skill.rs` held a config key
  in two places and they diverged within hours.
- **The type that holds the resource owns the limit on that resource.** Limits
  enforced in a loader are advisory to every other caller.
- **Trim by relevance, not by trigger token.** Trigger tokens test current usage;
  they do not test transferable judgment.
- **A claim you cannot demonstrate does not get written down.**

---

## 0.75 The rubric — how to score "tarmac"

Fidelity is per-item, not per-project. Score every task on four axes. **Tarmac
means 3 on all four, or a written exception in `DECISIONS.md` saying why not.**

| Axis | 0 | 1 | 2 | 3 — tarmac |
|---|---|---|---|---|
| **Correctness** | untested | happy path | edge cases | adversarial + regression fixture for each fixed bug |
| **Verification** | claim only | checked by hand once | a test exists | **CI enforces it on every push** |
| **Documentation** | absent | prose | prose + example | **example executes in CI** |
| **Reproducibility** | works on this machine | setup documented | toolchain pinned | **verified from a clean clone in CI** |

Worked examples so the scale is calibrated:

- `rumi --skill`'s type URL tables: **Verification 3** — generated from live
  registries, cannot drift.
- `rumi --skill`'s config-key hints: were **1** (hand-written, wrong within
  hours), now **3** — single constant, test builds a real config from it and
  asserts the loader accepts it, plus a second test proving the guard is not
  inert.
- The four how-to pages: currently **Documentation 1**. Prose with examples that
  do not run. That is what A4 fixes.
- `MAX_PATTERN_LENGTH`: currently **Correctness 1** — enforced on the loader path
  only, bypassed by both domain compilers. Phase S2 takes it to 3.
- `keep_matching`: currently **0 across the board** and documented as enforced.
  That combination is the worst square on this table.

### Which axes apply to which work

Not every task lives on all four axes. Demanding "adversarial testing" for a
markdown table rename produces a pile of exceptions, and a rubric that is mostly
exceptions is a ritual. Score only what applies:

| Task class | Axes that apply | Example |
|---|---|---|
| Behaviour change | all four | S2 moving limits into constructors |
| Schema / format | Correctness, Verification, Documentation | SF fixtures, type-URL renames |
| Docs and prose | Documentation, Verification | A3 fixing how-to pages |
| Build / tooling | Verification, Reproducibility | `just doctor`, CI jobs |
| Deletion | Verification only — `grep` returns zero hits | removing `check-no-std` |

### What the score is and is not

**It is a prompt for honesty, not an enforcement mechanism.** Where a score can
be computed by CI it should be, and then the CI check *is* the enforcement and
the number is redundant. Where it cannot, the number is the author grading their
own homework — and this entire plan exists because that author's self-attestations
("Phase 12 ✅", "INV enforced") were false.

So: state the scores, and treat any axis you scored 3 **without a CI check behind
it** as a claim you owe evidence for in the PR body. If you cannot name the check,
the honest score is 2.

---

## 0.9 Milestones

Phases are work. Milestones are **shippable states with gates**. Do not pass a
gate on partial evidence.

| M | Name | Phases | Exit gate |
|---|---|---|---|
| **M1** | The repo stops asserting what it cannot show | A | Every doc code sample is classified and enforced per the taxonomy below. Every roadmap ✅ corresponds to something CI runs. |
| **M2** | A stranger can start | B, H1–H3 | `cargo add rumi-core` + README example compiles unmodified, verified from a scratch crate outside the workspace. `just doctor` passes on a machine with nothing installed. |
| **M3** | Nothing loads clean and lies | S | S1, S2, S3 fixed, each with its falsifying test committed as a regression fixture. |
| **M4** | **Schema freeze** | SF | Every defect in §4 that crosses a schema boundary exists as a **failing** conformance fixture, and the schema's shape is decided and recorded. |
| **M5** | One schema | C | The M4 fixtures pass. `buf generate` is the only source of config types in all three languages. All 27 existing fixtures pass through the frozen schema everywhere. |
| **M6** | Everything is publishable | E, F, K | `cargo publish --dry-run` passes for all five crates in dependency order with no path patching. No published crate depends on a `publish = false` crate. CI builds every artifact. |
| **M7** | Released | G | Published, install instructions true, every `future`-marked doc block resolved. Phase L follows in 0.1.x. |

**Ordering.** M1 and M2 are independent of the rest and can run in parallel with
anything. M3 is independent — the security fixes are in hand-written runtime
code, not in the proto path. **M4 → M5 → M6 → M7 is a hard chain.**

**Why M4 exists, and why it is not part of C.** An earlier draft of this plan put
the correctness fixes *before* the migration "so defects are not carried
forward", and simultaneously marked them blocked on the migration's first task.
That was a deadlock: `ignore_case` lives in `convert.rs`, in a crate that has
never compiled, which C1 exists to fix.

The resolution is the project's own stated workflow, from `CLAUDE.md`: **write
the fixture first.** A defect that crosses a schema boundary is not a bug to fix
before migrating — it is an **acceptance test for the schema**. Write it now, red.
The migration is finished when it goes green. That is what M4 is: the defects
become the specification.

**Everything not frozen by publishing is 0.1.x.** Publishing freezes the config
schema, crate structure, feature defaults, type-URL names, and the security
posture of shipped artifacts. It does not freeze prose, doc-site navigation, or
`MatcherTree` wiring. Those ship later and are better for it.

### The M1 gate taxonomy

"Every command and code sample executes in CI" is not satisfiable while the
install instructions point at empty registries — and an agent who quietly
weakens that gate learns that gates are aspirational, which poisons every later
one. So classify instead. Every code block in the docs gets exactly one marker:

| Class | Meaning | Enforcement |
|---|---|---|
| `run` | executes and its output is asserted | executed in CI |
| `compile` | must type-check, not run | doctest / `no_run` |
| `cli` | a command whose *shape* is checked against `--help` | smoke test asserts the subcommand and flags exist |
| `future` | true only after a specific milestone | **must name the milestone**, and CI asserts it is still unreached |

`future` is the honest slot for `cargo add rumi-core` before M7. It is not an
escape hatch: a `future` block naming a milestone that has passed fails CI. That
is the mechanism that reminds Phase G to delete the pre-release notes.

**At each gate, write the scores.** A milestone claimed without its rubric filled
in is exactly the failure this plan exists to correct.

---

## 1. Ground rules

**You may commit. You may open pull requests. You may NOT merge.**
Not to `main`, not anywhere. A human merges. If a PR is green and you believe it
is ready, say so and stop.

**Branch per phase.** `phase-a/truth-repair`, `phase-b/dx-defaults`, and so on.
One PR per phase. Do not batch phases into one PR; they are separately
reviewable on purpose.

One deliberate exception: **Phase SF and Phase C may share a branch** if you
prefer, since SF's fixtures are meaningless until C makes them pass. If you do,
commit the red fixtures first and keep that commit separate, so the diff shows
the specification preceding the implementation.

**`just ci` must exit 0 before every commit.** It runs fmt, clippy with `-D
warnings`, all Rust tests, conformance fixtures, puma checks, bumi checks, docs
type-check and build, and `cargo audit`. Local and CI run the identical
sequence.

**Deletion:** `rm` is aliased to `rip` (a recoverable graveyard) at the user
level. Use `rip <path>` for untracked files and `git rm` for tracked ones. Never
`rm`, `\rm`, `/bin/rm`, or `find -delete`.

**Rust convention, in this order:**
```bash
cargo clippy --fix --allow-dirty --manifest-path rumi/Cargo.toml -- -W clippy::pedantic
cargo fmt --manifest-path rumi/Cargo.toml --all
```
`clippy --fix` rewrites code, so `fmt` runs after it. Reversing this leaves the
tree unformatted.

**No new dependencies** without saying why in the PR. Errors are hand-written
enums with manual `Display`/`Error` impls; `thiserror`, `anyhow` and friends are
a listed anti-pattern in `CLAUDE.md`.

**Verify, do not assume.** This plan exists because a great deal of this repo's
documentation asserted things that were not true. Do not add to that. If you
cannot demonstrate a claim, do not write it down.

---

## 2. What this project is

x.uma is a matcher engine implementing the xDS Unified Matcher API. You give it
a config of ordered rules and a context; it returns the action of the first rule
that matches, or an `on_no_match` fallback. The same config is meant to produce
the same answer in every implementation.

| Directory | What it is | Ships as |
|---|---|---|
| `rumi/core` | The engine. Reference implementation. | `rumi-core` on crates.io, lib name `rumi` |
| `rumi/ext/http` | HTTP domain | `rumi-http` |
| `rumi/ext/test` | Generic key-value domain | `rumi-test`, currently `publish = false` |
| `rumi/proto` | xDS proto types + conversion | `rumi-proto`, `publish = false`, **has never compiled** |
| `rumi/cli` | The `rumi` binary | `rumi-cli` |
| `rumi/crusts/python` | PyO3 bindings | `xuma-crust` on PyPI |
| `rumi/crusts/wasm` | wasm-bindgen bindings | `xuma-crust` on npm |
| `puma/` | Pure Python implementation | **package name is `xuma`**, not `puma` |
| `bumi/` | Pure TypeScript implementation | **package name is `xuma`**, not `bumi` |
| `docs/content` | Plain Markdown, framework-free | — |
| `docs/experience` | SvelteKit docs site, carries `/playground` | — |
| `spec/tests` | 27 conformance fixtures shared by all implementations | — |

**Nothing is published to any registry.** crates.io, PyPI and npm are all empty.
Zero users. Breaking changes are free right now and expensive the day after
release. That fact drives the entire ordering below.

Decisions of record are in `DECISIONS.md` (D-001 to D-025). Read it. Prior art
recovered from 2025 design conversations is in `scratch/phase-12/prior-art.md`.

---

## 3. The strategic decision, already made

**Migrate to proto-first, then publish.**

The proto becomes the config schema. YAML is ingested by converting to JSON and
running protojson, exactly as Envoy does via
`google::protobuf::util::JsonStringToMessage`. The hand-written `MatcherConfig`
types are retired, not aliased.

Why this order: the same domain model currently exists four times — the xDS
proto, plus hand-written `MatcherConfig` in Rust, Python and TypeScript. The
codegen that would collapse them is already configured in `buf.gen.yaml` for all
three languages and produces output nobody imports. Publishing the current
format means publishing a schema we intend to replace, and every break after
release costs real users.

Do **not** keep `MatcherConfig` as a compatibility shim. Zero users means zero
shims.

**One open design question you must answer in Phase C, not skip:** protojson is
substantially more verbose than today's YAML. A rule that is 5 lines today
becomes roughly 9, with camelCase, `matcherList`/`singlePredicate` wrappers,
fully-qualified `@type` URLs, and actions promoted from bare strings to
`TypedExtensionConfig`. Decide explicitly whether protojson is the **authoring**
surface or only the **wire** surface. If wire-only, keep the terse YAML as a
documented authoring dialect that lowers to proto — the lowering already exists
in the other direction in `rumi/proto/src/convert.rs`. If authoring, ship a
`rumi lower` command in the same release or hand-authoring dies. Record the
answer in `DECISIONS.md`.

---

## 4. Verified state of the world

**Read this as a lead list, not as truth.**

Every row was verified by hand on 2026-08-16 and the line numbers were right
then. They will decay: Phase A rewrites docs, Phase C rewrites `convert.rs`
outright. An agent reaching Phase C and reading "`convert.rs:226-236`" after the
migration will find something else there.

By this plan's own most load-bearing rule — *what CI does not check is not true* —
**this table is not true.** It is prose, hand-verified once, outside CI: the
fallback evidence tier used as primary evidence, which is the exact habit the
plan exists to correct. Naming that is not an excuse for it, so:

> **Each finding here must become a failing test or fixture in the phase that
> fixes it. The test supersedes the paragraph.** Once the test exists, strike
> the row.

Re-verify before acting. `grep` for the symbol, never the line number. If a row
no longer reproduces, that is itself a finding — record it and move on rather
than hunting for the original.

**Green:** 274 Rust tests, 294 Python, 258 TypeScript, 27 conformance fixtures,
`cargo audit` clean, CI green on PR #22, docs site builds and deploys.

**Broken or untrue:**

| # | Finding | Evidence |
|---|---|---|
| F1 | `rumi-proto` has never compiled | Zero tracked files in `rumi/proto/src/gen` in every commit in history |
| F2 | `keep_matching` documented as an enforced invariant, not implemented | `CLAUDE.md:213`; zero occurrences in any runtime source |
| F3 | `MatcherTree`/`RadixTree` unreachable from config | No tree variant in `MatcherConfig`; `convert.rs:88` returns "MatcherTree is not yet supported" |
| F4 | Claude domain is Rust-only | Absent from `puma/src` and `bumi/src`; README presents it as a peer of HTTP |
| F5 | CI never builds either crust | Zero `crust` references in `.github/workflows/ci.yml`; crusts are outside `default-members` |
| F6 | `ignore_case` silently dropped | `rumi/proto/src/convert.rs:226-236` reads it off the proto and discards it. Loads clean, matches case-sensitively. Silent wrong answer. |
| F7 | README example does not compile from README instructions | `rumi-core` has `default = []`; `Registry`, `RegistryBuilder`, `MatcherConfig` are behind the `registry` feature, mentioned zero times in `docs/content/` |
| F8 | Four how-to pages teach APIs that do not exist | `rumi eval`, `rumi validate`, `--config`, `register_input` — none exist. Real: `run`, `check`, `info`, `.input::<T>()` |
| F9 | `rumi-cli` cannot be published | Hard dependency on `rumi-test`, which is `publish = false` |
| F10 | `rumi-http` cannot be published | `proto` feature depends on `rumi-proto`, `publish = false` |
| F11 | `xuma.core.v1.StringMatcher` names a message that does not exist | `registry.rs:263`; `proto/xuma/core/v1/` contains only `action.proto` |
| F12 | `register_core_matchers` is a no-op in Python, absent in TypeScript | `puma/src/xuma/_registry.py:181-190` returns the builder unchanged |
| F13 | `xuma.test.v1.StringInput.value` is used as a lookup key | `proto/.../test/v1/inputs.proto:11-14` vs `rumi/ext/test/src/lib.rs:119-121`. Under protojson, every example in the repo breaks. |
| F14 | Query-string handling diverges between Python and Rust | puma strips query from path; Rust stores path verbatim. Same config, same input, different answer. |
| F15 | `CLAUDE.md` calls the packages `puma`/`bumi` | They are `xuma`. `CLAUDE.md:10-11, 95, 98` |
| F16 | HTTP compiler swallows invalid regex | `rumi/ext/http/src/compiler.rs:77-81` falls back to exact-matching the pattern literal. Route silently disappears. Sibling Claude compiler returns `Result`. |
| F17 | `data_type()` defaults to `"string"` | `data_input.rs:60-62`. A custom input returning `Int` that forgets to override passes the compatibility check, loads clean, never matches. |
| F18 | Docs snippets are invisible to CI | Every Rust block is ```` ```rust,ignore ````; shell blocks are unchecked. This is the root cause of F8. |

**Security review was still running when this plan was written.** Its findings
are folded in below. Check for a completed report before starting and
fold it in.

---

## 5. Phases

Dependency-ordered. Do not start a phase until the one before it is on a green
PR, except where noted as independent.

### Phase A — Truth repair

**Independent. Start here. Cheapest value in the plan.**

Documentation that lies is worse than missing documentation, because it is
followed. None of this is blocked by anything else.

- **A1.** Fix `CLAUDE.md`: package names are `xuma` not `puma`/`bumi` (lines
  10-11, 95, 98); `keep_matching` is deferred, not enforced (line 213 — the
  correct statement already exists in
  `.claude/skills/protocol-mastery/SKILL.md:161`); Phase 12 is **not** done;
  there is no `rumi-claude` crate (the roadmap row contradicts the crate
  structure section in the same file).
- **A2.** Audit every ✅ in the CLAUDE.md roadmap against code. The general rule
  discovered: **any phase whose subject is outside CI is unverified by
  construction.** That is Phases 7, 8, 9, 12, 14, 15. Demote what you cannot
  demonstrate.
- **A3.** Fix the four how-to pages and the getting-started pages. Known wrong:
  `rumi eval` → `run`; `rumi validate` → `check`; `--config` flag does not
  exist; `register_input(url, closure)` → `.input::<T>(url)` with an
  `IntoDataInput` impl; `SafeRegex` → `Regex` (SafeRegex is the proto spelling);
  `register_http` → `register` in Python; Python example passes a raw dict where
  a `MatcherConfig` is required (`parse_matcher_config` first);
  `BoolMatcher` config field is `expected` not `value`; `StringMatcher` config
  is `{value, match_type, ignore_case}` not `{exact}`; `compile_route_matches`
  third argument is `Option<A>`, and it does not return a `Result` so the
  README's `.unwrap()` is wrong.
- **A4.** Make regression structural — this is the part that matters. Drop
  `,ignore` from Rust snippets so doctests type-check them (`no_run` still
  compiles). Add a smoke test that executes every command string appearing in
  `docs/content/**` and `README.md`. `spec/tests/` already proves this project
  believes executable specs beat prose; the docs are the one surface where that
  belief was not applied.
- **A4b.** Fix the stale instruction itself: `CLAUDE.md:359` points at
  `scratch/next-session.md`, which does not exist. Either write it or remove the
  instruction. Also `protocol-mastery/SKILL.md:29` says `buma/proto/src/gen/`;
  the directory is `bumi`. The maps that would correct a newcomer are themselves
  drifting.

- **A5.** Rewrite `spec/tests/README.md`. The fixtures use three incompatible
  schemas — `matcher:`, `http_route_match:`, and `config:` — and the README
  documents the first, which no user can write. Only `config:` is the shipping
  format. Lead with it; rename the other two keys so they cannot be mistaken for
  it (`native_matcher:`, `compiler_route_match:`). The casing differs by one
  character between dialects (`{exact:}` vs `{Exact:}`), which is a trap.

**Done when:** every command and code sample in `README.md` and `docs/content/`
either executes in CI or does not exist in the docs. `just ci` green.

---

### Phase B — DX defaults

**Independent of C. Do it before publish; it is free now and breaking later.**

- **B1.** `rumi/core/Cargo.toml`: `default = ["registry"]`. Config loading is
  the entire pitch and it is currently opt-in behind an undocumented flag (F7).
  Cost is `serde` + `serde_json` in core, which is the price of the advertised
  feature.
- **B2.** Add `Registry`, `RegistryBuilder`, `MatcherConfig`, `TypedConfig`,
  `IntoDataInput`, `IntoInputMatcher`, `UnitConfig` to `rumi::prelude` under
  `#[cfg(feature = "registry")]`. Consider removing `RadixTree`/`MatcherTree`
  from it — nothing in getting-started uses them, and every name in a prelude is
  a name the reader must decide to ignore.
- **B3.** Delete `config: {}` from every example where it is empty. It is
  already optional (`config.rs:174` has `#[serde(default)]`, with a test) and
  writing it everywhere teaches that it is required.
- **B4.** `rumi info --verbose` printing each type URL's config field names.
  This is the highest-leverage single change for agent experience: it closes the
  authoring loop, so an agent never has to guess between `name`, `header` and
  `key`.
- **B5.** `rumi check` should report what it validated — rule count, inputs
  seen, fallback, max depth — not just `Config valid`.

**Already done, do not redo:** `rumi --skill` and `rumi --skill -r <name>` exist
(`rumi/cli/src/skill.rs`), generated from the live registries so they cannot
drift. `rumi run --trace` exists (`rumi/cli/src/trace_output.rs`) and renders
the extracted value against the matcher per rule.

**Done when:** `cargo add rumi-core`, paste the README example, and it compiles.
Verify by building a scratch crate outside the workspace.

---

### Phase SF — Schema freeze  *(milestone M4)*

**This phase writes tests, not fixes.** Every defect below crosses a schema
boundary — a field accepted on one side and dropped on the other. They are not
bugs to fix before migrating; they are the **acceptance criteria for the schema**.
Write each as a conformance fixture that fails today. Phase C is finished when
they pass.

That is this project's own stated workflow (`CLAUDE.md`: *"write fixture first,
conformance-driven development"*), applied to its largest phase for the first
time.

**Load `protocol-mastery` before starting.**

**SF0. Decide the schema shape. THIS IS A HUMAN DECISION — STOP AND ASK.**

**The deployment model, which you need before you can cost this.** x.uma is
embedded and **never speaks xDS itself**. Verified: no tonic, no
`DiscoveryRequest`, no subscription anywhere in the tree. The
`envoy_grpc_ext_proc` imports are type definitions, not a client. Config reaches
x.uma one of two ways:

```
human writes routes.yaml ──────────────► x.uma ──► Matcher
host's xDS client → proto Matcher ─────► x.uma ──► Matcher
```

The host owns the transport, the subscription, and ECDS. x.uma owns only the
step from config to matcher. `convert.rs:48` already exposes the right entry
point for the second path, `load_proto_matcher`; it has just never compiled.

Two consequences that are not up for debate:
- **No async in core, ever.** There is nothing to await when you never open a
  socket. This makes an existing judgment call structural.
- `envoy-grpc-ext-proc` as a *default* dependency pulls 101 crates against 7,
  including tokio, tonic, hyper and h2, into a library that never makes a
  network call. It is there for struct definitions. See E4.

**Now the actual question:** is protojson the **authoring** surface, or only the
**wire** surface?

This is not an implementation detail and an agent must not settle it alone. It
determines whether every example, fixture, playground preset and `--skill` output
in the repo becomes nine lines of camelCase with `@type` URLs and actions
promoted to `TypedExtensionConfig`, and whether puma and bumi each need a
YAML-lowering layer.

Envoy fuses the two paths into one schema: its YAML *is* protojson, run through
`JsonStringToMessage`. That is precisely why Envoy YAML is verbose — you are
hand-writing a wire format. **x.uma has a choice Envoy did not, because it owns
both loaders**, and the two paths have different audiences: the proto path is
consumed by a machine that generates it, where verbosity costs nothing; the YAML
path is typed by a human, where it costs a great deal.

The maintainer's current lean is **wire-only**: proto is the wire schema and the
validation authority, terse YAML stays the authoring surface and lowers into it.
The bridge is enforceable rather than asserted — a conformance fixture proves
terse YAML and its protojson equivalent build an *identical* matcher.

Honest cost of wire-only: two loaders to maintain, and the terse YAML becomes a
schema x.uma owns and must version.

What would overturn it: a control plane that needs to **emit** config a human
then hand-edits. Then one format must win and it has to be protojson. Confirm
that is not the model before committing.

Prepare both options with real costs — convert three representative fixtures each
way and count lines, concepts, and the diff to the docs — then **stop and put it
to the maintainer.** Record the answer in `DECISIONS.md` before writing migration
code.

Note the trap in the wire-only answer: if terse YAML survives as the authoring
dialect, the hand-written config layer is not retired, it is *renamed*. M5's gate
(`grep -rn MatcherConfig` returns only generated code) would then be satisfiable
by renaming, which is exactly the sort of gamed gate this plan exists to prevent.
If wire-only is chosen, replace that gate with one that has teeth.

**SF1–SF7. Write these fixtures. All must be red at the end of this phase.**

| # | Fixture asserts | Today |
|---|---|---|
| SF1 | `ignore_case: true` either matches case-insensitively or fails to load | silently dropped, matches case-sensitively (F6) |
| SF2 | `keep_matching: true` behaves as xDS specifies, or is rejected at load | accepted and ignored (F2) |
| SF3 | a config using `MatcherTree` / `exact_match_map` loads and dispatches | unreachable from config in all five implementations (F3) |
| SF4 | the test-domain input reads by its declared proto field name | proto says `value`, code uses it as a key (F13) |
| SF5 | `xuma.core.v1.StringMatcher` resolves to a real proto message | names a message that does not exist (F11) |
| SF6 | `custom_match` round-trips, and `register_core_matchers` works in all three languages | no-op in Python, absent in TypeScript, zero positive fixtures (F12) |
| SF7 | a path carrying a query string matches identically in every implementation | puma strips query, Rust does not (F14) |

**SF8. Fold in the renames that freeze with the schema.** Type-URL names are
frozen by publishing, so they must be decided here, not in Phase E:
- `xuma.test.v1.*` → `xuma.kv.v1.*`, with the crate split described in E2. The
  concept was never "test"; it is the CLI's default domain.
- Whether type URLs carry the `type.googleapis.com/` prefix. `protocol-mastery`
  states that convention; the code registers bare names. Pick one.
- The PascalCase `Exact` / lowercase `type:` inconsistency resolves itself if
  protojson wins. Confirm rather than assume.

**Done when:** every fixture above exists, fails for the documented reason, and
SF0 is answered in `DECISIONS.md`. **No production code changes in this phase.**

---

### Phase C — The migration  *(milestone M5)*

**Entry condition: Phase SF is complete and its fixtures are red.** This phase is
finished when they are green. If you are here without those fixtures, go back —
migrating without acceptance criteria is how the current defects got carried this
far.


This is the large one. Sub-phases are strictly ordered.

**Load `protocol-mastery` first.** It already answers questions this phase
otherwise re-derives: this project **commits generated code** (`SKILL.md:53`,
citing cncf/xds and go-control-plane as precedent), `buf.gen.yaml` v2 needs
`clean: true` to avoid stale files from renamed messages, config files use
ProtoJSON while wire transport uses binary proto, and ECDS is the intended
delivery channel for matcher configs. It also flags two things this repo is
currently doing wrong: type URLs should carry the `type.googleapis.com/` prefix
(the code registers bare names), and hard-coding type URLs as strings is listed
under Common Pitfalls with "use the proto `Name` trait" as the fix.

- **C0. Add `clean: true` to `buf.gen.yaml`.** One line, and it is the documented
  pitfall for exactly the stale-file class this phase is untangling.

- **C1. Make codegen produce a compiling crate.** Three stacked failures, all
  confirmed:
  1. `buf.lock` was empty; dependencies were never resolved. `buf dep update`
     fixes it. (Already run; the lock is populated.)
  2. `buf generate` only generates the local module. `lib.rs` includes
     `gen/xds/core/v3/...` and `convert.rs:27,274` genuinely uses those types,
     but no xuma proto imports xDS, so they are not in the generation graph.
     `buf generate buf.build/cncf/xds --template buf.gen.yaml` produces them
     (28 files total, ~136K).
  3. With the types present it reaches real compilation and fails:
     `pbjson_types::Any` does not implement `Eq`/`Hash` but the generated code
     derives both. This is a codegen configuration problem. Fix it in
     `buf.gen.yaml` (prost type attributes) rather than by hand-editing
     generated files.

  **`rumi/proto/src/gen` is gitignored** — that is the mechanical reason it has
  never been committed, and it contradicts `protocol-mastery/SKILL.md:53`, which
  states this project commits generated code (citing cncf/xds and
  go-control-plane). Remove the ignore rule before anything else in this phase,
  or the commit silently does nothing. Verify with `git check-ignore -v`.

  Then commit `rumi/proto/src/gen/` and wire the dep generation into `just gen`
  so it is reproducible. Add a CI check that `just gen` produces no diff — that
  makes drift impossible rather than policed.

- **C2. Prove the conversion works.** `rumi/proto/src/convert.rs` already
  contains eight end-to-end tests (proto → convert → load → evaluate). Run them.
  Add `-p rumi-proto` to the CI test job so a crate that does not compile can
  never again be marked done.

- **C3. Measure the fixture migration.** Push all 27 fixtures in `spec/tests/`
  through the protojson path. This tells you the true size of the migration,
  which nobody currently knows because it has never been carried to completion.

- **C4. Answer the authoring-surface question** from §2 and record it in
  `DECISIONS.md` before writing migration code.

- **C5. Retire the hand-written config types.** Rust first — it is the reference
  implementation and `convert.rs`/`any_resolver.rs` already live there. Then
  puma, then bumi, porting fixtures against a working reference rather than
  three at once against a moving target. Delete `MatcherConfig`,
  `FieldMatcherConfig`, `PredicateConfig`, `SinglePredicateConfig`,
  `OnMatchConfig`, `ValueMatchConfig` and their Python/TypeScript twins. Do not
  leave aliases.

**Done when:** one schema. `buf generate` is the only source of config types in
all three languages, all 27 fixtures pass through protojson in every
implementation, and `grep -rn "MatcherConfig" rumi puma bumi` returns only
generated code.

---

### Phase S — Security

**Release-blocking. A full review was run and the verdict was DO NOT SHIP in
current form.** Three findings were reproduced on-machine. Each fix below has a
falsifying test in the review; commit those as regression fixtures.

**S1 — bumi compiles a regex bomb from 20 characters. BLOCKING.**
`bumi/src/string-matchers.ts:91` calls `RE2JS.compile(pattern)`. `re2js`
implements neither of C++ RE2's compile-time guards: no `max_mem` program budget
and no nested-repetition product limit. `MAX_REGEX_PATTERN_LENGTH = 4096` bounds
pattern *length*; cost is driven by compiled *program size*. Wrong axis.

Measured on this machine:
```
a{100}                 6 chars     2ms     33MB
(a{100}){100}         13 chars     5ms     45MB
((a{100}){100}){100}  20 chars   279ms    282MB
```
One more nesting level measures 3.25 s / 2.19 GB; 27 characters extrapolates to
an OOM kill. **puma rejects the 13-char pattern outright** (`invalid repetition
size`), and Rust's `regex` crate rejects on its 10 MB size limit. bumi is the
only implementation without a compile-time guard.

Fix: bound it in `RegexMatcher`'s constructor — reject a `{n}` whose operand
already contains a `{m}`, mirroring RE2's own rule. `re2js` exposes no `max_mem`
option. Add a conformance fixture asserting all three implementations reject it.

**S2 — every resource limit is enforced in the config loader, not the
constructor it protects. BLOCKING, and it is the architectural root of S1.**
`Registry::check_pattern_length` is a *private method on `Registry`*
(`registry.rs:611`), called from one place. The public constructor it should
protect — `StringMatchSpec::to_input_matcher()` (`string_match.rs:56`) — checks
nothing. Confirmed: `HookMatch::compile` accepted an **8 MB** pattern against an
8192 limit.

The same shape reproduces independently in all three languages, which is what
makes it structural rather than an oversight. Bypass paths: both domain
compilers in Rust (`grep MAX_ rumi/ext/http/src/*.rs` returns nothing), direct
`RegexMatcher` construction plus the gateway in Python and TypeScript, and the
playground's `configToGraph`, which calls `parseMatcherConfig` without
`loadMatcher` and so inherits no limits at all.

Fix: move each check into the constructor of the thing being limited, then
delete `Registry::check_pattern_length` — the loader inherits the guarantee.
`MatcherError::PatternTooLong` already exists, so no new dependency. **The rule
to carry forward: the type that holds the resource owns the limit on that
resource.**

**S3 — `session_id` is accepted, counted as a constraint, then silently
discarded. BLOCKING. Total agent-gate bypass.**
`crusts/python/src/convert.rs:37` includes `session_id.is_none()` in the
empty-match guard, but core's `HookMatch` (`claude/config.rs:38-49`) **has no
`session_id` field** — verified. So a rule scoped only to `session_id` passes
the guard that exists specifically to prevent accidental catch-alls, converts to
an all-`None` `HookMatch`, and `Predicate::from_all(vec![], catch_all())`
returns a matcher that matches everything.

In an allowlist gate that means every tool call in every session is permitted by
a rule the operator believes is scoped to one session. Present in both the PyPI
wheel and the npm package.

Fix, smallest first: reject the field with a clear error until it is
implemented. Better: add `session_id` to `HookMatch` and a `SessionIdInput` —
`HookContext::session_id()` already exists (`claude/context.rs:189`). Add a
round-trip test asserting every non-`None` FFI field is represented in the core
struct; that catches this whole class.

**Follows in 0.1.1, not release-blocking:**
- Aggregate regex budget. 256 field matchers × a 13-byte regex = 3,328 bytes of
  config → **2.9 s of CPU** in release, with every declared limit respected.
  Per-item limits with no aggregate. Use `RegexBuilder::size_limit()` with a
  budget drawn from `rumi/core/benches/redos.rs`, not intuition.
- Empty rule list compiles to a catch-all (`claude/compiler.rs:119`,
  `http/compiler.rs:138`). Polarity depends entirely on the caller's action
  assignment: `compile_hook_matches(&[], "allow", Some("deny"))` allows
  everything. The crusts already solved this for the single-rule case with
  `match_all`; extend the same ceremony to the list.
- `validate()` is never called by either domain compiler (F6 above). The crusts
  do call it and are currently the only paths enforcing depth on compiler
  output.
- `serde_yaml` is archived upstream (March 2024) and is a **non-optional runtime
  dependency of three published artifacts**. No advisory covers it, which is
  exactly why `cargo audit` is clean and why a clean audit is not the same as
  safe. `serde_yml` carries RUSTSEC-2025-0068, so it is not the swap; use
  `serde_yaml_ng` or `serde_norway`.
- CI actions are all floating tags, none SHA-pinned, including in workflows
  holding `CARGO_REGISTRY_TOKEN` and `NPM_TOKEN`. `dtolnay/rust-toolchain@stable`
  is a *branch*. PyPI already uses OIDC trusted publishing; crates.io and npm
  now support it too.

**Controls that are already correct — do not weaken these without knowing what
they hold up:**
1. **`serde`'s 128-level recursion limit** is what actually stops config-borne
   stack exhaustion, *not* `MAX_DEPTH`. Measured: depth-50 configs are already
   rejected; roughly 100,000 levels are needed to overflow the stack. Changing
   deserializer, or calling `disable_recursion_limit()`, removes this silently.
   **Note for C1: prost has no equivalent guard**, so the proto config path needs
   its own depth check.
2. `serde_yaml` rejects YAML alias bombs — a 325-byte billion-laughs payload
   returns `repetition limit exceeded` in 30 ms. Whatever replaces it must be
   re-tested for this.
3. Rust `regex`'s 10 MB `size_limit` is the only thing capping the per-regex
   ceiling.
4. `google-re2` rejecting nested counted repetition is why puma is immune to S1.
   Do not swap it for a pure-Python engine.
5. `AnyResolver` is a closed-world allowlist (`any_resolver.rs:112-119`) —
   decoders monomorphized at registration, no reflective lookup. The classic
   polymorphic-deserialization failure is absent by construction.
6. `#[serde(deny_unknown_fields)]` on `HookMatch` and `ArgumentMatch`. Without
   it, a typo'd field in a deny rule silently produces S3's catch-all from YAML.
7. Zero `unsafe` in the entire tree — verified.
8. No `panic = "abort"` in any profile, so PyO3's `catch_unwind` works and Rust
   panics surface as `PanicException` rather than unwinding into CPython.
   Setting it for wheel size would break this.
9. No XSS in the docs site. Zero sink hits across `docs/experience/src`; SVG
   labels use `textContent`, not string concatenation; no URL-seeded state.
   `rehype-raw` processes only build-time repo markdown behind a hardcoded slug
   manifest — which makes raw HTML in a docs `.md` a review obligation, not a
   runtime hole. **If a playground share-link is ever added, re-audit** — it
   becomes remotely triggerable.
10. No regex metacharacter injection: prefix/suffix/contains/exact use native
    string operations in all three languages.

**Done when:** S1, S2 and S3 are fixed with their falsifying tests committed as
regression fixtures, and every item under "controls that are correct" has a test
or a comment naming what it protects.

---

### Phase E — Publishability: all of it, by crate design

**Everything ships.** `rumi-core`, `rumi-http`, `rumi-cli`, `xuma` on PyPI,
`xuma` on npm, `xuma-crust` on both. The blockers are dependency edges, not
missing capability, and the fix is crate structure rather than cutting features.

The governing principle from `.claude/skills/rust-mastery/SKILL.md:23` —
**features are strictly additive and never change core behaviour** — means a
`publish = false` crate on any dependency edge is a structural bug, not a
feature-flag question. `cargo publish` resolves optional dependencies too, so
gating does not help.

- **E1. Publish `rumi-proto`** (unblocks `rumi-http`, F10). Once C1 lands it is
  real, compiling code: `any_resolver.rs`, `convert.rs`, and committed generated
  types. It is the xDS control-plane path, which is the reason proto-first was
  chosen. Flip `publish = false`, give it a description, keywords, categories
  and a README. Publishing generated code is normal for prost crates; committing
  `gen/` in C1 is what makes it honest.

- **E2. Split `rumi-test` into `rumi-kv` + `rumi-test`** (unblocks `rumi-cli`,
  F9). Two concerns are fused in one crate:
  - **`rumi-kv`** — the generic key-value domain: `KvContext`, `xuma.kv.v1.*`
    type URLs, the registry registration. **Publishable.** This is what the CLI
    actually depends on and what it advertises as its default domain. The
    concept was never "test"; the misleading name is what predicted the release
    blocker.
  - **`rumi-test`** — YAML fixture loading for the conformance suite only.
    Stays `publish = false`. Nothing user-facing depends on it.

  **The type-URL rename `xuma.test.v1.*` → `xuma.kv.v1.*` is decided in SF8, not
  here** — names freeze with the schema, and an earlier draft had this task
  needing to land before a phase the milestone table placed after it. By the time
  you reach E, the names are settled and this is a mechanical crate split.

- **E3. Remove `rumi-test` from both crusts** (S-5). They declare it with
  `features = ["fixtures"]` — a YAML fixture loader inside a published wheel and
  npm package. They want `rumi-kv`.

- **E4. Split `ext-proc` and set `default = []` on `rumi-http`** (S-2, and it
  subsumes B2). The feature currently conflates two unrelated things:
  `k8s-gateway-api` supplies config types the compiler needs;
  `envoy-grpc-ext-proc` supplies data-plane types. Split into `gateway` and
  `ext-proc`. Set `default = []`.

  Measured: `default = ["ext-proc"]` pulls **101 crates versus 7** with
  `--no-default-features`, including tokio, tonic, axum, hyper, h2, prost,
  chrono, and two major versions of `http`. The heaviest root is
  `envoy-grpc-ext-proc v0.1.2` — **1,121 total downloads**, published 2025-10,
  single third-party maintainer, pre-1.0. That should not be a default
  dependency of a published library. Every consumer in this repo already opts
  out, which is the tell: the project never uses its own default.

  **Feature defaults are frozen after first publish.** This is the last free
  moment.

- **E5. Package hygiene for every artifact** (S-6). Each package root needs its
  own `LICENSE-MIT` and `LICENSE-APACHE` — the manifests declare
  `MIT OR Apache-2.0` but the files exist only at repo root, outside what gets
  packaged. `rumi/ext/http/README.md` does not exist, so its crates.io page
  would ship bare. Every published crate needs `description`, `keywords`,
  `categories`, `readme`, `repository`.

- **E6. Make `release.yml`'s dry-run predict the real run.** `cargo publish
  --dry-run` currently passes locally only via path patching and would be
  rejected by crates.io. Verified: `no matching package named 'rumi-proto'`.
  A dry-run that does not predict is worse than none.

- **E7. Publish order is a dependency chain and the workflow must respect it.**
  `rumi-core` → `rumi-proto` → `rumi-kv` → `rumi-http` → `rumi-cli`. Dependents
  cannot resolve until their dependency is live on crates.io, which can take a
  minute to index. `release.yml:97-119` currently publishes three crates in
  sequence with no gate; triggering it today would publish `rumi-core 0.0.2`,
  then hard-fail — **and crates.io versions can never be re-uploaded.** Add an
  index-availability wait between steps.

**Done when:** `cargo publish --dry-run` passes for all five crates in
dependency order with no path patching, and no published crate depends on a
`publish = false` crate.

---

### Phase F — CI as the arbiter

The general lesson from this whole exercise: **what CI does not check is not
true.** Every false claim found was outside CI's reach.

- **F1.** Add crust jobs. Two of five implementations are never built (F5).
- **F2.** Add `-p rumi-proto` to the test job.
- **F3.** A job that runs the README's literal config through every shipped
  runtime and asserts identical results. The README is the correctness claim;
  make CI enforce it.
- **F4.** The doc-command smoke test from A4.
- **F5.** `just gen` produces no diff.

**Done when:** every roadmap ✅ corresponds to something CI executes.

---

### Phase H — Documentation polish and cross-machine reproducibility

Phase A repaired the lies. This makes the whole thing *good*, and makes it work
on a machine that is not yours.

**H1 — Reproducible on any dev machine.** Today a fresh clone needs `just`,
`cargo`, `uv`, `bun`, `buf`, `mdbook` (no longer), `maturin`, `wasm-pack` and
`cargo-audit`, and nothing states that or checks it. Add:
- A `just doctor` that checks every required tool, reports version and what is
  missing, and exits non-zero. First command in `CONTRIBUTING.md`.
- Pin toolchains: `rust-toolchain.toml`, `.python-version` (already implied by
  `requires-python`), a Bun version. A `rust-version` (MSRV) field in the
  manifests — currently declared nowhere, so the only MSRV statements in the
  repo are inside a skill file.
- Decide the lockfile question recorded as open in D-016. `.gitignore:3` ignores
  `Cargo.lock` while `bumi/bun.lock` is tracked; the rule and reality disagree.
  A workspace shipping a binary normally commits `Cargo.lock`. Reproducible CI
  argues for committing all of them.
- `just verify-clean-clone` — build from `git archive` output rather than the
  working tree. This is the check that would have caught the 17 files missing
  from a commit because `.gitignore` had a bare `lib/` pattern. `just ci` runs
  against the working tree and structurally cannot catch that class.

**H2 — `CONTRIBUTING.md`.** Does not exist. Prerequisites, `just doctor`, the
clippy-then-fmt order, how to run one implementation's tests, how to add a
conformance fixture, and the rule that a fixture comes first.

**H3 — Root `SECURITY.md`.** Exists in `puma/` and `bumi/` but not at root and
not for the Rust crates. Include the reporting channel and the resource-limit
model, since that is the part consumers must reason about.

**Done when:** a colleague with none of the toolchain clones the repo, runs
`just doctor`, follows `CONTRIBUTING.md`, and has green tests without asking a
question.

---

### Phase K — The Claude hook contract  *(release-blocking, and not documentation)*

An earlier draft filed this under "documentation polish". It is a **new CLI
feature on an agent-safety path**, and as a docs subtask it would have received
docs-grade testing on the one code path where a wrong exit code silently permits
a tool call. That mislabelling is the finding.

A user-persona review reached a config they believed in and then stopped, 75
minutes in, because nothing documents how `rumi run claude` connects to an actual
hook. Real hooks deliver JSON on stdin and consume exit codes or JSON output.
The CLI takes `--event/--tool/--arg` flags, and `grep stdin rumi/cli` returns
nothing. Needed: a `rumi run claude --stdin` mode (or equivalent), the exit-code
contract, and a worked `settings.json` example. The README demos blocking
`rm -rf` on its front page; there is no path from that demo to a working gate.


**Fixtures required, not optional.** At minimum: a malformed hook payload on
stdin must not exit with the allow code; an unparseable payload must fail closed;
and the exit-code mapping must be asserted for match, no-match, and load-error.

**Done when:** a worked `settings.json` gates a real `Bash` call, and the fixtures
above are in CI.

---

### Phase L — Post-release polish  *(0.1.x, NOT release-blocking)*

Publishing freezes the schema, the crate structure, feature defaults, type-URL
names, and the security posture of shipped artifacts. It does not freeze prose or
doc-site navigation. Everything here ships after 0.1.0 and is better for it —
H7's five-minute-path re-review is only meaningful against the post-migration
docs anyway.

**H4 — Docs site completeness.** The four Diátaxis quadrants are structurally
present; check each is actually populated after the proto-first migration
reshapes every example. Specifically: every config sample changes in C5, and the
live `<matcher>` components in `docs/content/how-to/*.md` execute real configs —
they will break loudly, which is the point.

**H5 — `CHANGELOG.md`.** Does not exist. Start it at 0.1.0 with the format
change called out at the top, since anyone who found the repo pre-release needs
to know the config shape moved.

**H6 — API docs.** `cargo doc` is assembled into the site at `/api/rust` by
`docs.yml`. Python and TypeScript API docs were dropped when mdBook was retired
(`docs-python`/`docs-typescript` targets are gone). Either restore them into the
same assembly step or remove `docs/content/reference/api.md`'s promise of them.

**H7 — The five-minute path, re-verified end to end.** After everything above:
land on the site, understand what x.uma does, run something, get a decision.
Re-run the ACE-as-user review against the finished state.

**H9 — Surface the skill content on the docs site.** The four-sentence evaluation
model and the authoring reference that `rumi --skill` prints are, per that
review, "the best writing in the project" — and they ship sealed inside a binary
a newcomer has not built yet. In particular the missing-input rule ("if an input
finds no value, the predicate is false, that is not an error") appears nowhere on
the site as a plain statement, and for a safety gate that rule is the whole
ballgame. Generate the docs page from the same source as the CLI output so the
two cannot diverge.

**H10 — Give Claude a door.** The docs-site quadrants have no Claude entry;
Claude hooks live inside the *Rust* getting-started page and are findable only by
grep. The playground's `ModeTabs` offers Config and HTTP only, and its
Claude-flavoured "Block rm -rf" preset is emulated with `xuma.test.v1.StringInput`
keys — copy it into `rumi run claude` and it fails at load. Add a Claude mode, or
relabel the preset so it does not teach a config that cannot work.

**H11 — One format papercut. Check whether it still exists first.**

The PascalCase `Exact` amid lowercase `type:` is a Rust serde enum default that
leaked into a cross-language wire format. **Phase C's protojson migration very
likely deletes it outright** — confirm before spending anything here.
What survives regardless:
`config.md` never shows a compound predicate nested inside another compound, so
the reviewer had to infer nesting from a fixture written in the wrong dialect.
And the casing rule is never stated: `type: and` and `on_match: { type: action }`
are lowercase while `value_match: { Exact: … }` is PascalCase — a Rust serde
enum default leaked into a cross-language wire format. One nested example and one
sentence about casing.

**Done when:** a colleague with none of the toolchain clones the repo, runs
`just doctor`, follows `CONTRIBUTING.md`, and has green tests without asking a
question.

---

### Phase G — Release

Only after A–F are on green PRs and a human has merged them.

1. Version bump to `0.1.0` across all manifests. The format changed; `0.0.2` is
   not honest.
2. `CHANGELOG.md` — it does not exist yet.
3. Tag, then run the release workflows (`workflow_dispatch`, never yet run).
4. Publish in dependency order. `rumi-core` first; dependents cannot resolve
   until it is live.
5. Remove the "not yet on registries" notes from `README.md` and the three
   getting-started pages, and restore the plain install instructions.

---

## 6. Definition of done

Tarmac, concretely, for this project:

- [ ] Every doc code sample carries a `run` / `compile` / `cli` / `future`
      marker, and CI enforces each class (no unmarked blocks)
- [ ] Every roadmap ✅ corresponds to something CI runs
- [ ] One config schema, generated, in all three languages
- [ ] No path where a config loads clean and returns a wrong answer
- [ ] `cargo add rumi-core` plus the README example compiles unmodified
- [ ] `cargo publish --dry-run` passes for every published crate
- [ ] All five implementations agree on every fixture, including the SF ones,
      proven by CI
- [ ] Every §4 finding has become a test, and its row here is struck
- [ ] Every asymmetry stated plainly in the docs — Claude is Rust-only until it
      is not
- [ ] `just ci` green, including `cargo audit`
- [ ] Security review findings triaged, each fixed or explicitly accepted in
      `DECISIONS.md`

## 7. As you go

Append to `DECISIONS.md` whenever you make a call a future reader would
otherwise have to reconstruct. Follow the existing format: what was decided,
why, and what would justify revisiting. Newest first.

If you find something this plan gets wrong, fix the plan in the same PR as the
work. A stale plan becomes the next thing that lies.
