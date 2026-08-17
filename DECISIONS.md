# Decisions

Decisions of record for x.uma. Newest first. Each entry states what was decided,
why, and what would justify revisiting it.

Format is deliberately light. If a decision needs a page of argument, it belongs
in `scratch/` and gets summarized here.

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

**What would overturn this:** wanting a prost 0.14 runtime. Then the pins move
together with it, and whether `Any` gains `Eq`/`Hash` upstream has to be checked
first — not assumed.

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
rm -rf" preset is written with `xuma.test.v1.StringInput` keys and fails to load
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
- `xuma.test.v1.StringInput.value` being used as a lookup key (`PLAN.md` F13) is
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
