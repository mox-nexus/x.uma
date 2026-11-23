---
name: maintainer
description: >
  How to work in the x.uma repository: what is where, where new code goes, the order work
  happens in (decision, fixture, code, check), the commands that build and verify it, and the
  conventions for decisions, commits and pull requests. Load it at the start of any x.uma task,
  before adding a crate, domain, input, action, fixture, doc page or CI check, and before
  claiming anything is done. design-mastery covers what to expose; rust-mastery covers how to
  write the Rust; protocol-mastery covers proto and xDS.
---

# x.uma maintainer

x.uma is a matcher engine. It implements the xDS Unified Matcher API in one Rust engine,
`rumi`, with two bindings published as `xuma-crust`: PyO3 for Python, wasm-bindgen for
JavaScript. Domains (HTTP, Claude Code hooks, plain key-value maps) plug in at
`TypedExtensionConfig`.

It is a matcher engine, not a policy engine. It selects an action; it does not decide what the
action means. That line is a standing decision, and most scope questions are answered by it.

This skill is how to work here without breaking what the project has learned. The project
records its own state in three places, and you should read them rather than trust a summary,
including this one:

- **`PLAN.md`**: where the work stands and what is open. Read it top to bottom at the start of
  a session, and confirm your understanding with the maintainer before acting.
- **`DECISIONS.md`**: every decision of record, as D-entries. Read the relevant ones before
  revisiting anything they settle.
- **`CLAUDE.md`**: the architecture and the standing constraints.

---

## 1. The map

| Path | What it is |
|---|---|
| `proto/xuma/<area>/v1/` | The schema users write configs against. The contract. |
| `spec/tests/07_protojson/` | The conformance suite. Runs in the engine and both bindings. |
| `spec/tests/05_http/` | Gateway API route matches through the HTTP domain compiler. Runs in rumi. |
| `rumi/core/` | The engine: package `rumi-core`, library name `rumi`. The claude domain is its `claude` feature. |
| `rumi/proto/` | Generated proto types (`src/gen/`, committed) plus config loading: `convert.rs`, `any_resolver.rs`, `protojson.rs`. |
| `rumi/ext/kv/`, `rumi/ext/http/` | Domain crates. |
| `rumi/ext/test/` | The conformance harness. Not published. |
| `rumi/cli/` | The `rumi` binary, including Claude Code hook mode. |
| `rumi/crusts/python/`, `rumi/crusts/wasm/` | The two bindings. |
| `rumi/docs-tests/` | Compiles the Rust code blocks in `docs/content/`. |
| `docs/content/`, `docs/experience/` | The docs as plain Markdown, and the SvelteKit site that renders them, including the playground. |
| `bench/` | Committed benchmark artifacts and `RESULTS.md`. The benchmarks themselves live with the code they measure: `rumi/core/benches/` and the crusts' test folders. |
| `scripts/` | The repository's own checks, called from the `justfile`. |
| `reference/` | Evidence that `PLAN.md` and `DECISIONS.md` cite, such as the security review. Tracked. |
| `scratch/` | Session notes and working documents. Not tracked. |

**What will be published** (nothing is yet):
- **Five crates on crates.io,** in dependency order: `rumi-core`, `rumi-proto`, `rumi-kv`,
  `rumi-http`, `rumi-cli`. `scripts/check-publishable.mjs` derives the list from `cargo
  metadata` and fails if the release workflow disagrees.
- **`xuma-crust`** on PyPI and on npm.
- **Not published:** the crusts as Cargo crates (`publish = false`) and the test harness.

---

## 2. Where new code goes

| You are adding | It goes in | Because |
|---|---|---|
| Evaluation logic, a matcher kind, a limit | `rumi/core/` | Domain-agnostic, shared by everything |
| A new input or action for an existing domain | that domain's crate, plus its message in `proto/xuma/<area>/v1/` | Domains own their vocabulary |
| A new domain | a new `proto/xuma/<area>/v1/` package and a domain crate (or a core feature, if it brings no heavy dependency) | New domains must land without touching the engine's interfaces |
| Config loading or `Any` handling | `rumi/proto/` | One load path for files and control planes |
| Anything a binding needs to know | the engine, exposed to both bindings | Bindings interpret nothing |
| A behaviour test | a fixture in `spec/tests/07_protojson/` first | The suite is the source of truth for all three implementations |
| A rule about the repository | a check in `scripts/`, called from `just ci` | A rule nothing checks is a wish |

**The filter for core:** does it know about a specific domain? Then it is not core.

---

## 3. The order work happens in

### 1. Decide, in writing

**Principle.** Before changing anything that freezes at publish or that a D-entry covers, write
or update the D-entry. New vocabulary is defined in `DECISIONS.md` before it appears in code,
docs or skills.

**Why.** A decision not written down is not made; it gets re-argued every session. And a term
that appears in code before it is defined ends up meaning different things in different places.

**What it helps you catch.** A rename, a new result type, a new CLI verb or a new input whose
meaning lives only in a commit message.

**Why it helps.** The maintainer rules on a short written proposal instead of reverse-engineering
intent from a diff.

### 2. Write the fixture first

**Principle.** Behaviour starts as a conformance fixture that fails. Every fixture carries an
`implementations:` ledger naming who must pass it, and every runner enforces the ledger in
**both** directions: a fixture that omits an implementation must also fail for it.

**Why.** The suite is what keeps the engine and both bindings in agreement. The two-way ledger
stops a stale exception from hiding a finished migration, and stops a quietly fixed
implementation from going unnoticed. When a fixture and a security property disagree, the
fixture is wrong: this suite once required every implementation to treat an empty route list as
match-everything, so each was conformant by failing open.

**What it helps you catch.** A feature with no fixture, and a fixture that encodes a fail-open.

**Why it helps.** "Does it work in all three?" has one mechanical answer.

### 3. Implement, then check

**Principle.** Make the fixture pass in the engine, then in both bindings. Run the checks below,
and fix what they find. Never skip or weaken a check to get green.

**Why.** Every check in this repository exists because its absence once let a false claim ship.

### 4. Claim only what a check proves

**Principle.** Something is done when CI runs a check that would fail if it regressed. Until
then it is unverified, whatever it looks like locally.

**Why.** One phase of this project was marked done for months while one of its crates had never
compiled, because nothing built it. Another claim ("nothing compiles the docs' code samples")
was built on a search that had silently errored. A claim that something does not exist is the
easiest kind to get wrong.

**What it helps you catch.**
- Your own "done" with no CI behind it.
- A negative claim from a search that had no control.
- A self-correction that has not been independently re-checked. In this repository a
  correction goes to an independent skeptic (the `mrwolf` agent), who must verify it by
  executing something before it reaches the maintainer.

**Why it helps.** What you report becomes the foundation the next session builds on. False
foundations compound.

---

## 4. Commands

Everything goes through `just`. Run `just --list` for the full set; these are the ones you will
use.

| Command | What it does |
|---|---|
| `just doctor` | Checks every tool the repository needs is installed. Run it first on a new machine. |
| `just ci` | Everything CI runs, including both bindings' test suites, the codegen drift check and the npm package check. It needs every tool `just doctor` lists. Pipe it with `set -o pipefail`, or a failure hides behind the pipe. |
| `just build`, `just test`, `just lint` | The Rust workspace's default members. These exclude the crusts. |
| `just lint-strict` | Clippy as CI enforces it: all targets, warnings denied. |
| `just fmt` | Format. Before committing: `cargo clippy --fix --allow-dirty -- -W clippy::pedantic`, then `cargo fmt`, in that order. |
| `just crust-check` | Builds and tests both bindings, and runs each binding benchmark once. |
| `just test-protojson`, `just test-http-conformance` | The two conformance suites. |
| `just test-fixture-coverage` | Checks that the fixture ledger is current in both directions. |
| `just gen`, `just gen-check` | Regenerates the proto code; `gen-check` also fails if the committed tree differs, as CI does. |
| `just lint-proto`, `just breaking` | `buf lint`, and `buf breaking` against `main`. |
| `just docs-check`, `just docs-samples`, `just docs-commands`, `just docs-links` | The docs: every code block runs or compiles, every CLI command exists, every link resolves. |
| `just bench-smoke` | Runs the core Rust benchmarks once, so a broken one fails. It does not judge speed. The Python and wasm benchmarks get the same single run in `just crust-check`. |
| `just bench-artifact`, `just bench-compare` | The measured protocol under D-066. Every performance number in tracked markdown cites an artifact. |
| `just publishable`, `just publish-dry` | Publication checks. |
| `just verify-clean-clone` | Builds from a fresh clone, to catch files that exist only on this machine. |

---

## 5. Conventions

- **Comments** say why, never what. A why-comment is one sentence and may cite a D-entry.
- **Deleting files:** `git rm` for tracked files. For untracked ones, use `rip`, never `rm`.
- **Branches and pull requests:** work on a branch, merge to `main` through a pull request, and
  never force-push. Do not push without the maintainer's go-ahead for that session.
  - Merge with a merge commit when the branch's commits carry D-entry ids, or when benchmark
    artifacts are named after their hashes. Squashing or rebasing would lose those
    references.
- **Commit messages** name the decision they implement, as in `core(D-069): …`, and say why,
  not just what.
- **Status claims** in `CLAUDE.md` and `PLAN.md` use the project's legend:
  - ✅ means a CI check would fail on regression;
  - ⚠️ means the work exists unchecked;
  - "runs, not guarded" means CI executes it but cannot see it get worse.
- **Performance claims:** never write a number from memory. Cite a committed benchmark
  artifact, or say nothing.
- **The pre-release state:** nothing is published yet, and package names are chosen, not
  reserved. README and getting-started pages say so until a release lands.

---

## 6. The harness

| Part | Use it for |
|---|---|
| `design-mastery` skill | Any change to something that freezes at publish, or that adds an extension point. |
| `rust-mastery` skill | Writing Rust in the engine, the CLI or the bindings. |
| `protocol-mastery` skill | Proto, buf, `Any`, protojson, and Envoy's matcher semantics. |
| `constraint-auditor` agent, `/audit` | Reviewing a change against the protocol obligations before merge. |
| `/validate` | Running the full check suite and reporting it. |
| `/check-breaking` | Checking a change for breaking API or schema changes. |

---

## 7. Reference implementations

Envoy is the original, and rumi follows it unless a D-entry says otherwise:
- the protos: `~/oss/reference/cncf-xds/xds/type/matcher/v3/`;
- the C++ implementation: `~/oss/reference/envoy/source/common/matcher/` and
  `~/oss/reference/envoy/envoy/matcher/`.

When a behaviour is unclear, read Envoy's code before deciding.
