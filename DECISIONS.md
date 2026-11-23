# Decisions

Decisions of record for x.uma. Newest first. Each entry states what was decided,
why, and what would justify revisiting it.

Format is deliberately light. If a decision needs a page of argument, it belongs
in `scratch/` and gets summarized here.

---

## 2026-09-24 · Budgets and declared inputs, before publish

### D-069 · D-057 before publish: `MapInput` in core, `required` as Envoy's insufficient data, typed evaluate stays

**Decided.** Narrows D-057 to what publishing freezes, and says what its
`required` flag does, which D-057 named and did not define.

1. `xuma.kv.v1.MapInput{key}` becomes `xuma.core.v1.MapInput{key, type,
   required}`; the kv message is deleted. `type` is `"string"` (the default),
   `"int"` or `"bool"`, the vocabulary `DataInput::data_type` already speaks,
   so a matcher that cannot read the declared type is refused at load. A value
   that does not parse as its declared type is absent.
2. Every input message declares `bool required`. The registry reads it, not
   the factory, so no input implements absence twice. `required: false`, the
   default, keeps today's xDS semantics: an absent value makes its predicate
   false.
3. An absent **required** value is Envoy's `InsufficientData`
   (`source/common/matcher/field_matcher.h`, `list_matcher.h`): the predicate
   is undecided; `not` passes undecided through; `and` returns the first child
   that is not a match, undecided or no match, in order; `or` matches if any
   child matches and is otherwise undecided if any child was; a list or tree
   that meets undecided stops, and neither a later rule nor `on_no_match`
   fires. The engine's `Matcher::outcome` returns `Outcome::Undecided`
   (`evaluate` still returns `Option<A>`, `None` here); the bindings' result
   object gains `undecided: bool` beside `{matched, action, index,
   fallback}`, because a host that treats "no rule fired" as allow may treat
   "could not decide" as deny, and the result object freezes at publish; the
   trace marks the predicate `undecided`.
4. The typed `evaluate` methods on the bindings stay. The flat map D-057
   proposed measured at 2.25 µs against 600 to 900 ns for typed arguments in
   wasm (artifact `2026-09-08-f6c3adb3`); the packed string that beats both is
   additive and is 0.1.x.

**Why.** An absent value is false for a plain predicate, but `not` turns it
true: "allow unless role is guest" allows a request with no role (verified by
execution, 2026-09-24). Envoy does the same for a value that is absent with
all data available, so the engine is conformant, and the security page's
"missing data means no match" overstated what the invariant covers. `required`
is how a rule author says a value's absence is not an answer, and Envoy
already has the semantics for that: the evaluation cannot decide, so it does
not, rather than inventing a three-valued logic of our own. The host that
receives no decision applies its own default, which is where the policy for a
missing value belongs (the matcher is not a policy engine).

**What it entails.** Fixtures first, in `60_declared_inputs.yaml`:
`declared_input_type_mismatch` and `declared_input_unknown_type` (load
errors); `map_input_typed` (a `bool` read, and an unparsable value absent);
`required_input_absent_under_{not,and,or}`, `required_input_absent_stops_the_list`
and `required_tree_key_absent` (each asserting `on_no_match` does not fire);
`optional_input_absent_under_not` pinning the xDS default. D-057's
`declared_input_unknown_url` already exists as `unknown_input_type_url`. A
case asserts `expect_undecided`, and every other case now asserts it is
decided. The `MapInput` type URL changes in every fixture. The registration
seam and `just ext` follow in their own change.

**Revisit if.** Streaming input (Envoy's `MoreDataMightBeAvailable`) is
needed. `Undecided` is the state it would reuse; the difference is that the
host would call again with more data.

### D-068 · Per-request cost is budgeted along the worst path; the per-list and per-compound bounds go

**Decided.** Revises D-054. `MAX_PREDICATES_PER_EVALUATION = 256` bounds the
predicates one evaluation can reach, computed once the matcher is built: a list
costs every rule's predicate plus its nested matcher, since a nested miss moves
on to the next rule; an exact tree costs its dearest entry; a prefix tree costs
its dearest chain of prefixes, since a miss falls through to the next shorter
one (D-055); a fallback adds its own cost. `MAX_DOCUMENT_PREDICATES = 65_536`
bounds the document, counted during load as D-054's leaf budget was, so a
hostile document is still refused before it is materialised. It is a memory
bound, the same size as `MAX_TREE_ENTRIES`. `MAX_FIELD_MATCHERS` and
`MAX_PREDICATES_PER_COMPOUND` are removed, with their errors.

**Why.** D-054's leaf budget summed every leaf in the document, including those
under tree entries a request never visits. With the value 256 it capped any
document at 256 rules, so the trees D-054 names as the way to scale could not
hold more than 256 single-rule entries (verified by execution 2026-09-24: 256
entries load, 257 are refused, 300 plain-action entries load). It also made the
per-list bound unreachable, since every rule holds at least one predicate. The
2026-09-07 reviews bounded the right thing and the budget did not: Dijkstra,
that lifting the width is a security decision and a dispatcher over a list that
cannot exist is not an optimisation; Knuth, that per-request cost is linear in
the rules visited (~21 ns per rule on a miss) and scale comes from dispatch.
Both are statements about one request. The path budget is that statement, and
it covers a wide list and a wide compound, which lie on one path. Two bounds on
one quantity leave a reader to work out which fires; one does not.

**What it entails.** The per-request security posture is unchanged: a list or a
compound over 256 predicates is still refused, now by the path budget. What
changes is that a tree's entries no longer add up. Fixtures first:
`budget_tree_entries_do_not_add_up` and `budget_prefix_siblings_do_not_add_up`
(entries summing past 256, each a hundred, load and evaluate);
`budget_prefix_chain_adds_up` (a chain of prefixes summing to 300 is
refused); `budget_list_rules_add_up`, formerly `budget_aggregate_leaves`,
keeps its refusal under the new message. The document budget and the regex
budget are asserted in registry unit tests at exactly budget + 1, on the IR
path, which has no byte bound; through protojson `MAX_DOCUMENT_BYTES` binds
first.

**Revisit if.** The N-scaling benchmark (Knuth; Ixian's
`rule_count_{last_match,miss}`) shows a workload that needs more than 256
predicates on one path. Raising the number is then the security decision
Dijkstra named, taken on those measurements.

---

## 2026-09-08 · The freeze line: what is decided before first publish

Sixteen decisions from one deliberation (2026-09-07, focus mode: Burner,
Karman, Ace, K; Lotfi on two conflicts; Ixian's validation criteria), over a
proposal for one hyper-efficient core with two thin crusts. Recorded as a batch
because they share a boundary: everything publishing freezes is decided before
0.1.0, and everything it does not freeze comes after (D-053). The guild record
and the proposal are session files under `scratch/`, untracked; whatever an
entry needs from them is restated in the entry. Numbered in work order.

### D-052 · `puma/` and `bumi/` are deleted, not frozen as oracles

**Decided.** Both trees are deleted (`f3208660`, 2026-09-08). Neither is kept
as a frozen conformance oracle.

**Why.** An oracle has to be trusted more than the thing it checks. The pure
engines shared every spec misreading with rumi (D-050's fail-open was live in
all three; D-051), so agreement with them proved conformity to a shared
reading, not correctness. A frozen tree also keeps costing: it builds under
every toolchain bump or it leaves CI, and an oracle outside CI is a file. The
oracle the engine work needs is the linear list itself: a differential
property test asserting that any optimised path returns what the plain scan
returns, over generated matchers, run in the engine's own suite (D-053, step
6). That is an oracle with the same reader as the code under test and no
second reading of the spec to agree with.

**What it entails.** The `implementations:` ledger stays and is required
(`acd87e12`); its members are `rust`, `python`, `typescript`, meaning the
engine, the PyO3 binding and the wasm binding. Gate: `git grep -w -e puma -e
bumi` returns nothing outside `DECISIONS.md`, `PLAN.md` and `reference/`,
which are records and are not rewritten. History before 2026-09-08 is in git.

**Revisit if.** D-051's condition holds: a runtime the binding cannot reach,
with usage. A second engine would then be written fresh against the fixture
suite, not recovered from history.

### D-053 · The sequence: retire, decide, rename, shape the boundary, publish 0.1.0, then the engine

**Decided.** The order of work to first publish:

1. retire the duplicate engines (done: D-052);
2. this batch;
3. the rename pass (D-060), one commit, before any hot-path change;
4. boundary shaping: `OnceLock` registries, `ActionIndex` return (D-056),
   the flat context and declared inputs (D-057), one HTTP context (D-059),
   the registration seam, `DataInput::get` returning `Option` (D-061);
5. publish 0.1.0 behind the benchmark gate (D-066) and the crates dry-run;
6. the engine, as 0.1.x: borrowed extraction, compilers emitting trees,
   multi-pattern dispatch under the differential oracle, regex limits, the
   semantic matcher.

**Why.** Publishing freezes the FFI shape, the config schema, the type URLs,
the public traits and the security posture. It does not freeze the hot path: a
faster `evaluate` behind the same `from_config` is a patch release. So whatever
changes a frozen surface lands before 0.1.0, and whatever changes only speed
lands after it. The proposal's own sequence put the engine work first and the
retirement last; K's objection, adopted, is that this ties first publish to
work whose gate is a benchmark with no noise floor yet, and delays a freeze
that costs nothing while nothing is published. Retiring first is because every
later gate ("all runtimes agree") is cheaper with three runtimes than five,
and because a document written before the retirement describes packages that
no longer exist.

**What it entails.** Nothing in 0.1.x may change the documents `from_config`
accepts, the type URLs, the result object, a public trait, or the trace steps
(D-063). This answers D-051's open questions: deletion, not freezing (D-052);
this order; and `xuma` on npm ships as the thin package over the crust (D-058)
once step 4 lands, never as the pure engine in the interval.

**Revisit if.** Step 4 turns out to need an engine change that is not additive
(borrowed extraction changing the `DataInput` port, say). Then that change
moves before publish, because it changes a public trait.

### D-054 · Rule sets at scale are trees; per-list width stays bounded; aggregate budgets are checked during load

**Revised by D-068** (2026-09-24): the leaf budget summed branches a request
never visits; per-request cost is now budgeted along the worst path.

**Decided.** `MAX_FIELD_MATCHERS = 256` stays as the per-list bound. Large rule
sets are compiled into `MatcherTree`s, with lists beneath the leaves; the
domain compilers will emit first-field dispatch (`event` then `toolName` for
hooks, method then path for HTTP). Three aggregate budgets are added, checked
incrementally during load so a hostile document is refused before it is fully
materialised: total leaf predicates per root matcher; total regex count and
compiled regex memory (`RegexBuilder::size_limit` and `dfa_size_limit`,
closing F-03); and bytes accepted by `from_config`.

**Why.** A list's width is a per-request cost, so 256 is a DoS bound (D-045,
D-049) and lifting it would be a security decision, not tuning. A 10,000-rule
policy cannot load today, and if it could, both compilers emit one
`FieldMatcher` holding an `Or` of every rule (`claude/compiler.rs`,
`ext/http/src/compiler.rs`), so it would scan linearly and allocate once per
rule per evaluate. The tree already exists, is O(1) for exact keys and O(k) for
prefixes, holds 65,536 entries, and is reachable only from hand-written config.
Trees compose across the per-list bound without any limit moving. The per-list
bound is also not a budget: a hundred lists of two hundred rules are legal
today, and regexes compile during the walk while the root `validate` runs
afterwards, so the refusal comes after the cost has been paid.

**What it entails.** Fixtures first, red: `budget_aggregate_leaves` (three
lists of a hundred, each under 256, must fail to load) and
`budget_regex_memory` (the counter at refusal is at most budget + 1, proving
the check is incremental). The budgets change what `from_config` accepts, so
they land before 0.1.0; compilers emitting trees is engine work and lands
after. Each budget's value lives on the type that holds the resource (D-029).

**Revisit if.** A workload has a legitimate flat list over 256 rules that
cannot be keyed on any field. None is known; the hook workload keys on the
event.

### D-055 · Prefix trees fall through to shorter prefixes, as Envoy does since 1.35

**Decided.** When the longest matching prefix's nested matcher produces no
action, evaluation tries the next shorter matching prefix, and so on, before
falling to the enclosing `on_no_match`. The trace follows evaluate (INV-3): it
records each prefix tried, and no lookup evaluate did not perform.

**Why.** Envoy changed this in 1.35.0, released 2025-07-23
(`changelogs/1.35.0.yaml`): "prefix_match_map now continues to search for a
match with a shorter prefix if a longer match does not find an action. This
brings it in line with the behavior of matcher_list." Verified in
`source/common/matcher/prefix_map_matcher.h` at checkout `9d47ea91`:
`findMatchingPrefixes` returns every prefix, the loop walks longest first, and
on a nested miss continues, behind
`prefix_map_matcher_resume_after_subtree_miss`, which defaults on. rumi stops
at the longest (`matcher_tree.rs:129`, `find_longest_prefix_entry`), and a
nested miss goes straight to `on_no_match`. D-044 and D-046 (2026-08-18)
matched the pre-1.35 source, which the maintainer had analysed in March 2025;
neither considered retrying a shorter prefix. The proto is silent ("Longest
matching prefix wins", nothing about siblings), so this is a choice, and
Envoy's stated reason is the one to follow: a list continues past a rule whose
nested matcher misses, and a prefix tree is a list of prefixes ordered by
length.

**What it entails.** `radix_tree.rs:120` has `find_all_prefixes`, unused by
`evaluate` today. Fixture `prefix_tree_fallthrough` in `07_protojson/`, red at
HEAD, ledger all three: the tree `{"/api" → nested matcher that misses, "/" →
X}` on `/api/users` yields `X`. Exact maps are unaffected: one key, one
lookup. D-046's rule stands, since every recorded lookup is one evaluate made.

**Revisit if.** Envoy removes the guard in the other direction. Not expected:
the flag is the migration path away from their legacy behaviour.

### D-056 · The engine returns action indices; the crust holds the table; the user sees a result object

**Decided.** Across the FFI the engine returns `[ActionIndex]`, a slice, today
of length zero or one, with a scalar convenience beside it. In Rust the result
is `Option<ActionIndex>`; a sentinel exists only at the encoding, never in a
type. Indices are assigned at load by an `ActionFactory` for `u32` in the
shared `rumi-proto` load path, so PyO3 and wasm number identically, by a
deterministic pre-order walk with tree keys sorted, because `HashMap`
iteration order is not stable. The crust receives the typed action table once,
at load. The user never sees a bare integer: the result object is `{matched,
action, index, fallback}`. `NamedAction.metadata` is carried, not dropped.

**Why.** The boundary cost is not field access. After `d006b241` the hook path
costs ~780ns and the KV path, which probes no field at all, ~738ns; what
remains is call glue, the owned `String` return, and context construction. A
`u32` return removes one allocation, one transcode and one host string per
call, with certainty. Where the index is assigned was the guild's first
conflict: Burner wanted it crust-side with nothing new in core; Ace objected
to any bare integer reaching a user. Lotfi's resolution, adopted: the factory
sits in `rumi-proto` because both crusts already load through it, which is
the one place a single numbering can be guaranteed for both, and core learns
nothing. The slice is so `keep_matching` (deferred, PLAN.md F2/SF2) lands
additively: a pipeline that wants every applicable policy gets a longer slice,
not a new API. `metadata` is dropped at nine sites in `convert.rs` today
(`metadata: Default::default()`); it is how a policy names its processor
(D-065), so the loader carries it through.

**What it entails.** Fixture `result_slice`. Property: index→action is a
bijection, identical across two loads of the same document. No-op boundary
benchmarks `boundary_floor_{u32, option_string, flat_map}` in both crusts
establish the ABI floor per shape. The Rust API keeps `Matcher<Ctx, A>` with
`A` generic; `ActionIndex` is what the crusts choose for `A`.

**Revisit if.** A native Rust trace must carry the same index as a wasm trace
without going through `rumi-proto`; then the walk moves into core. Or the FFI
result needs the action's typed config per call, in which case the table
crosses per call and the index is redundant.

### D-057 · Inputs are declared by the domain proto messages, resolved at load, and extracted natively from a flat context

**Narrowed by D-069** (2026-09-24): what lands before publish, and what
`required` does.

**Decided.** The only way to declare an input is the domain's proto message:
`xuma.http.v1.HeaderInput{name}`, `xuma.claude.v1.ToolArgInput`, and so on.
Their factories compile to a slot lookup in a flat context, a `Map<String,
MatchingData>` or a packed buffer, chosen by measurement, that the host
marshals once per evaluate. `xuma.kv.v1.MapInput{key}` is promoted to
`xuma.core.v1.MapInput{key, type}` and the kv copy is deleted. No `FieldInput`
(a free field path plus a type as authoring surface) is added. Errors: an
unknown type URL or a type mismatch fails at load, never at evaluate; absence
at evaluate is governed by a per-declaration `required` flag; the trace shows
`absent`, not `false`. Matchers remain Rust-only, with a paved path: fixture
first, `just ext new` scaffolds a template crate, one registration seam that
both crusts read, `just ext build`.

**Why.** The crusts' context path is an Inner Platform by the boundary test:
about 33 hand-written per-domain, per-runtime marshallers for 15
capability-runtime pairs, growing as N×M; a tenth hook event changes three
files. Of three ways to let an extension author add an input from Python or
TypeScript, host callbacks fail on arithmetic (one boundary crossing per
extraction per rule, ~780ns × rules per evaluate) and break `Send + Sync` and
registry immutability; Rust-only makes the engine team the serialisation
point; declared data-only inputs pass. Burner's correction, adopted: the
declaration stays on the proto messages rather than a free field path, or the
legal field set becomes host convention and the registry's load-time
rejection of unknown URLs is lost. Karman: no `FieldInput`, since
`MapInput{key}` already is one, filed under kv by accident of origin. Ace:
absence at evaluate is a domain fact (an optional header), not an error, so it
is declared per input and visible in the trace.

**What it entails.** Fixtures `declared_input_unknown_url`,
`declared_input_type_mismatch`, `required_input_absent`. Registration seam
control: `just ext new demo && just ext build` passes a fixture in both crusts
with an empty diff on the crust registry files. The `MapInput` type URL moves
before publish (D-053). The flat port already exists in shape as `KvContext {
values: HashMap<String, String> }`.

**Revisit if.** An input that computes rather than extracts (an embedding, a
parse) is needed from a host language. Rust-only is the escape hatch by
design; if the paved path becomes the bottleneck, the callback option is
re-examined with a batch shape, not per extraction.

### D-058 · `xuma` on PyPI and npm is a thin host-language package over `xuma-crust`

**Decided.** `xuma` returns as a package whose runtime is `xuma-crust`,
exact-pinned. Every domain identifier in it (field names, event names, type
URLs) is buf-generated; hand-written code is domain-blind glue. A CI check
asserts that no domain field name appears in the package outside `gen/`.
`xuma-crust` leaves the getting-started pages once `xuma` exists.

**Why.** The guild's second conflict, resolved by Lotfi at 0.85 for this over
renaming the crust or putting the DX in Rust. A crust is an ABI, and its
surface is what wasm-bindgen and PyO3 produce; a host-language layer is where
types, docstrings and idioms live. Hand-writing domain vocabulary in that
layer would recreate the drift the retirement removed (D-051); the vocabulary
check is what makes the layer thin by construction rather than by review.

**What it entails.** `xuma` installs with no build step. It agrees with the
`rumi` CLI on the README config (`check-readme-agreement.mjs` grows a
column). `check-publishable.mjs` stops treating `xuma` as unpublishable when
the package exists. Until then, the docs name `xuma-crust`.

**Revisit if.** The glue becomes so small that the page-one example is
identical without it: then rename the crust and delete the layer. Or the
vocabulary check fails twice on one file: then the DX belongs in Rust after
all.

### D-059 · One HTTP context

**Decided.** The Gateway compiler (`rumi-http`, `compile_route_matches`) is
retargeted to the flat context of D-057. `register_simple` and the second
registry it builds are retired. `HttpRequest` and `HttpMessage` become one
context. Then `compile_route_matches` can be exposed from the bindings, and
the playground can regain a Gateway-route mode.

**Why.** F14 found `xuma.http.v1.PathInput` bound to two semantics split by
runtime: the crusts took the registration under which `/admin?x=1` matched
`{Exact: "/admin"}`, and the native path did not. That is the boundary test's
degeneration condition, a type URL whose meaning depends on the caller,
observed in this repo. Two registries over one type URL is the mechanism;
removing the second removes the class of defect. Docs promised
`compileRouteMatches` from the deleted engines; the promise left with
`f3208660` and returns only when it is honest.

**What it entails.** `git grep -cw register_simple rumi/` = 0. `05_http/`'s
ledger lists `rust` and widens to the bindings when they expose the compiler.

**Done, 2026-09-24.** `simple.rs` is deleted; `HttpMessage` is the one context,
and the domain is no longer behind a feature (`message` is gone). The bindings
and the CLI take the query string in the path, as it arrives on the wire: their
separate `queryParams` / `query_params` / `--query` channel could disagree with
the path, and the explicit value won, which is the same two-answers class in
miniature. Exposing `compile_route_matches` from the bindings is additive and
waits for a user who needs it; it would bring `k8s-openapi` into both.

**Revisit if.** Streaming (Envoy's three-valued `MatchResult` with
`InsufficientData`) needs a context that is not flat. That is the shape an
ext_proc adapter would need, and it is not a conformance-surface change.

### D-060 · The rename pass

**Decided.** Karman's table, executed as one commit before any hot-path
change:

| Today | After |
|---|---|
| `config` module | `ir` |
| `MatcherConfig<A>`, `MatcherKindConfig`, `MatcherTreeConfig`, `TreeTypeConfig`, `FieldMatcherConfig`, `PredicateConfig`, `SinglePredicateConfig`, `ValueMatchConfig`, `OnMatchConfig` | `MatcherIr<A>`, `MatcherKindIr`, `MatcherTreeIr`, `TreeTypeIr`, `FieldMatcherIr`, `PredicateIr`, `SinglePredicateIr`, `ValueMatchIr`, `OnMatchIr` |
| `TypedConfig` | `TypedExtensionConfig` |
| `IntoDataInput`, `IntoInputMatcher`, `IntoAction` | `DataInputFactory`, `InputMatcherFactory`, `ActionFactory` |
| `MatchingData::None` | `MatchingData::Absent` |
| `TestMatcher` (both crusts) | `KvMatcher` |
| the frozen `u32` (D-056) | `ActionIndex` |
| `SinglePredicate`, `OnMatch`, `MatcherKind`, `MatchingData` | unchanged: xDS vocabulary |

**Why.** `MatcherConfig` is not the config. The config is the protojson
document (D-026); `MatcherConfig` is what `convert.rs` produces from it and
`load_matcher` consumes, an unbound intermediate whose leaves are
`TypedConfig { type_url, Value }`, resolved later. An IR named "config"
invites a reader to serialise it, and one did: the terse dialect (D-042) was
that mistake. `TypedConfig` is the proto's `TypedExtensionConfig` with a
shorter name; the payload is a config, the envelope is not. `Into*` implies a
`self` being consumed; these traits have no `self` and construct from a
`Config`. `None` on `MatchingData` collides with `Option::None` in every match
arm and hides that absence is a domain state with its own trace rendering
(D-057). `TestMatcher` names the conformance domain after its origin, not its
use; the CLI's default domain is kv (PLAN.md SF8). Renames are free until
publish and a breaking change after, which is why this is a step of its own
and why it precedes the engine work whose diffs would otherwise carry it.

**What it entails.** `UnitConfig` stays: it is a config, the payload of an
input with no fields. Prelude re-exports, rustdoc and docs follow in the same
commit. The gate is the count-zero grep for each old name (`git grep -w`, exit
code checked, a planted hit as control).

**Revisit if.** Not `config`→`ir`. The rest are cheap until publish and frozen
after.

### D-061 · Comments: no *what*; a *why* is one sentence that may cite a decision

**Decided.** Source comments do not describe what code does; the code and the
rustdoc do that. A comment that gives a reason states it in a sentence that
stands on its own, and may add a decision id as the pointer to the fuller
argument. Rustdoc on public items is contract and is kept. History narratives
in source ("until 2026-08-31 this was...") move to this file. `DataInput::get`
returns `Option<MatchingData>`, so that None→false (INV-1) is enforced by the
type rather than restated in a comment at every site; when that lands, the
`Absent` variant of D-060 goes with it, because two spellings of absence with
no stated precedence is the defect D-044 removed one level up, and the trace's
`absent` (D-057) is a field on the step, not a variant of the value.

**Why.** The engine work of 0.1.x touches every hot-path file, and a comment
describing the old mechanism would then describe the wrong one: drift by
construction. "No comments" was proposed; Karman's correction, adopted, is that
it holds for *what* and fails for *why*, since a reader who cannot find the
reason for a limit or an ordering removes it. D-030 said source comments do not
cite this file, because a bare "see D-029" is a pointer the reader cannot
follow if the log is renumbered. That concern is met by requiring the sentence
to carry its reason; the id is then a convenience, and the log has been
append-only and never renumbered through fifty-one entries. D-030's rule that
a comment carries its own reason stands; its rule that a comment never carries
an id is relaxed to that extent, and no further.

**What it entails.** The rename pass (D-060) applies the rule to every file it
touches. The `Option<MatchingData>` port change is decided here and lands in
step 4 of D-053, because it changes the trait extension authors implement and
so precedes publish.

**Revisit if.** The log is ever renumbered. Then every cited id is a broken
pointer, and the sentence-first rule is what keeps the comments true.

### D-062 · `DataInput::get` is pure, and inputs have an identity

**Decided.** Purity of `DataInput::get` (same context, same result; no effect
observable by another call) is a documented precondition of the trait, in its
rustdoc, as contract. The trait gains `fn identity(&self) -> Option<Cow<'_,
str>>`, default `None`; an input returning `Some` asserts that two inputs with
equal identities read the same value from any context.

**Why.** Both are prerequisites for merging predicates that read one input into
a single multi-pattern probe (aho-corasick over literals, `RegexSet` over
regexes), which is where sub-linear evaluation inside a tree bucket comes
from. A merge that reorders or de-duplicates extraction is sound only if
extraction is pure and "same input" is decidable; today the trait bound admits
a `Mutex`, and the trait exposes only `data_type()`. Reordering is already
relied on, silently: `evaluate_with_trace` visits every child where `evaluate`
short-circuits (INV-7), and the two must agree (INV-3). Dijkstra's finding was
that nothing states the precondition that makes that agreement sound.

**What it entails.** Inputs returning `None` from `identity` stay on the
linear path. Declared inputs (D-057) derive an identity from their
declaration, so every generated input has one. The precondition is a trait doc
change before publish; the merging is 0.1.x.

**Revisit if.** A legitimately impure input is needed (a counter, a clock). It
returns `None` from `identity` and is never merged; if it also needs to be
called exactly once per evaluate, that is a new contract and a new decision.

### D-063 · Trace steps are contract

**Decided.** The steps of `evaluate_with_trace` are public surface: across the
FFI as `{index, matched, predicate}` (D-046), in Rust as `EvalSteps`. An engine
that does not walk the linear list must still produce the step list the linear
list would have produced, up to the first match (INV-2, INV-3).

**Why.** The steps cross the FFI, and `rumi --trace` and the playground render
them; a consumer that counts them or reads a rule index has a contract whether
or not one was declared. The alternative, "diagnostic, may change", would let
a dispatcher that skips rules emit fewer steps, and a trace that disagrees
with the config is a second source of truth (D-046). Deciding this before a
dispatcher exists means the dispatcher is built to the contract instead of
the contract being read off the dispatcher.

**What it entails.** Fixture `trace_steps_linear`: step count equals the
linear count; control: a dispatcher that skips must fail it. Tree fall-through
(D-055) adds a step per prefix tried. The differential oracle asserts equal
steps as well as equal results.

**Revisit if.** Producing the steps costs more than evaluation on a workload
that traces by default. None does; the cost is paid only by the caller that
asks.

### D-064 · Core completeness: ranges and lists first; CEL, IP and SNI deferred

**Decided.** The comparators added next are numeric ranges (xDS `Int64Range`,
`DoubleRange`) and list-of-string (`ListStringMatcher`). Deferred, with
reasons: CEL (`xds.type.matcher.v3.CelMatcher`) is a language with its own
runtime and dependency, not a comparator over erased data; IP/CIDR
(`IPMatcher`) and server name (`ServerNameMatcher`) are network vocabulary
that no current domain produces, and both are additive when one does.

**Why.** Core is complete when it compares everything the xDS matcher
vocabulary compares, over data an input produced; producing the data is the
input's job (the matcher-engine fence, CLAUDE.md, 2026-02-08). Ranges and
lists are comparisons with no dependency. CEL would add a dependency to a
crate whose only one is `regex`, and its expressions read the context
directly, which makes one thing an input and a matcher at once. Deferring is
not rejecting: each returns with a domain that needs it and a fixture that
shows it.

**What it entails.** Fixtures in `07_protojson/` for each comparator, ledger
all three. `MatchingData` is marked `#[non_exhaustive]` before publish, so the
planned `Vector` variant for semantic matching (0.1.x, with the determinism
rules for `f32` accumulation across native, wasm and PyO3) is additive.

**Revisit if.** A domain arrives with an IP or SNI input, or a user has a rule
that needs CEL and cannot be written as predicates over declared inputs. The
second is the interesting case: it may argue for a declared input, not for
CEL.

### D-065 · Matcher, action, processor

**Decided.** One mapping, so docs and code use one vocabulary:

- The **matcher** selects. Its output is an action it does not interpret. The
  generic `A` in `Matcher<Ctx, A>` stays opaque (the arch-guild fence,
  2026-02-08).
- The **action** is a policy: user-authored governance config plus a reference
  to the processor that applies it. Both travel as `TypedExtensionConfig`,
  resolved by type URL at load; `NamedAction.metadata` carries the processor
  reference (D-056).
- The **processor** executes. It is platform-administered (clients, pools,
  runtime), has its own configuration that never passes through xuma, and
  registers the policy types it accepts, so "which actions are legal here" is
  a property of the registry a slot resolves against.
- **Merging and overrides** are the control plane's; the data plane receives
  resolved config.
- A **policy engine** is built on the matcher, never inside it. `rumi run
  claude` is the first: a hook-event processor whose policy is allow or deny
  with a reason.

**Why.** This is how Envoy's matcher and `Composite` filter relate: the
matcher hands an opaque action to `onMatchCallback`, the consumer downcasts and
acts, factories run at load. It is also the definition the maintainer's own
archive held unchanged from 2025-01 to 2026-04 (two memex trawls,
2026-09-07). Recording it stops the drift in which "policy" names the matcher
in one document, the action in another and the processor in a third.
`NamedAction.metadata` is otherwise a field with no consumer, and "carry it or
delete it" was the fork; carrying it is what gives an action a processor.

**What it entails.** Docs say "matcher engine" and describe a policy engine
as a consumer. The slice return (D-056) is what a pipeline uses to want every
applicable policy later. No `Policy` type in core.

**Revisit if.** A second integration shows cross-domain policy pain that a
pipeline of matchers cannot express. That was the arch-guild's own revisit
condition, and it is unchanged.

### D-066 · The benchmark gate: no published number is false

**Decided.** The performance gate for publishing is not "beats pure
TypeScript", which no longer exists, but: every quantitative claim in tracked
markdown cites a benchmark artifact, and the artifact supports it. Protocol:
ten cold launches of `just bench-collect` at one commit on a clean tree (a
dirty flag rejects the artifact); per pair, the median per launch and the CV
over the ten medians; gate `G = max(1.20, 1 + 3·CV)`; any ratio inside
`[1/G, G]` is reported as indistinguishable; A/B interleaved, paired ratio
with a 90% bootstrap CI; artifact `bench/artifacts/<date>-<sha>.json` carrying
sha, dirty flag, hardware, cores, toolchain versions, build profile, and
`--release` asserted for PyO3. If CV exceeds 0.15 on a commit, publish the CV
and gate nothing. The maintainer's own Claude hooks configuration is a
committed workload (`bench/workloads/claude-hooks.json`), run in all three
runtimes.

**Why.** This release cycle has been about claims outrunning evidence
(`b68ca25f`: the measurement path could not have produced the claims it
justified). The 68x and 3.46x in D-051 exist only in prose; `bench/RESULTS.md`
is a stub; `bench/raw/` is empty. A gate on a comparison with a deleted
implementation is unmeetable; a gate on citation is checkable. The noise floor
comes first because a ratio without one is a number, not a measurement.

**What it entails.** `scripts/check-perf-claims.mjs` in `ci`: a sentence
matching a quantity or faster/slower/beats must carry `[bench:<artifact>]`;
the artifact exists, its sha resolves, and the number is within G of it.
Controls in `scripts/fixtures/perf-claims-{bad,good}/`; the bad set (uncited,
missing artifact, off by more than G) must fail. D-051's numbers are restated
with an artifact or deleted. Rollback: a post-release sentence failing the
check is deleted, not amended. `bench/RESULTS.md` returns under this protocol.
Roadmap phases 9 and 14 become ✅ when a slower pair fails CI against a
committed artifact, which is what the legend's "regressed" means.

**Revisit if.** CV stays above 0.15 on the CI hardware. Then the artifact is
produced on dedicated hardware and CI checks citation, not magnitude.

### D-067 · New vocabulary is defined here before it appears anywhere else

**Decided.** "Policy gradient", "planes of policy", "agent cooperative", and
any treatment of authorization as a matcher graph have no definition in the
maintainer's archive and none here. They appear in no doc, code or rustdoc
until an entry in this file defines them: what they name, and what they
exclude.

**Why.** Two memex trawls on 2026-09-07 found the processor and policy
vocabulary (D-065) stable for fifteen months and these terms absent. A word
coined mid-session reads as design once it lands on a page; six months of
documents inherited the three-engine reality that way (D-051). A term with no
definition cannot be checked for drift.

**What it entails.** The batch shape `MatcherSet.evaluate(ctx) ->
Vec<ActionIndex>`, which the proposal motivated as "planes of policy", is
described by what it does: one context against many matchers.

**Revisit if.** Defining one. That is the point.

---

## 2026-09-06 · One core

### D-051 · The duplicate engines are retired; `xuma` becomes the binding

**Decided.** `rumi` is the engine. Python and TypeScript get a binding plus a DX
layer over it, not a reimplementation. The pure-Python engine in `puma/` and the
pure-TypeScript engine in `bumi/` are debt and will be retired.

**Why.** One core with ports and adapters is what `CLAUDE.md`'s architecture
diagram has always described. The three-engine reality was never decided: on
`55df2d21` a roadmap table cell changed from "PyO3/maturin bindings" to "Pure
Python implementation" and the bindings moved to a new `p.uma[crusty]` slot,
which `c9afb7e0` then built. `a01790f0` renamed `puma`→`xuma` and
`puma-crusty`→`xuma-crust`, and the shipped names stopped showing any of it.
Six months of documents inherited the result as though it were the design.

The cost is measured, not theoretical. Three engines do not cross-check a
misreading of the spec — they triplicate it. D-050's fail-open was live in all
three compilers simultaneously, and the `#32` width limits reached Rust only.
Every spec change lands three times, and this repo has spent four PRs finding
places where it landed twice.

**Why now.** Nothing is published. Checked 2026-09-06: all nine names —
`rumi-core`, `rumi-proto`, `rumi-kv`, `rumi-http`, `rumi-cli` on crates.io;
`xuma` and `xuma-crust` on PyPI and npm — return 404. Retiring a package nobody
has installed costs nothing. After a release it costs a deprecation cycle, and
`xuma` on both registries is currently the *pure* implementation, so the rename
would be a breaking change to a published API rather than a bookkeeping one.

**What it entails, stated plainly rather than discovered later.**

The TypeScript binding is presently much slower than the engine it replaces:
after the `d006b241` boundary fix, pure TS still wins `exact_hit` 68x, and only
`regex_hit` goes to the binding (3.46x). So this decision makes the boundary
work **blocking**, not opportunistic — `xuma` on npm cannot ship at 68x slower
than the code it replaced. The known remaining items are typed `extern`
accessors with `final` in place of `Reflect::get`, `OnceCell` for the two
registries `fromConfig` rebuilds per call, and the returned-`String` transcode.
The floor is nowhere near reached: raw JS→wasm calls were measured at ~4.5ns in
2018 and we are at 780ns.

Python is not in the same position — the PyO3 binding already wins everywhere
except `compile_complex`, and by ~10x on regex.

The conformance suite changes in kind, not in value. "Five implementations
agree" becomes "the engine and its bindings agree", which is still a real check:
`F14` found `xuma.http.v1.PathInput` bound to two different semantics in Rust,
where both crusts took the path that made `/admin?x=1` match `{Exact: "/admin"}`
and the native path did not. Bindings diverge. The `implementations:` ledger
stays; its membership shrinks.

**What would justify revisiting.** A runtime where the binding cannot go —
an edge runtime that forbids WASM, or a Python environment that forbids native
extensions — carrying enough usage to be worth a second engine. That is a
distribution constraint, and it is the only argument that survives; "it is
faster today" is an argument for fixing the boundary, which is now the plan.

**Decided since, 2026-09-08.** The three questions this entry left open have
their answers in the batch above: `bumi/` and `puma/` are deleted, not frozen
(D-052); the order is retire, decide, rename, shape the boundary, publish,
then the engine (D-053); and `xuma` on npm ships as a thin host-language
package over the crust once the boundary work lands, never as the pure engine
in the interval (D-058).

---

## 2026-08-31 · The catch-all becomes a ceremony

### D-050 · A matcher that matches everything must be asked for by name

`and_predicate([], catch_all())` returns a catch-all, because an empty
conjunction is vacuously true. Same for the empty disjunction. So a rule with no
conditions, and a rule list with no rules, both compiled to "match everything"
in all three implementations. Pointed at an allowlist, `DELETE /etc/passwd`
returned ALLOW. The realistic way in is not writing `HttpRouteMatch::default()`
on purpose — it is a typo'd field name in YAML.

Both are now errors. `compile_catch_all` / `compileCatchAll` is the explicit
form, and `HookMatch` gains a `match_all` flag — pushing down the ceremony the
crusts have had since the security review, which guarded the FFI while the Rust
API underneath stayed open.

**Four things this turned up that are worth more than the fix.**

*The conformance suite asserted the bug.* `http_empty_routes_matches_all`
required every implementation to match everything on an empty list. So this was
not an oversight that survived the suite; it was a **contract the suite
enforced**, and each implementation was conformant precisely by failing open.
Now `http_empty_routes_is_refused`, which required teaching both HTTP runners
`expect_error` — with `error_contains` mandatory, the lesson the protojson
runner had already learned.

*It was never a spec divergence, in either direction.* The framing while fixing
it was "we diverge from Gateway API, which says no conditions = match all." xDS
says the opposite on the field itself: *"if no matcher above matched and this
field is not populated, the match will be considered unsuccessful."* And Envoy
enforces the same thing a layer up — `RouteMatch.path_specifier` carries
`option (validate.required) = true`
(`data-plane-api/envoy/config/route/v3/route_components.proto:622`), so a route
with no path specifier is a config error there too. The fix **aligns with the
reference implementation** rather than departing from a spec. The comments
asserting a Gateway API divergence were written, then corrected before they
became the record.

*The engine was right the whole time.* An empty `matcherList` loaded through the
registry falls to `on_no_match` — verified by execution. Only the convenience
layer disagreed with the engine underneath it, which is the inversion of where
you would look for it. The layer the project calls the door handle was the one
that failed open.

*It could not have been caught at load, and that is structural.* Substitution
destroys the evidence. Afterwards the predicate is `PrefixMatcher("")` and a
compiled catch-all is byte-for-byte a deliberate one. `Matcher::validate` sees a
valid matcher because it *is* a valid matcher. The only moment the mistake
exists as information is inside the compiler, at the substitution — which is why
the check lives there and nowhere else.

**Why not just copy the loader's semantics** (empty → no-match → `on_no_match`)?
Because there `on_no_match` is config the operator wrote, while in the compiler
it is an argument. `([], "allow", "deny")` and `([], "deny", "allow")` are
opposite outcomes from the same empty input; the library cannot pick the safe
one. Refusing is the only answer that does not guess.

**Revisit if:** a consumer has a legitimate need for a vacuous rule inside a
list — the `on_no_match` slot covers the default-route case today, and
`compile_catch_all` covers the rest.

---

## 2026-08-23 · Limits, and the module graph that was hiding them

### D-049 · Widths are validated by `Matcher.validate()` in all three implementations

The security review's F-02 named the rule: *the type that holds the resource
owns the limit on that resource.* #32 applied it in Rust by moving the width
checks from `Registry::load_*` onto `Matcher::validate`, so every construction
path — the domain compilers above all — inherited them.

puma and bumi were never carried across. Their `validate()` checked depth only;
`MAX_FIELD_MATCHERS` and `MAX_PREDICATES_PER_COMPOUND` stayed in the loader. A
257-route `compile_route_matches` was rejected by rumi and accepted by both
others, and nothing recorded the divergence. Fixed here, with regression tests
that each carry a not-inert control.

**Why it survived is the more useful part.** It was not an oversight of
intent — it was unreachable. `limits.ts` imported `MatcherError` from
`matcher.ts`, so `matcher.ts` could not import `limits.ts` back without a
cycle; puma had the mirror-image arrangement with `_registry`. The file that
needed the constants could not name them. So `MatcherError` and the two width
errors moved to `bumi/src/errors.ts` and to `puma/src/xuma/_matcher.py`
respectively, both re-exported from their old homes so no import breaks.

**The generalisation:** when a limit lives somewhere other than the type that
owns the resource, check whether the module graph is the reason before assuming
someone forgot. A cycle is a silent architectural argument against the correct
placement, and it wins by default.

**Revisit if:** a fourth implementation appears, or the limits gain a
per-deployment override — at which point they stop being constants and this
placement stops being obviously right.

---

## 2026-08-18 · Constructor naming

### D-047 · `Matcher::list`, not `Matcher::new`

D-044 made `Matcher` an xDS `oneof matcher_type` — a list of field matchers XOR
a lookup tree. `new` was left as the list constructor, which reads as *the* way
to build a `Matcher` and is true of neither half.

Renamed to `Matcher::list`, paired symmetrically with `Matcher::tree`.

Fifty-one call sites, of which twenty-two are benchmarks and twenty-three are
`matcher.rs`'s own tests; about five are production. All mechanical, all
compiler-verified. The change is only free before publish, and nothing is
published.

Two things the rename found that a sed alone would have missed, and which are
the reason the sweep goes wider than the source tree:

- `Self::new` inside `from_predicate`, invisible to a `Matcher::new` pattern.
- Four documents describing the API as it works *now* — `docs/content/concepts/pipeline.md`,
  the `rust-mastery` and `maintainer` skills, and `PLAN.md`. A skill still
  saying `Matcher::new` is worse than a stale comment: it is loaded as guidance
  and acted on.

Prose occurrences are deliberately **not** updated: they describe what was true
when written. A decision log is appended to, not rewritten; editing the name
inside one would falsify the record to tidy a grep.

**Correction, 2026-08-23.** This entry originally said "two occurrences" and
cited `DECISIONS.md:979`. Both were wrong, and were caught only when a skeptic
was asked to verify the correction rather than the claim. There are **six**
surviving prose references — `DECISIONS.md` 13/29/32/1011, `bench/RESULTS.md:222`,
and `CLAUDE.md:381` — and **line 979 does not contain `Matcher::new` at all**;
the intended line is 1011. The wrong citation reached this file, the #53 commit
message, and the maintainer. It is left visible here rather than silently
patched, because a decision log that quietly repairs its own errors is exactly
the unreliable narrator this repo keeps finding.

The successor has 51 `Matcher::list` references plus one `Self::list` at
`rumi/core/src/matcher.rs:137` — invisible to any `Matcher::` pattern, which is
the same trap this entry warns about two paragraphs up.

---

## 2026-08-18 · MatcherTree from config (SF3)

### D-044 · `Matcher` is a list XOR a tree, and the tree owns no fallback

xDS models `Matcher` as `oneof matcher_type { MatcherList, MatcherTree }` plus a
separate `on_no_match`. rumi modelled only the list, as a `pub matcher_list`
field, and `convert.rs` rejected `MatcherTree` wholesale.

Two shapes were considered. Letting a tree appear only under `OnMatch::Matcher`
was rejected as **not merely worse but unrepresentable**: `OnMatch::Matcher`
holds a `Matcher`, so a tree would need a third `OnMatch` variant — on the one
type whose stated job is mirroring a two-arm oneof — and that variant has no
protojson spelling. rumi would then load configs puma and bumi cannot express,
breaking the one artifact holding the five implementations together.

So `Matcher` carries `MatcherKind::{List, Tree}`. A closed enum over a closed
oneof, not a trait object: a third arm would be an upstream xDS change, and a
`Box<dyn>` dispatch point would invite matcher kinds no protojson can spell.

**`MatcherTree::on_no_match` is deleted.** The proto `MatcherTree` has exactly
two members, `input` and the `tree_type` oneof — there is no field to carry a
tree-level fallback, so it could never survive `convert.rs`. Keeping it would
have put two fallbacks on one condition with no stated precedence, recreating
one level up the ambiguity `OnMatch`'s enum exists to prevent.

Nothing is lost. Because `matcher_type` is a oneof, "the tree produced no
action" *is* "the enclosing `Matcher` produced no action". The one
distinguishable sub-case — a key hit whose nested matcher returned `None` — is
closable inside that entry, by giving its nested `Matcher` its own
`on_no_match`, exactly as the proto's own comment describes.

**Revisit if** `keep_matching` is ever implemented (PLAN.md F2/SF2). It
reintroduces "matched but continue", a third outcome, which could reopen the
split.

### D-046 · A tree lookup is one trace step, and does not fake a predicate

`EvalTrace.steps` became `EvalSteps::{List, Tree}`, mirroring `MatcherKind`. A
tree performs a map lookup, not a predicate evaluation, so there is no
`Predicate` to trace. Two dishonest options were rejected:

- **Synthesize a `PredicateTrace::Single`.** `rumi --trace` and the playground
  would then render a predicate that is not in the config. A trace that
  invents structure is a second, wrong source of truth.
- **Emit no steps.** That says nothing about the only thing that happened, and
  it silently invalidates the property that a non-fallback result implies at
  least one recorded step.

`TreeLookupTrace` records six facts, each corresponding to a distinct
misconfiguration a reader has to be able to tell apart: the rule applied, the
input, the extracted key, the entry key that won, what it dispatched to, and
whether the fallback was consulted. Separating `key: None` (the input produced
nothing usable as a string) from `key: Some(k), matched_key: None` (a real key
that found no entry) matters most — those are two different bugs that
previously collapsed into one silent miss.

`find_all_prefixes` is deliberately **not** used in the trace path. INV-7's
no-short-circuit rule is scoped to the predicate tree; a trace that performed
more lookups than `evaluate` would be an invitation for someone to later let it
influence the result. The winner is recorded, and nothing else.

**Across the FFI**, where the trace is a flat `{index, matched, predicate}`
list in both crusts, a tree maps to a single step with `index: 0`, `matched`
set from whether an entry was hit, and the lookup rendered into `predicate`.
The FFI shape is unchanged, so no consumer breaks. This was the one thing the
review of `EvalSteps` explicitly did not audit, and it did need handling — the
crusts failed to compile until it was done, which `just ci` does not catch
because it does not build them.

---

### D-045 · Making trees config-reachable required closing a stack-overflow path first

`MatcherTree` had no `depth()` and no `validate()`, and `Matcher::depth()` and
`validate_widths()` traversed only `matcher_list` and `on_no_match`. A tree
entry holds an `OnMatch`, which can hold a `Matcher`, which can hold another
tree. So `matcherTree -> onMatch.matcher -> matcherTree -> ...` nests without
bound, reports `depth() == 1`, passes `validate()`, and then overflows inside
the recursive `evaluate()`.

`MAX_DEPTH = 32` is the *only* thing preventing that, because iterative
evaluation is deferred to v0.2. Wiring SF3 without traversing tree entries would
have handed the bypass to anyone who can write a config — a config-triggerable
stack exhaustion behind the check that exists to prevent exactly it. Both
`depth()` and `validate_widths()` now recurse through `MatcherKind::Tree`.

**Width is deliberately not bounded by copying `MAX_FIELD_MATCHERS`.**
That limit exists because list evaluation is O(n) per request. Tree lookup is
O(1) hash or O(k) in the input key's length, so entry count does not affect
evaluation cost, and large routing tables are the whole reason trees exist.
A bound belongs here for config-parse memory, but with that rationale and its
own number — reusing 256 would freeze a limit whose stated reason does not
apply to it.

**Two silent-loss defects fixed in passing**, both of which would have frozen:
duplicate keys were dropped without a word (`HashMap` collect keeps the last;
`RadixTree::insert` discarded its `Option` return), and a tree whose input is
not a string can never match — now rejected at construction, per the
arch-guild rule that extension points are validated when built, not when
evaluated.

**Not a defect, checked rather than assumed:** an adversarially deep protojson
document does *not* exhaust the stack during `convert`. Every entry point routes
through `parse_matcher`, which calls `AnyResolver::pack`, which enforces
`MAX_JSON_DEPTH = 128` from depth 0 — so nothing recursive downstream ever sees
a deeper document.

---

## 2026-08-18 · Retiring the terse dialect, and checking the READMEs

### D-043 · A crate README's Rust blocks are doctests

Three crates ship a README with a Rust example. All three examples were wrong,
and had been for as long as they existed:

| Crate | Defect |
|---|---|
| `rumi-core` | showed the terse dialect, deserialized into `MatcherConfig` |
| `rumi-http` | referenced an undefined `config`, and used `?` in a non-`Result` scope |
| `rumi-kv` | same two errors as `rumi-http` |

None had ever compiled. `scripts/check-readme-agreement.mjs` covers only the
**root** README's `routes.yaml` block, so every crate README — the crates.io
front page, and the first thing a new reader copies — was outside CI's reach and
therefore unverified by construction.

Each crate now carries:

```rust
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;
```

`cargo test --doc` compiles and runs the blocks. Two of the three defects above
were found by adding this, not before it — the mechanism paid for itself inside
one commit.

Examples needing an optional feature are wrapped in `# #[cfg(feature = "…")] {`
so they compile under default features and actually run under `--all-features`,
which CI does via `just test-full`.

**Revisit if** a README grows an example that genuinely cannot be executed —
then mark that block `ignore` and say why in the block, rather than dropping the
wiring.

### D-042 · The tree-shape config types do not derive `Deserialize`

D-026 retired the terse JSON dialect in favour of canonical protojson. It stayed
*authorable* in Rust for months afterwards, because `MatcherConfig`,
`FieldMatcherConfig`, `PredicateConfig`, `SinglePredicateConfig`, `TypedConfig`
and `OnMatchConfig` still derived `Deserialize`. No fixture used it, no other
implementation could read it, and nothing documented it — but `serde_json::from_str::<MatcherConfig<String>>`
compiled, so it was still a format the crate offered.

puma and bumi had already closed this by construction: frozen dataclasses and
classes with constructors, reachable only through `parseProtojson()`. Rust was
the sole outlier.

Removing the impls makes writing the dialect a **compile error** rather than a
convention. The types remain — they are the intermediate representation
`Registry::load_matcher` consumes and `rumi-proto`'s converter produces, and
deleting them would require `rumi-core` to depend on `rumi-proto`, which is the
wrong direction. So Phase C's stated gate ("`grep -rn MatcherConfig` returns
only generated code") was never achievable and is restated: **no hand-written
config type is deserializable.**

Payload types keep their `Deserialize`. A `TypedConfig.config` body is
deserialized as the registered factory's `Config` associated type — that is the
extension seam working as designed, not a dialect.

What it cost: 66 call sites. 60 tests were rebuilt against struct literals, and
8 were deleted because their subject was the deserializer itself — 6 in
`config.rs` and the two `SinglePredicate` oneof tests in `registry.rs`. Neither
oneof requirement was dropped: `ValueMatchConfig` is an enum, so both-set is now
unrepresentable rather than rejected, and neither-set is caught by the converter
and fixtured in `08_invalid_configs.yaml` — five implementations assert it where
one used to. Test count went 329 → 321, exactly the 8 removed.

`rumi/core/benches/config.rs` was measuring the terse dialect's parse cost,
which was always cheaper than what any caller ran and, after this, measured
nothing that existed. It now walks the shipping path — `parse_matcher_str` then
`load_proto_matcher`. The honest numbers: ~6.7 µs to load a simple config,
against 73 ns to evaluate it, matching the 82 ns of a hand-built matcher.

**Revisit if** a second config dialect is ever genuinely wanted. It would need
its own reader producing these types, not a derive on them.

---

## 2026-08-18 · The core matcher registrations

### D-041 · `xuma.core.v1.StringMatcher` is not registered; `BoolMatcher` is, and now exists

`rumi info` advertised two matcher type URLs that **no config could load**.
Neither had a proto message, so `AnyResolver` could not resolve either, and a
`customMatch` naming one failed with "unknown type URL" — while the CLI listed
it as available. That is the advertises-what-it-cannot-do class this repo has
spent the release cycle removing, sitting in the tool's own `info` output.

They are resolved differently, because they are not the same mistake.

**`StringMatcher` is deregistered.** It was a second way to say what
`valueMatch` already says — xDS's own `StringMatcher`, on the
`singlePredicate` itself. `customMatch` is the seam for comparisons the oneof
*cannot* express; duplicating one it can is how a schema grows two spellings
for one thing. A custom domain that wants it can still register it, and
`load_custom_match_string_matcher` now does exactly that, which keeps the seam
tested while removing the shipped redundancy.

**`BoolMatcher` gets its message.** `valueMatch` compares strings only, so a
boolean genuinely has nowhere else to go — it is the case that justifies
`customMatch` existing. `proto/xuma/core/v1/matchers.proto` defines it, and
every resolver registers it.

**Three implementations disagreed about this, three different ways.** rumi
registered both; puma's `register_core_matchers` returned the builder unchanged
under a docstring claiming otherwise (PLAN.md F12); bumi had no such function
at all. And in puma neither domain called it, so fixing the body alone would
have changed nothing. All three now register `BoolMatcher`, from every domain.

**Revisit if** an input that produces `MatchingData::Bool` ships. That would
make a positive conformance fixture expressible, which today it is not.

### D-040 · Type compatibility is validated at load in all three implementations

`rumi` rejected an incompatible input/matcher pair at load — a `MapInput`
(string) against a `BoolMatcher` (bool) — while puma loaded the same config and
returned `None` at evaluation, and bumi had no check at all. One config, a load
error in one implementation and a rule that silently never fires in the other
two. That is the divergence the conformance suite exists to catch, in
`CLAUDE.md`'s own "validate extension points at construction" constraint.

All three now check it and produce the same message:
*input produces "string" data but matcher supports ['bool']*.

**The optional half is a separate protocol in puma and an optional method in
bumi, deliberately.** A Python `Protocol` member is *required* for structural
typing even when it has a default body, so folding `data_type` into `DataInput`
made every existing input stop satisfying it — 28 type errors. `TypedDataInput`
and `TypedInputMatcher` are separate protocols; absent means `"string"`, which
is rumi's default. bumi uses `dataType?()` for the same reason.

This is what made the `BoolMatcher` fixture expressible. It was in the
fixture-coverage gap list for exactly one commit, with the divergence named as
the reason it could not be written — the list working as intended rather than
as a place things go to be forgotten.

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
