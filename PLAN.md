# Release plan — x.uma 0.1.0

**Written for an agent arriving with no context.** Read this file top to bottom
before touching anything. Every task tells you how to prove you are done.

Claims were verified against the code on 2026-08-16 and **re-verified on
2026-08-17**, when several were found to be wrong and were corrected in place —
see §4, where the corrections are called out rather than quietly applied. Treat
it as a lead list, not as truth, and re-verify before acting. That is the same
rule the plan applies to everything else.

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
`CLAUDE.md` (project), `DECISIONS.md` (D-001 to D-027, newest first — **D-026
settles the config format and is load-bearing for Phases SF, C and E**),
`reference/security-review-2026-08-16.md` (the full security review; Phase S
depends on it), `reference/prior-art-2025-design.md` (2025 design conversations
recovered from memex). Both live in `reference/` and are **tracked** — they used
to sit under the gitignored `scratch/`, where a fresh clone could not see them.

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
  only, bypassed by both domain compilers. Phase S's SEC2 takes it to 3.
- `keep_matching`: currently **0 across the board** and documented as enforced.
  That combination is the worst square on this table.

### Which axes apply to which work

Not every task lives on all four axes. Demanding "adversarial testing" for a
markdown table rename produces a pile of exceptions, and a rubric that is mostly
exceptions is a ritual. Score only what applies:

| Task class | Axes that apply | Example |
|---|---|---|
| Behaviour change | all four | SEC2 moving limits into constructors |
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
| **M2** | A stranger can start | B, H1–H3 | README example compiles unmodified in a scratch crate **outside** the workspace, resolving `rumi-core` by path. The `cargo add rumi-core` line itself is a `future` block naming M7 — it cannot resolve until then, and a gate that requires it is the same unsatisfiable shape M1's taxonomy exists to fix. `just doctor` passes on a machine with nothing installed. |
| **M3** | Nothing loads clean and lies | S | SEC1, SEC2, SEC3 fixed, each with its falsifying test committed as a regression fixture. |
| **M4** | **Schema freeze** | SF | Every defect in §4 that crosses a schema boundary exists as a **failing** conformance fixture, and the schema's shape is decided and recorded. |
| **M5** | One schema | C | The M4 fixtures pass. `buf generate` is the only source of config types in all three languages, and all three `gen/` trees are **tracked** (F20). All 27 fixtures pass through the frozen schema everywhere, or C4 records why a fixture legitimately does not. `just test-full` compiles (F19). |
| **M6** | Everything is publishable | E, F, K | `cargo publish --dry-run` passes for all five **crates** — `rumi-core`, `rumi-proto`, `rumi-kv`, `rumi-http`, `rumi-cli` — in dependency order with no path patching. No published crate depends on a `publish = false` crate. CI builds every artifact, including the two crusts and the puma/bumi packages (seven artifacts total; the crates are a subset). |
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

**That check needs state, and the plan does not define any.** "CI asserts the
milestone is still unreached" requires a machine-readable record of which
milestones have passed. Add one in Phase A — a `MILESTONES.toml` at repo root,
or a table in this file that the checker parses. Whichever, it is the single
source and reaching a milestone means editing it. Without this the taxonomy's
sharpest marker is unimplementable.

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

Decisions of record are in `DECISIONS.md` (D-001 to D-027). Read it. Prior art
recovered from 2025 design conversations is in
`reference/prior-art-2025-design.md`.

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

**The authoring-surface question is answered. See `DECISIONS.md` D-026.**
protojson is the **authoring** surface, not merely the wire surface. Config files
stay — rules are defined in YAML or JSON files, both accepted — but their
contents follow protobuf's canonical JSON mapping. The terse dialect is retired,
not kept as a lowering layer.

The reason is the `x`: x.uma exists to implement the xDS matcher API across
languages, and a bespoke config dialect makes that premise false on the one
surface users actually touch. Verbosity (a compound rule goes from ~11 lines to
~27) is accepted; where hand-authoring ergonomics matter the answer is tooling —
a rule builder or graph export — not a second schema.

Do not re-open this in Phase C. If you believe it is wrong, the only thing that
overturns it is a rule protojson cannot express.

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

**Green (re-measured 2026-08-17 on merged `main`):** **276** Rust tests under
`just test` — note that is `default-members`, which excludes `rumi-proto` — 294
Python, 258 TypeScript, 27 conformance fixtures, `cargo audit` clean, docs site
builds and deploys. PR #22 is **merged**; `main` is the baseline now.

**Broken or untrue:**

| # | Finding | Evidence |
|---|---|---|
| F1 | `rumi-proto` has never compiled | Reproduced 2026-08-17: `cargo check -p rumi-proto` → 4 errors, `pbjson_types::Any: Eq`/`Hash`. **Correction to the original evidence:** it is *not* true that `gen/` was never tracked — `935ed9f` added 14 files and `e36dd29` removed them, both ancestors of HEAD. 28 files (960K) sit there untracked today. See C1. |
| F2 | `keep_matching` documented as an enforced invariant, not implemented | `CLAUDE.md:213`. **Corrected evidence:** not "zero occurrences" — there are 8, all `keep_matching: false` literals in `rumi/proto/src/convert.rs`'s test module. Zero in `rumi/{core,ext,cli,crusts}`, `puma/src`, `bumi/src`. |
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
| F19 | **`just test-full` does not compile** | `cargo test --all-features` → the same 4 errors as F1, because `--all-features` enables `rumi-test/proto` → `rumi-proto`. `just ci` passes anyway: `just test` uses `default-members` (`rumi/Cargo.toml:6`) and `justfile:68` hard-codes `--exclude rumi-proto`. A shipped `just` target is red and the gate cannot see it. |
| F20 | Generated code is gitignored in **all three** languages | `.gitignore:58-60` covers `rumi/proto/src/gen/`, `puma/proto/src/gen/`, `bumi/proto/src/gen/`. All three exist on disk untracked (28 / 7 / 6 files); nothing imports the Python or TypeScript ones. M5's "all three languages" gate and CI5's no-diff check are both vacuous over untracked trees. |
| F22 | The plan cited files a clone does not contain | `.gitignore:63` ignores `scratch/` wholesale, and §0 told the reader to open `scratch/phase-12/prior-art.md`. Fixed 2026-08-17 by moving both cited artifacts to a tracked `reference/`. Left as a row because the *class* recurs: `just verify-clean-clone` (H1) is the check that would catch the next one. |
| F21 | Fixtures use **four** dialects, not three | `matcher:` (14), `config:` (7), `http_route_match:` (5), `http_route_matches:` **plural** (1, `spec/tests/05_http/multiple_routes.yaml:5`), each with its own branch in all three loaders. A5 says three. |

**Security review: complete, and now in the repo.**
`reference/security-review-2026-08-16.md` — recovered 2026-08-17
from the session transcript that produced it, having never been written to disk.
Read it before Phase S; the falsifying tests SEC1–SEC3 need are in it, several
already run and passing.

**Its numbering does not match this document's.** The review uses `F-01..F-06`,
`S-1..S-5`, `L-1..L-4`. This plan uses `F1..F21` for §4 findings and `SEC1..SEC3`
for Phase S tasks. When you see a bare `F6`, check which document you are in.
The mapping is in Phase S.

**And it contains at least one error.** Review F-04 says there is no
`SessionIdInput`; there is, at `rumi/core/src/claude/inputs.rs:48`, registered at
`claude/mod.rs:72`. Treat the review as evidence, not as truth — the same rule
this section applies to itself.

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

- **A5.** Rewrite `spec/tests/README.md`. **Read this whole item before starting;
  it is not the cheap docs task the phase header implies.**

  The fixtures use **four** incompatible schemas, not three — `matcher:` (14),
  `config:` (7), `http_route_match:` (5), and `http_route_matches:` plural (1,
  `spec/tests/05_http/multiple_routes.yaml:5`). Each has its own branch in each
  of three loaders (`rumi/ext/test/src/fixture.rs`, `puma/tests/conftest.py:213`,
  `bumi/tests/helpers/fixture-loader.ts:180`). The README documents `matcher:`,
  which no user can write.

  **Do not rename the keys yet.** An earlier draft said to rename `matcher:` →
  `native_matcher:` and `http_route_match:` → `compiler_route_match:`. D-026
  retires the terse config dialect, and C4 decides whether the 14 `matcher:`
  fixtures migrate to protojson or stay as native-construction tests. If they
  migrate, renaming their keys first is throwaway work in three loaders.

  So A5 splits:
  - **Now:** document what exists, honestly — four dialects, which one is the
    shipping format, which loader reads which. Say plainly that this is
    transitional and points at C4.
  - **After C4:** rename or delete, once you know which survive.

  Rubric note: the rubric's task-class table files this under *Docs and prose*.
  That is wrong for the second half — renaming keys parsed by three loaders is a
  **behaviour change** and scores on all four axes.

**Done when:** every code block in `README.md` and `docs/content/` carries
exactly one `run` / `compile` / `cli` / `future` marker per §0.9's taxonomy, and
CI enforces each class. (The earlier wording — "either executes in CI or does not
exist" — predates the taxonomy and is the unsatisfiable form §0.9 replaced.)
`just ci` green.

---

### Phase B — DX defaults

**Independent of C. Do it before publish; it is free now and breaking later.**

- **B1.** `rumi/core/Cargo.toml`: `default = ["registry"]`. Config loading is
  the entire pitch and it is currently opt-in behind an undocumented flag (F7).
  Cost is `serde` + `serde_json` in core, which is the price of the advertised
  feature.
- **B2.** Add `Registry`, `RegistryBuilder`, `TypedConfig`, `IntoDataInput`,
  `IntoInputMatcher`, `UnitConfig` to `rumi::prelude` under
  `#[cfg(feature = "registry")]`.

  **Do not add `MatcherConfig`** — C5 deletes it and D-026 retires the format it
  describes. An earlier draft listed it here, which would have put a prelude
  export and its deletion in the same release, two milestones apart. Whatever
  replaces it as the loaded-config type after C5 goes in the prelude then, not
  now. Consider removing `RadixTree`/`MatcherTree`
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

**Done when:** the README example compiles unmodified in a scratch crate outside
the workspace, with `rumi-core` resolved by path. `cargo add` is verified at M7,
not here — see M2's gate.

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

**SF0. The schema shape. DECIDED 2026-08-17 — `DECISIONS.md` D-026.**

**protojson is the authoring surface.** One schema for authoring and wire alike.
Config files stay — YAML or JSON, both already accepted by the loader
(`rumi/cli/src/main.rs:379-381`) — and their contents follow protobuf's canonical
JSON mapping. The terse dialect (`type: and`, `value_match: { Exact: … }`, bare
string actions) is **retired, not preserved** as a lowering layer.

The deciding argument was not verbosity. It was that x.uma exists to implement
the xDS matcher API across languages, and a bespoke config dialect makes that
premise false on the one surface users touch. Verbosity is real — a compound rule
goes from 11 lines to 27 — and is accepted; hand-authoring ergonomics are a
tooling problem (rule builder, graph export), not a schema problem, and that
tooling is post-release.

Read D-026 for the full reasoning, the consequences it settles, and what would
overturn it. **Do not re-open it here or in Phase C.**

**What this decision settles elsewhere in this plan:**

| Settled | Was |
|---|---|
| Type URLs carry `type.googleapis.com/` | open in SF8; protojson requires it in `@type` |
| F13 (`StringInput.value` used as a key) is release-blocking | a schema wart |
| PascalCase `Exact` beside lowercase `type:` (H11) | deleted by construction, not documented |
| M5's gate has teeth | was satisfiable by renaming if the dialect survived |
| `rumi --skill` / `rumi info --verbose` are critical path | Phase B polish |

**Still open, and it is Phase C's to settle:** the 14 fixtures using the
`matcher:` dialect load through `rumi/ext/test/src/fixture.rs`, which builds
matchers directly and never touches the config path. Whether they migrate or stay
as native-construction tests is a separate call. See A5, which must not rename
their keys before this is answered.

**The deployment model, which the decision rests on.** x.uma is embedded and
**never speaks xDS itself**: no client we wrote and no subscription in our own
source. (Careful with the stronger claim — `cargo tree -p rumi-http` *does* show
`tonic v0.14.6` under default features, pulled transitively by
`envoy-grpc-ext-proc`. That is exactly what E4 removes.) Config reaches x.uma one
of two ways:

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

Envoy fuses the two paths into one schema: its YAML *is* protojson, run through
`JsonStringToMessage`. x.uma now does the same thing, for the same reason.

**What the format actually looks like**, so nobody has to re-derive it. Field
names below come from the generated serde impls
(`gen/xds/type/matcher/v3/*.serde.rs`, `gen/xds/core/v3/*.serde.rs`), not from
memory. Today's `spec/tests/06_config/02_compound_predicates.yaml`, 11 lines:

```yaml
matchers:
  - predicate:
      type: and
      predicates:
        - type: single
          input: { type_url: "xuma.test.v1.StringInput", config: { key: "role" } }
          value_match: { Exact: "admin" }
        - type: single
          input: { type_url: "xuma.test.v1.StringInput", config: { key: "org" } }
          value_match: { Prefix: "acme" }
    on_match: { type: action, action: "admin_acme" }
```

The same rule as protojson, 27 lines:

```yaml
matcherList:
  matchers:
    - predicate:
        andMatcher:
          predicate:
            - singlePredicate:
                input:
                  name: role-input
                  typedConfig:
                    "@type": type.googleapis.com/xuma.test.v1.MapInput
                    key: role
                valueMatch:
                  exact: admin
            - singlePredicate:
                input:
                  name: org-input
                  typedConfig:
                    "@type": type.googleapis.com/xuma.test.v1.MapInput
                    key: org
                valueMatch:
                  prefix: acme
      onMatch:
        action:
          name: admin_acme
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: admin_acme
```

Three things that example exposes, each of which is a task below:

1. The action stops being a string. `NamedAction`
   (`proto/xuma/core/v1/action.proto`) is the adapter between xDS's
   "action is a `TypedExtensionConfig`" and rumi's `A = String`. It is currently
   referenced **only** in `convert.rs`'s test module, so its shape is still free.
2. That says `MapInput`, not `StringInput` — because the proto's `StringInput`
   has one field, `value`, while the Rust `StringInput` holds a `key` and does a
   map lookup (`rumi/ext/test/src/lib.rs:61-75`). Terse YAML hid this by passing
   `config` through opaquely. protojson cannot. That is F13 / SF4, and D-026
   makes it release-blocking.
3. `keepMatching` is a plain boolean on every `onMatch`, one keystroke from being
   set and doing nothing. That is F2 / SF2.

**This example is hand-derived and unexecuted** — `rumi-proto` does not compile
(C1). Treat it as schema-correct and runtime-unverified until C2 runs.

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
- ~~Whether type URLs carry the `type.googleapis.com/` prefix.~~ **Settled by
  D-026.** protojson requires the full URL in `@type`, so the bare names the code
  registers (`registry.rs`) are now a defect. Write a fixture that fails on a
  bare name.
- ~~The PascalCase `Exact` / lowercase `type:` inconsistency.~~ **Deleted by
  D-026**, since the dialect that carried it is retired. Confirm with a grep at
  the end of Phase C rather than assuming.
- **SF9 (new).** A fixture asserting the same rule, written as protojson-in-YAML
  and as protojson-in-JSON, builds an identical matcher. Both syntaxes are
  already accepted (`main.rs:379-381`); D-026 makes that a supported guarantee
  rather than an accident, so it needs a test.

**Done when:** every fixture above exists and fails for the documented reason.
**No production code changes in this phase.** SF0 is already answered — do not
re-cost it.

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
     (28 files, 960K on disk).
  3. With the types present it reaches real compilation and fails, 4 errors:
     `pbjson_types::Any` implements neither `Eq` nor `Hash`, but the generated
     `xds.core.v3` code derives both (`gen/xds/core/v3/xds.core.v3.rs:226`).

  **C1.3 is an open problem, not a task. Budget for it accordingly.** An earlier
  draft said "fix it in `buf.gen.yaml` (prost type attributes)". **That is not an
  available move**: `type_attribute` *adds* attributes and nothing in prost-build
  removes a derive. The derives are emitted by prost-build's own per-message
  derivability inference, computed against `prost-types`' `Any` — while
  `rumi/proto/Cargo.toml:22-24` deliberately aliases `pbjson-types` as
  `prost_types` so generated code gets serde impls. The alias is the root.

  Known options, none free, none yet tried:
  - `extern_path`-remap `.google.protobuf.Any` to a type that implements both
  - drop the pbjson alias — **this changes how protojson handles `Any`, which is
    now the authoring format (D-026), so it is not a local decision**
  - a post-processing step in `just gen` that strips the derives

  Steel-man before choosing, and record the answer in `DECISIONS.md`. This sits
  on the M4→M7 chain and is the most likely place to lose a day.

  **`rumi/proto/src/gen` is gitignored (`.gitignore:58`) — and that was a
  deliberate decision, not an oversight.** Commit `e36dd29` removed 14 previously
  tracked files with the reason *"Generated proto files are deterministic output
  of `just gen`. Removing 46K lines of generated code from tracking keeps PRs
  reviewable."* Note that the same commit added `.gitattributes:2-4`
  `linguist-generated=true`, which already collapses those diffs for tracked
  files — so the stated reason is self-defeating, but you must argue with it
  rather than around it. It contradicts `protocol-mastery/SKILL.md:53`, which
  states this project commits generated code. Reverse it explicitly in
  `DECISIONS.md`; do not just delete the line.

  **It is a three-language decision.** `.gitignore:58-60` also ignores
  `puma/proto/src/gen/` and `bumi/proto/src/gen/`. Both exist on disk (7 and 6
  files), both untracked, nothing imports either. M5's gate demands `buf generate`
  be the source of config types **in all three languages**, and the "`just gen`
  produces no diff" check is vacuous over untracked directories.

  Then commit all three `gen/` trees and wire the dep generation into `just gen`
  so it is reproducible.

- **C2. Prove the conversion works.** `rumi/proto/src/convert.rs` already
  contains eight end-to-end tests (proto → convert → load → evaluate). Run them.
  Add `-p rumi-proto` to the CI test job so a crate that does not compile can
  never again be marked done.

- **C3. Measure the fixture migration.** Push all 27 fixtures in `spec/tests/`
  through the protojson path. This tells you the true size of the migration,
  which nobody currently knows because it has never been carried to completion.

- **C4. Settle the `matcher:` fixture dialect.** The one question D-026 left
  open. 14 of 27 fixtures load through `rumi/ext/test/src/fixture.rs`, which
  builds `Matcher` values directly and never touches the config path — so
  protojson does not automatically apply to them. Either migrate them (and delete
  that loader, with its own `MatcherConfig`/`PredicateConfig`/`OnMatchConfig`
  types) or keep them as explicit native-construction tests with a comment saying
  why. **Whichever you pick, reconcile it with A5 and with M5's gate before
  writing code** — see the note under A5.

  The authoring-surface question that used to live here is **answered**: D-026,
  and restated in §3 and SF0. Do not re-open it.

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
current form.** The review is at `reference/security-review-2026-08-16.md`.
Each fix below has a
falsifying test **in that file**, several already run and passing; commit them as
regression fixtures rather than re-deriving them.

**Numbering map. The review and this plan use different schemes — check which
document you are in before resolving any bare `F6` or `S3`.**

| This plan | Review | Subject |
|---|---|---|
| SEC1 | F-01 | bumi regex compile bomb |
| SEC2 | F-02 | limits in the loader, not the constructor |
| SEC3 | F-04 | `session_id` accepted then discarded |
| 0.1.1 list | F-03 | aggregate regex budget |
| 0.1.1 list | F-05 | empty rule list compiles to a catch-all |
| 0.1.1 list | F-06 | compilers never call `validate()` |
| E7 | S-1 | `publish = false` dependency edges |
| E4 | S-2 | `rumi-http` default features |
| 0.1.1 list | S-3 | `serde_yaml` archived |
| 0.1.1 list | S-4 | CI actions unpinned |
| E5 | S-5 | package-root LICENSE files, missing http README |
| **below** | L-1..L-4 | **were dropped from this plan entirely; restored below** |

§4's `F1..F21` are a third scheme and correspond to none of the above.

**The review contains at least one error, so verify before acting on it.** F-04
states there is no `SessionIdInput`. There is —
`rumi/core/src/claude/inputs.rs:48`, registered at `claude/mod.rs:72`, present at
the reviewed commit. The defect is real (`claude/config.rs` has no `session_id`
field) but SEC3's fix is smaller than written: the input already exists.

**SEC1 — bumi compiles a regex bomb from 20 characters. BLOCKING.**
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

**SEC2 — every resource limit is enforced in the config loader, not the
constructor it protects. BLOCKING, and it is the architectural root of SEC1.**
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

**SEC3 — `session_id` is accepted, counted as a constraint, then silently
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
- `validate()` is never called by either domain compiler (**review F-06**, not
  §4's F6, which is `ignore_case` — see the numbering map above). The crusts
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

**The review's L findings, which an earlier draft of this plan dropped entirely.**
They are restored here because §6 requires every review finding to be fixed or
explicitly accepted, and a finding that is not written down cannot be either.

- **L-1.** The playground's `configToGraph`
  (`docs/experience/src/.../graph/config-to-graph.ts:42`) calls
  `parseMatcherConfig` without `loadMatcher`, so it enforces neither
  `MAX_FIELD_MATCHERS` nor `MAX_DEPTH` — another SEC2 instance. ELK runs on the
  main thread; ~5,000 field matchers locks the tab. **Self-inflicted only,
  because there is no share-link and no URL-seeded state.** If a rule builder
  with export or a share feature ships, this becomes remotely triggerable —
  re-audit then. Recorded in D-026.
- **L-2.** `StringMatcher::regex_ignore_case` (`rumi/core/src/input_matcher.rs:373`)
  builds `format!("(?i){pattern}")`; a pattern starting `(?-i)` neutralizes it.
  Config-author-controlled, no privilege crossing. Noted so it is not mistaken
  for a guarantee — which matters more now that SF1 will assert `ignore_case`
  semantics.
- **L-3.** `trace_string_match` (`claude/compiler.rs:236-240`) uses
  `.is_ok_and(...)`, so an **invalid regex traces as "did not match"** while
  `compile()` returns `Err`. The CLI's `--trace` is not affected — it calls
  `evaluate_with_trace` on an already-compiled matcher (`cli/src/main.rs:206`) —
  but `HookMatch::trace()` is public and ships as `HookMatcher.trace()` in the
  PyO3 wheel (`crusts/python/src/matcher.rs:222`). Trace is what an operator
  reaches for to answer "why didn't my deny rule fire?", and it gives a different
  answer than the compiler.
- **L-4.** `puma/src/xuma/_string_matchers.py:144` catches only `re2.error`; a
  non-`str` pattern escapes as a raw `TypeError`, outside the `MatcherError`
  contract.

**Ship criteria, from the review's own verdict.** DO NOT SHIP becomes SHIP WITH
FIXES when F-01, F-02, F-04, S-1 and S-2 are closed with their falsifying tests
committed, **and** a comment documents why deferring F-06 (the iterative rewrite)
is safe. That last clause is easy to lose; it is the only part of the verdict
this plan does not otherwise cover.

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

**Done when:** SEC1, SEC2 and SEC3 are fixed with their falsifying tests committed
as regression fixtures; every item under "controls that are correct" has a test or
a comment naming what it protects; and **every L finding is either fixed or
explicitly accepted in `DECISIONS.md`** — §6 requires the whole review triaged,
not just the blocking part.

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

- **E3. Remove `rumi-test` from both crusts** (not in the review — this is the plan author's own observation; it was mis-cited as S-5). They declare it with
  `features = ["fixtures"]` — a YAML fixture loader inside a published wheel and
  npm package. They want `rumi-kv`.

- **E4. Split `ext-proc` and set `default = []` on `rumi-http`** (S-2, and it
  subsumes the `rumi-http` default-features change; the *current* B2 is a
  prelude task and is unrelated). The feature currently conflates two unrelated
  things:
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

- **E5. Package hygiene for every artifact** (review S-5; there is no S-6). Each package root needs its
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

- **CI1.** Add crust jobs. Two of five implementations are never built (§4 F5).
- **CI2.** Add `-p rumi-proto` to the test job, and `just test-full` to CI (see §4 F19).
- **CI3.** A job that runs the README's literal config through every shipped
  runtime and asserts identical results. The README is the correctness claim;
  make CI enforce it.
- **CI4.** The doc-command smoke test from A4.
- **CI5.** `just gen` produces no diff — for all three `gen/` trees, which requires C1 to have tracked them.

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

**Done when:** every item above is either shipped or explicitly deferred with a
reason. Nothing here blocks 0.1.0; H7's five-minute re-review is the closing
check, and it is only meaningful against post-migration docs.

(The previous wording here was a verbatim copy of Phase H's done-when, describing
`just doctor` and `CONTRIBUTING.md` — H1 and H2, neither of which is in this
phase.)

---

### Phase G — Release

Only after **A, B, SF, C, S, E, F, K, and H1–H3** are on green PRs and a human
has merged them. (An earlier draft said "A–F", which silently omitted SF, K —
whose own header says release-blocking — and the H items M2 requires.)

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

Tarmac, concretely, for this project. **D-026 makes the first item precise for
the first time** — "one config schema" used to be arguable, because a terse
authoring dialect lowering into proto could be described either way. It cannot
now.

- [ ] **One config schema — protojson, generated from proto, in all three
      languages.** No hand-written config types, no terse dialect, no aliases.
      `grep -rn "MatcherConfig" rumi puma bumi` returns only generated code —
      currently **221 hits**, and see C4 for the one legitimate remaining
      question (the `matcher:` fixture loader)
- [ ] All three `gen/` trees are **tracked**, and `just gen` produces no diff in
      CI (F20 — the check is vacuous while they are gitignored)
- [ ] `rumi-proto` compiles, is in the CI test job, and `just test-full` is green
      (F1, F19)
- [ ] Every doc code sample carries a `run` / `compile` / `cli` / `future`
      marker, CI enforces each class, and the milestone state the `future` check
      reads from is a real file
- [ ] Every roadmap ✅ corresponds to something CI runs
- [ ] No path where a config loads clean and returns a wrong answer
- [ ] The README example compiles unmodified outside the workspace; at M7,
      `cargo add rumi-core` does too
- [ ] `cargo publish --dry-run` passes for every published crate, in dependency
      order, with no path patching
- [ ] All five implementations agree on every fixture, including the SF ones,
      proven by CI
- [ ] Every §4 finding has become a test, and its row here is struck
- [ ] Every asymmetry stated plainly in the docs — Claude is Rust-only until it
      is not
- [ ] `just ci` green, including `cargo audit`
- [ ] **The whole** security review triaged — F-01..F-06, S-1..S-5 **and
      L-1..L-4** — each fixed or explicitly accepted in `DECISIONS.md`, plus the
      F-06 deferral comment the review's ship criteria require

## 7. As you go

Append to `DECISIONS.md` whenever you make a call a future reader would
otherwise have to reconstruct. Follow the existing format: what was decided,
why, and what would justify revisiting. Newest first.

If you find something this plan gets wrong, fix the plan in the same PR as the
work. A stale plan becomes the next thing that lies.
