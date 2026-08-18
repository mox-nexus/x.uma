# Decisions

Decisions of record for x.uma. Newest first. Each entry states what was decided,
why, and what would justify revisiting it.

Format is deliberately light. If a decision needs a page of argument, it belongs
in `scratch/` and gets summarized here.

---

## 2026-08-18 · How Python and TypeScript read protojson

### D-039 · `CLAUDE.md`'s "Pure Python" is false for puma and true for bumi

`puma/uv.lock` lists **zero** `py3-none-any` wheels for `google-re2` — macOS,
manylinux and Windows wheels per Python version, nothing else. puma has not been
pure Python since RE2 replaced the `re` module, and a table saying otherwise is
the kind of claim this repo has spent four PRs removing.

bumi is different and the asymmetry matters: `re2js` is a pure-JavaScript port,
so bumi genuinely is pure TypeScript. "puma is already impure, so a dependency
is free" does not transfer.

What the word was standing in for is worth keeping under a name that can be
held: **puma and bumi carry no protobuf runtime and read protojson directly.**
That is the only boundary distinguishing them from `xuma-crust`, which is the
same engine, faster, already on PyPI and npm. Erase it and the two artifacts
have the same boundary — which makes them one artifact with two names.

**Revisit** by deleting puma and bumi, not by making them heavier. If the
dependency-light property is not worth holding, the packages are not either.

### D-038 · puma and bumi read protojson directly; no generated types in either

Measured 2026-08-18, both languages:

| | given `{"kye": "role"}` for `MapInput` |
|---|---|
| ts-proto `fromJSON` | returns `{key: ""}` — no error |
| betterproto `from_dict` | returns `MapInput(key='')` — no error |

Neither rejects an unknown field, so **neither can satisfy
`protojson_rejects_an_unknown_field`** — the fixture carrying the migration's
headline claim, that a typo'd field in a deny rule is a load error rather than a
rule that silently never fires.

Generated code that is lenient is *worse* than a hand-written reader, not
equivalent: a reviewer audits a hand-written parser and does not audit a file
headed `DO NOT EDIT`. Note also that both return an **empty key** — precisely
the fail-open D-035 rejects in Rust's constructor.

So the Python and TypeScript plugins are removed from `buf.gen.yaml`, and the
fifteen generated files that were tracked but imported by nothing — `betterproto`
absent from `pyproject.toml`, `@bufbuild/protobuf` absent from `package.json`,
and `bumi/tsconfig.json` never including the directory — are deleted. They were
the "unverified by construction" trap the plan opens with, sitting in the repo.

**The document has two regions, and the boundary is region-shaped, not
language-shaped.** D-034 already split them by making `pack` stop at `@type`:

- **the xDS tree** — schema owned upstream and frozen, ~12 structural types,
  destructured and discarded in one pass. Hand-walked in puma and bumi,
  mirroring `rumi-proto`'s `convert.rs`.
- **`Any` payloads** — schema owned by x.uma, growing with every domain, and
  where the security property lives. The contract is *an unknown field is a load
  error*, in all three implementations, satisfied by whatever means each
  language actually can.

Hand-writing the payload reader is safe for a reason that was already paid for:
`scripts/check-proto-field-types.mjs` (D-033) bounds payloads to `string`,
`bool`, `bytes` and `map<string, string>`. The proof that made D-034's round trip
an identity is the same proof that makes a hand-written payload reader adequate.

**The price of this decision, which is not optional.** Under generated types the
dependency on `proto/xuma/**` is an arrow the build can see. Here it is a
human's memory. That is a missing arrow, not a wrong one, and it has to be made
explicit or this is a rationalisation: a descriptor-driven check must assert
that for every message and field in `proto/xuma/**`, a fixture sets it *and* a
sibling misspells it and expects a load error. Without that, three
implementations agree only on what somebody remembered to fixture.

Worth recording what this decision does **not** claim. Codegen propagates
shapes, not semantics — and of the schema-crossing defects this repo actually
shipped, only F13 was a shape. F6, F2 and F14 were all semantic and generated
types would have caught none of them. Neither mechanism dominates; they cover
different halves.

**Revisit if** a generator appears that rejects unknown fields by default —
`@bufbuild/protobuf`'s strict `fromJson` is the near-term candidate and was not
measured here. The dependency-inversion objection to Google's `protobuf` for
Python stands separately: it would pull an ecosystem in for the whole document
to fix a property only the payload region needs.

## 2026-08-18 · One config vocabulary

### D-037 · There is no `proto` feature, because there is nothing left for it to select

`rumi-kv` and `rumi-http` each carried two `IntoDataInput::Config` types for the
same input, chosen by `#[cfg(all(feature = "registry", not(feature = "proto")))]`.
Enabling `proto` did not add; it deleted two public config types and replaced
eight impls. One replacement turned a required `name` into `unwrap_or_default()`,
so `config: {}` went from a load error to `HeaderInput::new("")` → `None` →
predicate false. A header deny rule stopped firing and nothing reported it.

The generated proto types are now the only config types, so `registry` pulls
`rumi-proto` directly and the `proto` feature is gone. Deleting the feature
rather than reconciling the two sides is the point: a `not(feature = …)` cfg on
a trait's associated type is only writable when there are two vocabularies to
choose between. `check-features.sh` no longer lists a `proto` combination, and
`cargo test --all-features` is green for the first time.

`rumi-test`'s `proto` feature went too. It linked `rumi-proto` and no source
file under `ext/test` ever referenced it.

**Revisit if** a consumer needs config types without a protobuf dependency. They
would be asking for a second vocabulary, which is the thing this removes.

### D-036 · Type URLs name what the schema actually says

Three type URLs disagreed with their own messages, and all three freeze at first
publish:

| Registered | Schema said | Now |
|---|---|---|
| `xuma.test.v1.StringInput` | one field, `value`, used by the code as a lookup key | `xuma.kv.v1.MapInput` with `key` |
| `xuma.http.v1.AuthorityInput` | message named `HostInput` | `AuthorityInput` |
| `xuma.test.v1.*` | a package named for a test suite | `xuma.kv.v1.*` |

The first is F13, and it was not a naming wart: with the hand-written config the
key was `key`, with the proto config it was `value`, so the same crate read a
different config field depending on a feature. Both descriptions were true, in
different builds.

The key-value domain is a peer of HTTP and the CLI's default — it was never
"test", and `MapInput` was already the message that meant what the code did.

**Revisit** never, for these three. Type URLs are the config schema and they
freeze at 0.1.0. This was the last moment.

## 2026-08-18 · The protojson load path

### D-035 · Invariants that hold a limit are enforced by the type that holds the resource — including width, and including identity

D-029 said limits belong to the type holding the resource, and then only the
pattern-length limits actually moved. `MAX_FIELD_MATCHERS` and
`MAX_PREDICATES_PER_COMPOUND` stayed in `Registry::load_*`, so a matcher built
by a domain compiler — the path `CLAUDE.md` calls the door handle — carried
neither. Both are now checked in `Matcher::validate()` and `Predicate::validate()`,
and both domain compilers call `validate()` before returning. A hundred-thousand
-child `and` is one level deep, so depth never covered it.

The same rule extends to identity. A header name, a query parameter name, a tool
argument name and a map key all say *where* to read a value. Empty, they read
nothing, the predicate is false, and a deny rule silently stops denying. proto3
cannot express "required", so the schema cannot carry this — the constructor
must. `HeaderInput::new`, `QueryParamInput::new`, `ArgumentInput::new` and
`rumi_kv::StringInput::new` now return `Result` and reject an empty identifier.

Enforcing it in the config loader instead would have covered one of five routes.
The constructor covers the config path, both compilers, both crusts and direct
construction at once, and a fixture becomes a regression test rather than the
enforcement.

**Revisit if** a legitimate caller needs an empty identifier to mean "any". It
does not today: `ctx.header("")` returns `None`, not every header.

### D-034 · Canonical protojson is packed into `Any` before deserialization, by the registry that already unpacks it

Measured 2026-08-18: `pbjson-types`'s `Any` does not implement protojson's
`@type` expansion. A canonical document fails with ``unknown field `@type` ``.
It cannot implement it — expanding an `Any` needs the payload's schema at
deserialization time, and static codegen has no descriptor pool to look it up
in. D-026 chose protojson as the authoring surface, so the gap is ours to close.

`AnyResolver::pack` closes it: walk the document, encode each `@type` object to
binary, replace it with the `{typeUrl, value}` form the generated impls read.
Both directions are installed by the same `register::<T>()` call, so a type that
can be read but not written is unrepresentable rather than merely unlikely.

**The round trip through binary is deliberate.** It costs a JSON→bytes→JSON hop
at load time and buys the file path and the control-plane path meeting at the
same proto value, sharing one conversion walk. The alternative — a second walk
reading JSON straight into config types — is two implementations of one
semantics, free to disagree. This repo has already shipped that bug once.

**The walk stops at `@type` and requires a string there.** Neither is defensive
tidiness; both are correctness. Descending into payload bodies would corrupt a
rule whose `NamedAction.metadata` used `@type` as a key, and treating a
non-string `@type` as a type URL would break a `MatcherTree` match key spelled
`@type`. Those are the schema's only two user-controlled key positions, and both
are pinned by tests.

**Revisit if** an `xuma.*` message gains an `Any` field — nested expansion is
unsupported, and would then need a descriptor-directed walk. It fails closed
until then.

### D-033 · `proto/xuma/**` field types are restricted to what round-trips as the identity

`scripts/check-proto-field-types.mjs` fails the build on any field outside
`{string, bool, bytes, map<string, string>, message in the same file}`.

The JSON→bytes→JSON hop in D-034 is lossless *because the surface is small*.
`int64` round-trips as a JSON string, enums as names, floats turn NaN into a
string, and `optional` carries explicit presence the hop erases. None of those
is unusable — each just makes the round trip a transform rather than an
identity, at which point the reasoning above has to be redone.

Proof by inspection decays silently on the first field somebody adds. This
converts it to proof by construction. The check was falsified against six banned
types before being wired in; it caught five, and the miss — `optional` slipping
past a skip pattern written as `option` — is why it was tested at all.

**Revisit** by deleting a ban and adding the round-trip fixture that replaces
the proof for that type.

## 2026-08-17 · Reproducibility

### D-031 · Every lockfile is committed, and the toolchain is pinned

D-016 left the lockfile question open, and the repo answered it inconsistently
in the meantime: `.gitignore` excluded `Cargo.lock` and `bun.lock` while
`bumi/bun.lock`, `puma/uv.lock`, `rumi/crusts/python/uv.lock` and
`rumi/crusts/wasm/bun.lock` were all tracked. A rule that half the repo ignores
is not a rule.

All of them are committed now. A workspace that ships a binary commits its
lockfile, and reproducible CI wants the rest.

Toolchains are pinned alongside them: `rust-toolchain.toml` (1.95.0),
`.python-version` (3.12) and `.bun-version` (1.3.12). `stable` is a moving
target, and an unpinned toolchain is how `just ci` passes on one machine and
fails on another for reasons unrelated to the change under test — which
happened twice in one day with unpinned GitHub Actions.

**Pinning has a cost, found immediately.** `rust-toolchain.toml` applies to
everything cargo does in the repo, including installing CI tooling — the audit
job failed because `cargo-audit`'s transitive `kstring` requires rustc 1.96 and
the pin forced 1.95. That job now sets `RUSTUP_TOOLCHAIN=stable`: the pin exists
to fix what x.uma is *built* with, not to constrain what a tool that only reads
`Cargo.lock` can be installed with.

Committing the lockfile also removed a step. Both `just audit` and the CI job
used to run `cargo generate-lockfile` first, which meant they audited a freshly
resolved graph rather than the one that ships.

**No MSRV is declared.** `rust-version` would be a claim nothing checks, since
CI builds on one toolchain and no job tests an older one. Declaring one without
that job is exactly the pattern this plan exists to correct. It can be added the
day a job verifies it.

### D-032 · `just doctor` and `just verify-clean-clone`

Two checks, each for a failure mode `just ci` structurally cannot see.

**`just doctor`** lists every required tool and exits non-zero if one is
missing. A fresh clone needed nine tools and nothing said so. Two defects in one
week traced to it: the wasm crust had never been built on the machine that
claimed it worked, and `uv run maturin` failed in CI while passing locally
because maturin was on one PATH and not the other.

**`just verify-clean-clone`** builds from `git archive HEAD` — what a clone
actually receives — rather than the working tree. `just ci` runs against the
working directory, so a file that exists locally but is untracked or ignored is
invisible to it. That class has bitten this repo three times: seventeen files
hidden by a bare `lib/` gitignore pattern, a `playground` workspace entry
pointing at a directory deleted nine commits earlier, and generated proto code
that was ignored while a crate depended on it.

The middle one was found by this work rather than by reasoning about it: the
stale workspace entry only fails under `--frozen-lockfile`, so the repo had a
broken clean install for nine commits and every local `bun install` tolerated it.

**Not in `just ci`.** Both are slower than the gate should be, and
`verify-clean-clone` needs a commit to archive. They belong before opening a
pull request, which is what `CONTRIBUTING.md` says.

---

## 2026-08-17 · Security

### D-030 · Source comments do not cite `DECISIONS.md`

Nineteen comments across five files were written citing "see `DECISIONS.md`
D-029" before D-029 existed. That is the same defect as PLAN.md F22 — a citation
the reader cannot follow — committed inside the security fix that F22 was meant
to teach.

The deeper problem is not the missing entry. A comment saying *see D-029* couples
source to a document's numbering scheme, and this repo has already had that
numbering collide four ways (PLAN.md §4 findings, PLAN.md Phase F tasks, the
security review's own `F-01..F-06`, and Phase S tasks against the review's
`S-1..S-5`). A reader who follows such a pointer into a renumbered or
reorganised file learns nothing and trusts the next comment less.

**A comment carries its own reason.** "Limits live here because a loader-only
check let `HookMatch::compile` accept 8 MB" survives renumbering, file moves and
restructuring of the log. "See D-029" does not.

`DECISIONS.md` keeps its job — recording *why* a call was made, for a reader
asking that question. It is not an index that source code points into.

**What would overturn this:** nothing likely. If a decision needs more context
than a comment can hold, the comment still states the conclusion, and the log is
searchable by subject.

### D-029 · Resource limits belong to the type that holds the resource

The security review's F-02: the architectural root of two other findings, and it
reproduced independently in all three implementations, which is what made it
structural rather than an oversight.

Every declared limit was enforced in the config loader, so
`MAX_PATTERN_LENGTH`, `MAX_REGEX_PATTERN_LENGTH`, `MAX_FIELD_MATCHERS` and
`MAX_PREDICATES_PER_COMPOUND` were advisory on any path that was not the
JSON/YAML registry loader. Measured: `HookMatch::compile` accepted an **8 MB
pattern against an 8192-byte limit**, and puma's HTTP gateway accepted a
**40,960-byte regex against a 4,096 limit**.

The bypasses were not obscure. They were the domain compilers — the "door
handle" `CLAUDE.md` promotes as the way to use the engine — plus direct
construction and the docs-site playground's graph renderer.

**The rule: the type that holds the resource owns the limit on that resource.**
A limit enforced in a loader is advisory to every other caller. Limits now live
in the constructors — `StringMatchSpec::to_input_matcher`, puma's five
`__post_init__`, bumi's five constructors — and the loaders inherit them.
`Registry::check_pattern_length` was deleted rather than kept as a second copy.

Three consequences worth knowing:

- **The HTTP compiler could not report the error.** It never used
  `StringMatchSpec` at all, and swallowed invalid regexes into an exact match on
  the *pattern literal* — a comment called that "fail-safe"; it silently deletes
  the route. `compile_route_matches` and `HttpRouteMatchExt` now return `Result`,
  matching the Claude compiler, which always did. An API break, free at zero
  users and expensive after publish.
- **Both crusts hard-coded their own copies** of the limits. Three constants with
  no compile-time link is how they drift; they now import core's. The same move
  in puma and bumi put the limits in `_limits.py` / `limits.ts` so the matchers
  can enforce without importing the registry that imports them.
- **bumi needed a second, different limit.** `re2js` implements neither of C++
  RE2's compile-time guards, and pattern *length* is the wrong axis — cost tracks
  compiled program size. Measured through bumi's own `RegexMatcher`:
  `((a{100}){100}){100}`, twenty characters, **282 ms and 286 MB**.
  `regex-budget.ts` bounds the product of nested repetition counts at 1000, which
  is C++ RE2's own `kMaxRepeat` — matched rather than invented, so anything
  accepted here upstream RE2 would also accept.

**A near-miss, recorded because the method matters more than the result.** The
first probe reported the bomb no longer reproduced. It ran from `/tmp` and
resolved a different `re2js` than bumi's source does. Run inside `bumi`, it
reproduced at the review's numbers. A false negative on a blocking finding is
worse than no check at all, and the whole difference was module resolution:
reproduce inside the package under test, not next to it.

**One correction to the review itself.** Its F-04 states there is no
`SessionIdInput`. There is, and there was at the reviewed commit. The
`session_id` defect was real — both crusts accepted the field, counted it toward
the guard against accidental catch-alls, and then dropped it because core's
`HookMatch` had no such field — but the fix was one field and one compiler
branch, not a new input.

**What would overturn this:** nothing about the rule. If a limit genuinely cannot
be evaluated at construction — an aggregate budget spanning many matchers, like
the total regex cost deferred to 0.1.1 — it belongs to whatever type owns the
aggregate, not back in the loader.

---

## 2026-08-17 · Codegen

### D-028 · Codegen plugins are pinned, and generated code is committed

`rumi-proto` had never compiled. The cause was not what two reviews said it was,
so the wrong diagnosis is recorded here alongside the right one.

**What it actually was.** `buf.gen.yaml` referenced
`buf.build/community/neoeinstein-prost` with **no version**. That floating
reference had moved to v0.5.0+, whose prost-build infers
`#[derive(Eq, Hash)]` on messages — including `xds.core.v3.TypedExtensionConfig`,
which holds an `Option<Any>`. Neither `prost-types::Any` nor `pbjson-types::Any`
implements `Eq` or `Hash`, so the derive cannot compile against any available
`Any`. Meanwhile `rumi/proto/Cargo.toml` pins `prost = "0.13"`. **Codegen and
runtime had drifted apart, and nothing checked.**

Measured, generating the same protos with each version:

| plugin | layout | messages with `Eq, Hash` |
|---|---|---|
| `neoeinstein-prost:v0.4.0` | flat `gen/<package>.rs` | 0 — compiles |
| `neoeinstein-prost:v0.5.0` | nested directories | 4 — does not compile |

Pinned to `neoeinstein-prost:v0.4.0` + `neoeinstein-prost-serde:v0.3.0`, the
pair that matches the `prost = "0.13"` runtime. `rumi-proto` compiles and its 14
tests pass, three of them end-to-end proto → convert → load → evaluate.

**The wrong diagnosis, recorded so it is not re-derived.** Both an adversarial
plan review and this session's first writeup said the root was
`rumi/proto/Cargo.toml`'s aliasing of `pbjson-types` as `prost_types`, and that
dropping the alias was one of the options. Tested: removing the alias produces
**16 errors instead of 4** — the same `Eq`/`Hash` pair plus 12 serde failures.
The alias is load-bearing and orthogonal; it is why protojson works at all.
Keep it.

**Generated code is now committed**, for all three languages — but the reason
matters, because the first version of this entry gave the wrong one.

The argument that does *not* carry it: "the ignore made `just gen` produce no
diff, so drift was undetectable." True, but committing is not the only fix — CI
could regenerate fresh on every run and store nothing, and then drift is
impossible because there is nothing to drift from. `protocol-mastery` lists that
as the alternative (Envoy/Bazel). Nor did the ignore cause `rumi-proto` to go
uncompiled; the unpinned plugin and a CI job that never ran `-p rumi-proto` did,
and both are fixed independently.

**The argument that does carry it is publishing.** E1 publishes `rumi-proto` to
crates.io. `cargo package` follows git tracking, so gitignored generated code
does not reach the published crate and it would ship broken. Generating at build
time instead would force every consumer to have `buf` and network access. This is
why prost-ecosystem crates commit generated code, and it is the reason here.

This reverses `e36dd29`, which excluded generated code because it "keeps PRs
reviewable". That concern is real and is now handled two ways: the same commit
already added `.gitattributes linguist-generated=true`, which collapses these
diffs in review, and the scoping below cuts the tracked tree in half.

**Generate only what is used.** `buf generate buf.build/cncf/xds` without
`--path` pulls the entire module: ORCA load-reporting messages and services, xDS
and udpa annotation metadata, the legacy udpa namespace. That is **14 files and
~4,500 lines that `lib.rs` never includes and nothing compiles** — committed once,
in a matcher engine, before being caught. `just gen` now scopes to
`xds/core/v3`, `xds/type/v3`, `xds/type/matcher/v3`. If a fourth package is ever
needed, add it explicitly rather than removing the scoping.

Three consequences that bit immediately and are worth knowing:
- **ruff honours `.gitignore`.** Un-ignoring `puma/proto/src/gen/` made ruff
  start linting generated Python. Excluded explicitly in `pyproject.toml`; the
  ignore file had been doing lint configuration by accident.
- **`clean: true` plus a two-pass network generate can destroy the tree.** The
  second pass fetches from the BSR, which rate-limits; it failed after the first
  pass had already wiped, leaving 44 files deleted from the working tree. `just
  gen` now stages both passes and swaps only on success, and deletes nothing —
  outgoing trees are moved into the staging directory for the OS to reap.
- **The pin fixes the output layout too.** v0.4.0 emits flat files, v0.5.0 nested.
  `rumi/proto/src/lib.rs`'s `include!` paths follow the pin, and say so.

**Alternatives considered, so this is not a one-way door.**

*Generate in `build.rs` and track only the `.proto` sources* — the Envoy/Bazel
shape. Measured: 3,673 tracked lines (175 ours + 3,498 vendored xDS) against
10,990 generated, and the tracked content would be readable source. Rejected
because `prost-build` 0.13 does not bundle `protoc`; it needs one in `PATH` or a
~15 MB vendored-binary dependency, which puts a compiler toolchain between a user
and `cargo add rumi-core`. Envoy can do this because it is an *application* whose
consumers already run Bazel. We publish libraries to a registry. The published
prost ecosystem — `tonic-health`, `tonic-reflection`, `etcd-client` — lands the
same way we have, for the same reason.

*Depend on an existing published xDS crate* instead of generating the xDS half at
all. This would delete 8,478 of the 10,990 tracked lines and one whole codegen
pass, so it was investigated properly rather than left as a note:

| crate | downloads | prost | protojson / serde | tonic |
|---|---|---|---|---|
| `envoy-types` 0.7.6 | 1.4M | **0.14** | **no features at all** | 0.14, non-optional |
| `xds-api` 0.2.0 | 15.5k | **0.13** ✓ | **`pbjson` feature** ✓ | 0.12, **non-optional** |
| `xds-types` 0.1.0 | 232 | 0.13 | none | non-optional |
| `data-plane-api` 0.1.1 | 4.3k | last published 2022 | — | — |

`envoy-types` is the popular one and cannot serve the config path at all: no
serde or pbjson feature, so no protojson, which D-026 makes the whole point.

**`xds-api` is the near miss** — exactly our prost 0.13, exactly our pbjson 0.7,
and it ships a `pbjson` feature, which is evidence the approach is sound and that
someone else needed protojson out of xDS types. It fails on one axis: `tonic` is
a **non-optional** dependency. After C5 `rumi-proto` is on the critical path for
every consumer of `rumi-core`, so this would pull tonic, hyper, h2 and tokio into
a library that never opens a socket — the same 101-crates-against-7 problem E4
exists to fix, except unavoidable instead of merely a default, and in direct
conflict with D-027. It is also 17 months without a release.

**The trigger to revisit:** `xds-api` making `tonic` optional, or any crate
appearing with xDS matcher types, a pbjson feature, and no mandatory transport.
That is a small enough change upstream to be worth re-checking each release.

**What would overturn this:**
- `prost-build` bundling `protoc` again, or a zero-cost way to generate at
  consumer build time. Then `build.rs` wins on tracked-line count outright.
- A published upstream crate supplying the xDS types at a compatible prost
  version — see `xds-api` above.
- Wanting a prost 0.14 runtime. Then the plugin pins move with it, and whether
  `Any` gains `Eq`/`Hash` upstream must be **checked, not assumed**.

---

## 2026-08-17 · Schema freeze

### D-026 · protojson is the config format, not a wire format hidden behind a dialect

Config files stay — rules are defined in YAML or JSON files, both accepted, as
they are today. What changes is what is *inside* them. The xDS proto is the
schema for authoring and wire alike, and config files follow protobuf's
canonical JSON mapping. The terse dialect (`type: and`,
`value_match: { Exact: … }`, actions as bare strings) is retired, not preserved
as an authoring layer that lowers into proto.

**The reason is the `x`.** x.uma exists to implement the xDS Unified Matcher API
across languages, and `CLAUDE.md` already commits to xDS naming throughout "for
ecosystem compatibility". The config format was the one surface that did not
follow that — and it is the surface that matters most, because the config *is*
the interface. A bespoke dialect makes the premise false.

Supporting, not deciding: most configs will be generated by an agent reading the
schema, where a documented mechanical mapping beats a dialect learned from prose.
The repo already contains the failure that predicts — the playground's "Block
rm -rf" preset is written with `xuma.kv.v1.MapInput` keys and fails to load
under `rumi run claude`, an authoring mistake against a dialect with no schema
behind it.

**Verbosity is accepted.** A compound rule goes from ~11 lines to ~27, with
`matcherList` / `singlePredicate` wrappers, camelCase, `@type` URLs, and actions
promoted to `TypedExtensionConfig`. Envoy users hand-write this daily. Where
hand-authoring ergonomics matter the answer is tooling — a rule builder or graph
export from the playground — not a second schema. That tooling is post-release
and is not a prerequisite for this decision.

**What this rejects:** the wire-only option recorded as the maintainer's lean in
`PLAN.md` SF0. Wire-only costs two loaders and a second schema x.uma would own
and have to version, and it buys terseness for an audience that turns out to be
mostly machines. It also leaves M5's gate ("`grep -rn MatcherConfig` returns only
generated code") satisfiable by renaming rather than retiring.

Consequences that follow without further debate:

- Type URLs carry the `type.googleapis.com/` prefix, because protojson requires
  it in `@type`. The code registers bare names; that is now a defect, not a style
  choice. This settles SF8's second bullet.
- `xuma.kv.v1.MapInput.value` being used as a lookup key (`PLAN.md` F13) is
  release-blocking rather than a wart. Under protojson an author reads `value`
  from the proto and gets key-lookup semantics — a silent wrong answer.
- `rumi --skill` and `rumi info --verbose` move onto the critical path. If the
  schema is the authoring interface it must be machine-legible at authoring time.
- The PascalCase `Exact` beside lowercase `type:` (`PLAN.md` H11) is deleted by
  construction rather than documented.

**Still open, to be settled in Phase C:** the 14 fixtures using the `matcher:`
dialect load through `rumi/ext/test/src/fixture.rs`, which builds matchers
directly and never touches the config path. Whether they migrate to protojson or
stay as native-construction tests is a separate, smaller call.

**Security note for the tooling.** The docs-site playground is currently safe
because there is no URL-seeded state anywhere in it — `configToGraph` calls
`parseMatcherConfig` without `loadMatcher` and therefore enforces no resource
limits, but nothing remote can reach it. A rule builder with export is one step
from share-by-link, which makes that path remotely triggerable. Re-audit before
shipping any share feature.

**What would overturn this:** nothing about verbosity. Only a demonstrated rule
that protojson cannot express and the terse dialect could.

### D-027 · No async in core, ever

Declared structural in commit `8a5f996` and in `PLAN.md` SF0, but never recorded
here, which §7 requires. Recording it now.

x.uma is embedded and does not speak xDS itself. The host owns the transport, the
subscription and ECDS; x.uma owns only the step from config to matcher. There is
nothing to await when you never open a socket, so evaluation stays synchronous.

**One correction to the evidence as written in SF0.** SF0 says "no tonic, no
`DiscoveryRequest`, no subscription anywhere in the tree". The last two hold; the
first does not — `cargo tree -p rumi-http` shows `tonic v0.14.6` under default
features, pulled in by `envoy-grpc-ext-proc`. The accurate claim is: no client we
wrote, and no subscription in our own source. That transitive tonic is precisely
why `PLAN.md` E4 sets `default = []` on `rumi-http`.

**What would overturn this:** x.uma growing its own xDS client, which would make
it a control-plane consumer rather than an embedded library.

---

## 2026-08-12 · Playground folded into the docs app

### D-025 · Bundle: measured, reverted, recorded

A docs page ships ~260 kB gzipped, most of it Shiki. Switching to `shiki/core`
with static theme and grammar imports looked like the obvious fix and measured
**worse**: 260 kB to 306 kB. The `shiki` bundle lazy-loads grammars by dynamic
import, so pinning them statically made them eager.

Reverted, with the number written into the file so nobody repeats it.

The real cost is architectural. `svelte-exmarkdown` renders Markdown at runtime,
so the highlighter reaches the browser even though every page is prerendered and
the code is already highlighted in the shipped HTML. Fixing it means rendering
Markdown to HTML at build time and hydrating only the `<matcher>` islands. Worth
doing, and not worth guessing at bundler flags to avoid.

For scale: the landing page is 92 kB gzipped, a docs page 260 kB, the playground
route 670 kB. The playground earns its weight (CodeMirror, roughjs, elkjs, the
engine) and only loads when visited.

### D-024 · The playground is a route, not a package

It was a second SvelteKit app with one route, its own adapter, vite config,
package manifest, base path, and four `just` targets. Nothing justified the
separation: same dependency on `xuma`, same deploy, same audience, and the docs
already linked to it.

It is now `/playground` inside `docs/experience/`. The URL is unchanged.

What this deleted rather than maintained: one `svelte.config.js`, one
`vite.config.ts`, one `package.json`, one `tsconfig.json`, one base path, four
`just` targets, one CI job, one deploy assembly step, and the Catppuccin palette
the playground was still using.

The base-path guard from D-023 was written mostly to protect the playground,
because an SPA fallback document forces absolute asset URLs. As a prerendered
route it emits relative ones, so the built site is now **421 relative references
and zero absolute**. The bug class is gone by construction rather than policed.
The guard stays: it is cheap, and it fails loudly if the output shape ever
regresses.

Route-level code splitting means the playground's weight does not reach readers
who never open it.

---

## 2026-08-12 · Docs site shipped, mdBook retired

### D-023 · Base-path correctness is enforced, not documented

The failure this prevents is invisible in the artifact. SvelteKit emits absolute
asset URLs for SPA-fallback documents; built without the right `paths.base` the
site loads fine locally and serves a blank page with `/_app/*` 404s once deployed
under a prefix. Nothing looks wrong until production.

`scripts/verify-base-path.mjs` runs inside each app's `build` script, not as a
separate CI step, so no build can skip it. It distinguishes the two legitimate
output shapes: relative `./_app/` references (fully prerendered pages, immune to
base mistakes) and absolute `/_app/` references (SPA fallback, vulnerable). It
fails when it finds neither, because a check that passes silently on an
unrecognised shape reads as a guarantee it is not providing.

The docs site turns out to emit only relative references and is structurally
immune. The playground was the real exposure, and it now also declares
`paths.base` and uses `404.html` as its fallback.

### D-022 · mdBook retired

`book.toml`, `docs/book/`, `SUMMARY.md`, and every `mdbook` justfile target are
gone. Navigation now derives from the typed manifest in
`docs/experience/src/lib/data/docs.ts`, which requires a Diataxis `kind` on every
entry, so a page cannot be added without deciding where it belongs.

One deploy builds both apps and the Rust API docs into a single directory. The
old workflow deployed mdBook only, which is why the README's playground link had
404'd since it was written.

The old mdBook index carried a pipeline diagram and an implementations table
worth keeping; both moved to the landing page. Its "~958 conformance tests"
claim was unverified and was replaced with measured numbers (274 Rust, 294
Python, 258 TypeScript, 27 fixtures).

### D-021 · Enactment over explanation

The landing page runs the engine above the fold. The `<matcher>` tag works in any
content file, and the pattern is the cix seam: `rehype-raw` parses literal HTML
in Markdown, and a renderer map swaps the tag for a Svelte component. Content
stays plain Markdown, portable and preprocessor-free.

The engine evaluates during prerender, so the correct decision is in the static
HTML before hydration and becomes editable after.

The missing how-to quadrant was written rather than shown empty: routing on a
header, adding a custom input, debugging a match, and sharing one config across
languages.

---

## 2026-08-11 · CI and skill-corpus pass

### D-020 · rust-mastery references trimmed from 1,965 to 778 lines

The whole corpus arrived in one commit (`52e22de`) as a side-car to a PR titled
"RE2 migration for puma and bumi" — a Python and TypeScript change. SKILL.md was
authored for x.uma and is accurate. `references/` came in as a complete generic
Rust reference set, unfiltered, and was never reviewed as a corpus.

Deleted, after verifying zero trigger tokens across `rumi/`: `async.md`,
`backend.md`, `native.md`, `embedded.md`, `proc-macros.md`, `networking.md`,
`data-plane.md`, `sources.md`. `frontend.md` was replaced by `wasm.md`, retargeted
at the actual wasm-bindgen crust.

`data-plane.md` looked relevant because it mentions xDS eight times. Those
mentions are about the xDS *discovery transport* (`xds-api`, tonic, async), and
its own verdict table says "xDS control plane: use Go, not Rust". x.uma
implements the Unified Matcher *data model*, with zero tonic and zero async.
Vocabulary collision, not relevance.

`cli.md` and `ecosystem.md` survive but recommended `color-eyre` and `thiserror`,
which CLAUDE.md lists as anti-patterns. Both now carry an override banner at the
point of contradiction.

**Rule going forward:** do not add a reference for a domain until the repo
actually triggers it.

**Correction, same day.** Trimming by trigger token was too crude a rule. It
tests current usage, not transferable judgment, and it cost one thing worth
keeping: `embedded.md` carried a worked typestate example (`PhantomData<State>`,
consuming `self` to transition) which is the exact mechanism behind
`RegistryBuilder::build(self)` and a demonstration of the principle SKILL.md
argues for. It was dropped because its host file mentioned `cortex-m`.

The pattern was lifted into SKILL.md §6 as a three-tier ladder (enum → consume
`self` → runtime check), which is a better home than a reference file nothing
routed to. Everything deleted remains in git (`git show HEAD~1:<path>`); nothing
is unrecoverable. `async.md`'s cancellation-safety material stays cut on purpose:
it is entirely tokio-specific, core forbids async, and if a transport ever lands
we should fetch current guidance rather than a 2026-02 snapshot.

### D-019 · Four factual drifts corrected in SKILL.md

Verified against source, not taken on report. The load-bearing one: SKILL.md
documented `Custom(Box<dyn Any>)` when the real variant is
`Custom(Arc<dyn CustomMatchData>)` (`matching_data.rs:139`). Code written from
that spelling does not compile, and `Box` would break `Arc::ptr_eq` identity
while `dyn Any` would drop the `Send + Sync + Debug` bounds.

Also corrected: "`Matcher::new` → `validate()` → use" is true only of the
registry and FFI paths, not the domain compilers; INV-7 is scoped to the
predicate tree because `Matcher::evaluate_with_trace` must honour first-match-wins
(`matcher.rs:193`); the constraints table named 1 of 5 resource limits; and
iterative evaluation is deferred to v0.2, not enforced. The INV-7 error was
duplicated in the maintainer skill and was fixed in both places.

### D-018 · `check-no-std` deleted rather than repaired

It named the wrong package (`rumi` instead of `rumi-core`) and a feature that
does not exist (`alloc`). There is no `#![no_std]` anywhere and `regex` with
default features requires std. It asserted a capability the project does not
have and had never once run. If no_std is wanted, it is a feature to build, not
a check to keep.

### D-017 · Test suites now run in CI

Nothing ran them on push or PR. `docs.yml` built documentation and deployed it;
that was the entire CI surface. For a project whose correctness claim is "five
implementations pass one conformance suite", nothing enforced the claim.

`ci.yml` runs rust, python, typescript, playground, and `cargo audit`. Every
command in it was verified locally first, and `just ci` runs the identical
sequence so green on a laptop means green on GitHub.

Four checks were broken and had been for some time: `bumi typecheck` (tsconfig
named `bun-types`, `@types/bun` is installed), `bumi fmt:check` (`--check` is
Prettier's flag, Biome has none), `puma mypy` (no stubs for google-re2 under
strict), and `clippy --all-targets -D warnings` (a dead `user_id` field in a test
struct, written three times and read zero times). CI would have caught all four
on the commit that introduced them.

### D-016 · Lockfile policy left as-is, flagged

`.gitignore:3` ignores `Cargo.lock` and `:87` ignores `bun.lock`, yet
`bumi/bun.lock` is tracked. Rule and reality disagree. Committing `Cargo.lock` is
the norm for a workspace shipping a binary (`rumi-cli`). Left unchanged because
it is a deliberate prior choice; CI is written not to depend on the missing
lockfiles. **Open question for the maintainer.**

---

## 2026-08-11 · Public-readiness pass

### D-015 · Packages are unpublished, and the docs now say so

`rumi-core`, `rumi-http`, `rumi-cli`, `xuma`, and `xuma-crust` resolve on no
registry. crates.io, PyPI, and npm are all empty, and no git tag exists. Both
release workflows are `workflow_dispatch` and have never been run.

The README and all three getting-started pages previously opened with install
commands that 404. They now carry a pre-release note and a build-from-source
path that works today.

**Revisit when:** the first release lands. Remove the notes, restore the plain
install instructions, and claim the names before someone else does. The names
are chosen, not reserved.

### D-014 · bumi publishes built output, not raw TypeScript

`bumi/package.json` pointed `main` and `types` at `src/index.ts` and shipped
`files: ["src/"]`, with no build step. Source uses `.ts` import specifiers, so
any consumer type-checking against it failed. That was measurable: the playground
reported 20 such errors from inside `node_modules/xuma`.

Added `tsconfig.build.json` using TypeScript 5.7's `rewriteRelativeImportExtensions`,
which rewrites `./predicate.ts` to `./predicate.js` on emit. Package now ships
`dist/` with `.js`, `.d.ts`, and source maps. Playground type errors went to zero.

**Consequence:** bun copies local file deps at install time, so `dist/` must
exist before install. The `playground-*` just targets now depend on `bumi-build`.

### D-013 · License unified to `MIT OR Apache-2.0`

The repo ships `LICENSE-MIT` and `LICENSE-APACHE`, and the Rust crates declared
the dual license, but puma declared `MIT` and bumi declared `MIT`. `xuma-crust`
declared nothing at all and had no description.

All four now declare `MIT OR Apache-2.0`, matching the files actually shipped.

### D-012 · `just build` and `just test` follow `default-members`

Both used `--workspace`, which overrides `default-members` and pulled in
`rumi-proto`. Its sources come from `buf generate` and are not checked in, so a
fresh clone could not build or test. The doc targets already used
`--exclude rumi-proto`, so the intent was established.

Dropped `--workspace` from `build`, `test`, and `lint`. `default-members`
(`core`, `ext/test`, `ext/http`, `cli`) is the default set, matching the comment
already in `rumi/Cargo.toml`. Run `just gen` before working on proto.

### D-011 · CLI config fixture is checked in

`load_yaml_config` and `eval_yaml_config` read a hardcoded
`/tmp/xuma-yaml-test/config.yaml` that no clone ever had. They failed for
everyone except the machine that created it by hand.

Fixture now lives at `rumi/cli/tests/fixtures/config.yaml` and resolves through
`CARGO_MANIFEST_DIR`, so it works from any directory on any machine.

### D-010 · The playground keeps its local path dependency on bumi

`"xuma": "../bumi"` stays, rather than moving to the published package.

The playground must demonstrate the engine at the current commit. Pointing it at
a registry version would make it lag the repo and demo behavior that is not the
code in front of the reader. Install instructions are the opposite case and must
name published packages.

**Watch for:** drift between published `xuma` and local `bumi`. The site should
state which version the playground runs.

---

## 2026-08-11 · Matcher diagram

### D-009 · roughjs, not Excalidraw or tldraw

The diagram is derived from user config at runtime, so there is nothing to
author. Excalidraw and tldraw both declare `react` and `react-dom` peer
dependencies, and this is a Svelte 5 app. The React-free Excalidraw packages
exist but publish only as prereleases (`@excalidraw/utils@0.1.3-test32`).

roughjs is 0.17 MB with no peer dependencies, and it is the layer Excalidraw
draws with. What Excalidraw adds on top is an element model, an editor, arrow
binding, text-in-shape layout, and its font. Of those, a read-only tree needs
text placement and a font.

**Revisit when:** the diagram needs to become editable. That is the case
Excalidraw and tldraw are actually built for.

### D-008 · Node size has exactly one source

The original bug: `layout.ts` told ELK every node was 180x50 while the DOM sized
nodes by content. ELK solved a layout for boxes that were never drawn, so nodes
overlapped and edges routed through them.

`measure.ts` is now the only place a node's dimensions are computed. Layout and
drawing both read it, so they cannot disagree. Sizing is arithmetic off the
monospace advance width rather than DOM measurement, which keeps it
deterministic, identical between server and client, and free of the
measure-then-layout race.

**Consequence:** this holds only while the diagram uses a monospace face. A
proportional font would require real measurement, and the race would return.

### D-007 · `.excalidraw` export without the dependency

The format is plain JSON. `excalidraw.ts` writes it directly rather than
depending on a React package to produce a file the app opens anyway.

Exports use the vivid trichrome while the screen uses soft OKLCH. Per
`~/mox/brand/system.md` the soft values are the documentation layer and the
vivid ones are the brand layer. An exported file travels into decks and design
docs, so it is a brand-layer artifact.

### D-006 · The graph model is renderer-independent

`config-to-graph.ts` imported `Node` and `Edge` from `@xyflow/svelte` and set a
CSS string on the fallback edge. The model now defines its own types and carries
`kind: "no-match"` instead. The renderer decides that means dashed, red, and
labelled.

roughjs shapes draw with a seed hashed from the node id. roughjs is otherwise
non-deterministic, and an unseeded redraw makes the diagram twitch on every
keystroke.

---

## 2026-08-11 · Documentation site

### D-005 · SvelteKit with a content/experience split, not mdBook and not MDsveX

mdBook cannot embed the live engine in prose. It renders Markdown to static HTML
with no bundler and no component model, so the playground can only ever be a
separate destination reached by a link. For an engine whose whole surface is
config in, decision out, live examples inside the prose are categorical.

MDsveX was considered and rejected. The pattern to follow is the one already
running in `~/mox/products/cix/docs/`:

- `docs/content/` holds plain Markdown, framework-free and portable
- `docs/experience/` is a SvelteKit app, aliased to it via `$content`
- content loads through `import.meta.glob(..., { query: '?raw', eager: true })`
- `svelte-exmarkdown` renders it, with `rehype-raw` parsing literal HTML
- a renderer map turns a custom tag into a Svelte component

That last line is the seam. cix writes `<chart id="..."/>` in Markdown and maps
`chart` to a component. x.uma gets `<matcher example="..."/>`. The Markdown stays
Markdown, so content is portable and there is no preprocessor in the build.

### D-004 · Typography register: cix · operator

IBM Plex Sans for display, IBM Plex Mono for body. From `~/mox/brand/system.md`:
"this is a tool, not a magazine." A matcher engine's documentation is operator
surface, not publication surface.

### D-003 · Wordmark only, no sigil yet

x.uma has no entry in `~/mox/brand/family-sigils.md`. Rather than improvise a
variant or block the site on commissioning one, the site ships on typography and
trichrome alone.

**Revisit when:** the studio has capacity to design the mark. Adding it later
does not require reworking the site.

### D-002 · Four Diátaxis quadrants, how-tos to be written

The mox tokens define `--quadrant-explanation`, `--quadrant-how-to`,
`--quadrant-reference`, and `--quadrant-tutorials`, so Diátaxis is tokenized into
the brand. Existing docs cover three quadrants. The how-to pages get written as
part of the site work rather than shipping a visibly empty quadrant.

Navigation derives from a typed manifest carrying `kind`, so a page cannot be
added without declaring which quadrant it belongs to.

### D-001 · Soft OKLCH throughout, vivid on the landing hero

Follows `system.md` exactly. The documentation layer uses the soft OKLCH
trichrome. The brand layer, meaning one hero moment, uses the vivid hex.
Restraint in the body, impact at the entrance.
