---
name: protocol-mastery
description: >
  The wire side of x.uma: protobuf schemas under proto/xuma/, buf codegen, the xDS Unified
  Matcher API and Envoy's semantics for it, TypedExtensionConfig and google.protobuf.Any, type
  URLs, and canonical protojson. Load it when adding or changing a .proto message or field,
  touching buf.yaml or the buf.gen templates, regenerating code, changing how configs load,
  resolving an Any, or deciding what an xDS field means for rumi. design-mastery covers whether
  a schema change should happen; rust-mastery covers the Rust around it.
---

# Protocol mastery for x.uma

x.uma implements the xDS Unified Matcher API. Its config is an xDS `Matcher` message; every
input and every action in it is a `TypedExtensionConfig`, a name plus a
`google.protobuf.Any`. The proto schema is the contract users write against. The Rust code is
one implementation of it.

That ordering is the first principle, and everything else follows from it.

---

## 1. The schema is the contract

### The proto decides what a config means

**Principle.** When the proto and the code disagree, the code is wrong. A field's meaning is
written in the `.proto` comment above it, and the code implements that comment.

**Why.** Users, control planes and other implementations read the schema, not rumi's source.
Behaviour that lives only in code is a second, undocumented contract. And the schema, the type
URLs and the config format all freeze at first publish.

**What it helps you catch.**
- A behaviour added in Rust with no comment in the proto.
- A comment that says one thing while the converter does another.
- A field that exists in the proto but is silently ignored. That is why the loader refuses
  `keep_matching: true` rather than accepting it: rumi does not implement it, and accepting it
  would let a config read one way and behave another.

**Why it helps.** The maintainer can review a schema change by reading the proto diff alone.

### Every message and field says what it does, including when it is absent

**Principle.** Each message has a comment. Each field's comment says what it means, what an
empty or default value does, and what happens when the value is missing at match time.

**Why.** In proto3 a missing scalar is indistinguishable from its default. So "absent" has to be
defined in words. In x.uma it decides whether a gate can fail open: an input that reads nothing
makes its predicate false, and an input marked `required` that reads nothing leaves the match
undecided.

**What it helps you catch.** A new input message whose comment does not say what absence does.

**Why it helps.** Fail-open questions get answered in review, from the schema, before any code
runs.

### Additive changes only, checked by `buf breaking`

**Principle.** Add fields and messages. Never renumber, retype or reuse a field number. When
removing a field, reserve its number and name.

**Why.** A config written today must still load tomorrow. Proto's wire compatibility rests
entirely on field numbers keeping their meaning. `buf breaking` checks the schema against
`main` at file level (`just breaking`). CI does not run it, nor `buf lint`, so run both
yourself before a schema change merges, until a CI step does.

**What it helps you catch.**
- A "harmless" rename.
- A field number reused after a removal.
- A `string` turned into a message.

**Why it helps.** The compatibility promise becomes a tool's result, not a reviewer's memory,
and once it runs in CI, it holds without anyone remembering.

---

## 2. Codegen

### One generated tree, for the one engine

**Principle.** buf generates Rust only, into `rumi/proto/src/gen/`, and the generated code is
committed. Python and TypeScript have no generated types: the bindings hand protojson text to
the engine, which reads it through this one tree.

**Why.** Generated types in each language were tried and measured. Neither the TypeScript nor
the Python generator rejected an unknown field: a misspelt key became an empty value without
an error. A config with a typo then quietly means something else, which for a security matcher
is the worst failure. One strict reader in the engine is better than three lenient ones.

**What it helps you catch.** A proposal to add protobuf codegen to a binding "for type safety".
Ask whether its reader rejects unknown fields before anything else.

**Why it helps.** Strictness lives in one place, so it is tested in one place.

### Two buf passes, pinned plugins, and `clean` only on the first

**Principle.** `just gen` runs `buf.gen.yaml` first and then `buf.gen.rust.yaml`.
- The first pass has `clean: true` and wipes the tree.
- The second pass generates the xDS dependency types that no local proto imports, and appends
  them to the tree.
- Plugin versions are pinned in both files, and the two files must agree.

CI fails if the committed tree differs from a fresh generation.

**Why.**
- **`clean` in both passes** would delete what the first pass wrote.
- **Floating plugin versions** have already broken the build once. A newer prost plugin derived
  `Eq` and `Hash` on messages that hold an `Any`, which does not implement them, so xDS's
  `TypedExtensionConfig` stopped compiling.
- **The pinned plugin** matches the `prost` runtime version in `rumi/proto`.

**What it helps you catch.**
- Adding `clean` to the second template.
- Bumping one plugin without the other, or without the runtime crate.
- Committing a hand edit inside `gen/`.

**Why it helps.** Regeneration stays a boring, reproducible step, and a drift in it is caught
by CI, not discovered by a user.

---

## 3. `Any`, type URLs and protojson

### Type URLs: one spelling written, one name compared

**Principle.** Documents write type URLs as `type.googleapis.com/<package>.<Message>`. Strip
the prefix at one boundary and look up by the fully qualified message name.

Today that holds on the config-loading path: the `Any` resolver strips the
`type.googleapis.com/` prefix. The core `Registry` stores and looks up type URLs as raw strings,
so a `TypedExtensionConfig` built by hand in Rust with the full URL does not resolve. Know
which path you are on.

**Why.** Valid type URLs come in more than one spelling, and protobuf libraries disagree on
which they emit. Comparing raw strings makes two correct configs disagree.

**What it helps you catch.** A new lookup that compares the URL as given.

**Why it helps.** "Works from a file, fails from a control plane" never becomes a bug report.

### Canonical protojson writes `Any` expanded; rumi reads it through binary

**Principle.**
- **Canonical protojson writes an `Any` expanded:** the payload's own fields inline, beside an
  `@type` key.
- **The serde code generated for x.uma's protos writes it packed:** a `typeUrl` beside base64
  of the binary encoding.
- **rumi converts expanded documents to packed form** through the same registry that decodes
  `Any` payloads. Every config, from a file or from a control plane, then meets at one proto
  `Matcher` value and goes through one conversion walk.

**Why.** Expanding an `Any` needs the payload's schema at serialisation time, which static
codegen does not have. So the generated code cannot read canonical documents unaided.

Routing through binary costs a round trip once per config load, never per match. It buys one
conversion, not two. Two readers of the same semantics have already disagreed once in this
repository.

The registry that decodes is the registry that encodes, so the set of readable types and the
set of writable types are one set by construction.

**What it helps you catch.** A second loader that reads JSON straight into config types: faster
to write, and free to disagree with the binary path.

**Why it helps.** The control-plane path and the file path cannot drift, because they are the
same path.

### Unknown fields are rejected

**Principle.** The loader rejects any field the schema does not define. A conformance fixture
requires it of the engine and both bindings.

**Why.** A typo in a matcher config must fail loudly. Accepting it means the typo's intended
meaning silently disappears, and the config protects less than its author thinks. Variation the
design anticipates (new extension types) arrives through type URLs, not through unknown
fields.

**What it helps you catch.** Any lenient deserialisation path: `#[serde(default)]` on a config
type, an ignore-unknown-fields option, a hand-written reader.

**Why it helps.** "My rule never fires" turns into a load error with the field's name.

### `TypedExtensionConfig`: the name is a label

**Principle.** In `TypedExtensionConfig`, `typed_config`'s type URL decides what is loaded.
`name` is a label for humans and traces, and nothing resolves by it.

**Why.** That is the xDS contract. Envoy resolves extensions by type URL, so resolving by name
would make a config mean different things in Envoy and in rumi.

**What it helps you catch.** Code that dispatches on `name`, or a docs example implying it
matters.

**Why it helps.** Configs stay portable between rumi and Envoy.

---

## 4. Envoy's matcher semantics, as rumi implements them

The reference is Envoy: the protos under `~/oss/reference/cncf-xds/xds/type/matcher/v3/`, and
the implementation under `~/oss/reference/envoy/source/common/matcher/` and
`~/oss/reference/envoy/envoy/matcher/`. When a behaviour is unclear, read Envoy's code before
deciding. When rumi departs from it, the departure is a decision with a D-entry.

| Concept | xDS / Envoy | rumi |
|---|---|---|
| `OnMatch` | `oneof { matcher; action }`, never both | `enum OnMatch { Action(A), Matcher(Box<…>) }` |
| Nested matcher misses | the parent's `OnMatch` fails, and evaluation continues with the next field matcher | same |
| `on_no_match` | on `Matcher` only, not per `OnMatch` | `Matcher.on_no_match`; a tree has no fallback field of its own |
| List vs tree | `oneof matcher_type` | `MatcherKind::List` or `MatcherKind::Tree`, exclusive |
| First match wins | yes; `keep_matching` records an action and continues | first match wins; `keep_matching: true` is refused at load because it is not implemented |
| Prefix tree | on a nested miss under the longest prefix, retry the next shorter (Envoy 1.35) | every matching prefix is tried, longest first, and the trace records each attempt |
| Missing data | three states: not available, more might arrive, all available | an absent value makes a predicate false; for a `required` input it leaves the match undecided. That is the "required value absent" case of Envoy's richer model; streaming ("more might arrive") is not implemented |
| Regex | RE2 | Rust's `regex` crate: linear time, close to RE2 but not identical |

**What the table helps you catch.** A "fix" that makes rumi friendlier but different from
Envoy, such as a fallback per `OnMatch`, or continuing after a match.

**Why it helps.** Users who know Envoy can trust rumi to mean the same thing, and the
maintainer can settle a semantic question by pointing at Envoy's code.

---

## 5. Namespaces and layout

- **Packages:** extensions live under `xuma.<area>.v1`: `xuma.core.v1` (shared inputs, matchers,
  actions), `xuma.http.v1`, `xuma.claude.v1`. New domains follow the pattern of Google
  (`google.protobuf`), xDS (`xds.type.matcher.v3`) and Envoy (`envoy.config.core.v3`).
- **Schemas:** `proto/xuma/<area>/v1/*.proto`. Upstream xDS and protoc-gen-validate arrive as
  buf dependencies (`buf.yaml`); never vendor them.
- **Generated code:** `rumi/proto/src/gen/`, machine-written. Hand-written code beside it
  converts the proto `Matcher` to rumi's internal form (`convert.rs`), resolves `Any`
  (`any_resolver.rs`) and handles protojson (`protojson.rs`).
- **A domain's context:** described by a proto message in its package, such as
  `xuma.claude.v1.HookContext`. The proto describes what a host can supply; the inputs read
  from it.
- **Lint:** `buf lint` with the STANDARD rules, except version-suffix naming (`just lint-proto`).

---

## 6. Wire formats

| Format | When x.uma uses it |
|---|---|
| Canonical protojson (YAML or JSON syntax) | config files, docs, fixtures, and what the bindings accept |
| Binary proto in an `Any` | the internal meeting point of every load path, and the format a control plane would deliver |
| `TypedStruct` (schema-less JSON) | not used. x.uma's extensions have schemas, so `TypedExtensionConfig` is enough |

Delivering configs from a control plane (for example via Envoy's extension config discovery
service, ECDS) is not built. Nothing in the engine should assume it either way.
