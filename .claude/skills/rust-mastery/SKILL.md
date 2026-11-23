---
name: rust-mastery
description: >
  How to write Rust in x.uma, and above all how to build the boundary between the engine and
  its Python and TypeScript bindings. Load it when writing or reviewing code in rumi (the
  engine, its domain crates, the CLI) or in either crust (PyO3, wasm-bindgen); when designing a
  trait, an error type or a registry; when choosing or pinning a dependency; when a change
  crosses the FFI boundary; or before any performance work. Overrides generic Rust advice
  where x.uma's architecture needs something different. design-mastery covers what to expose
  and why; protocol-mastery covers proto and xDS; maintainer covers where code goes.
---

# Rust mastery for x.uma

x.uma is one Rust engine, `rumi`, with two thin bindings over it: `xuma-crust` for Python
(PyO3) and for JavaScript (wasm-bindgen). The engine is where the work happens. The bindings
exist to carry a config in and a result out, as cheaply and as faithfully as possible.

Most of what goes wrong in this repository goes wrong at that boundary, so most of this skill
is about it. The rest is the engine's design judgment and the obligations every change must
keep. The judgment comes from how mature Rust libraries solved the same problems and from this
repository's own mistakes; it is written so you can apply it without either.

Each principle gives what to do, why it holds, what it lets you catch, and why it makes
working with the maintainer better. Where generic Rust advice disagrees, x.uma's reasons win,
and the reasons tell you when they stop applying.

---

## 1. x.uma's posture, and where it departs from generic advice

| Generic advice | In x.uma | Why |
|---|---|---|
| Use `thiserror` or `anyhow` | Typed error enums, with `Display` and `Error` written by hand | The test is whether the caller needs to match on the error. For a library, and for anything crossing into a binding, it does, so variants are kept, never erased the way `anyhow` erases them. `thiserror` is a sound way to write typed errors; x.uma writes them by hand to keep the engine's dependency list short. That is a project choice, not a rule of Rust. |
| Generics everywhere for speed | `Box<dyn DataInput<Ctx>>` and `Box<dyn InputMatcher>` | Type erasure at the data level is what makes one matcher type serve every domain. Monomorphise at registration, erase behind a boxed closure. |
| Shared state behind `Arc<Mutex<_>>` | Immutable after build, no locks | `RegistryBuilder` becomes a frozen `Registry`. Thread safety comes from never mutating, not from synchronising. |
| Features that switch behaviour | Features that only add | A feature that changes what a type URL means once produced a fail-open. Features add capability; they never change semantics. |
| Async for I/O-shaped APIs | No async in the engine | Matching is synchronous, CPU-bound and short. Async would add cost and no benefit. |
| Pull in a crate for convenience | Almost never | The engine's few dependencies each earn their place: a linear-time regex crate, whose guarantee protects every user from runaway matching, and serde for config loading, on by default because loading configs is the engine's purpose. |

---

## 2. Engine design

### Core traits take `&self`

**Principle.** `DataInput::get` and `InputMatcher::matches` take `&self`, and so must anything
the engine calls on the hot path.

**Why.** With `&self`, a compiled matcher can be shared across threads and handed to a binding
behind an `Arc`, and forwarding impls for pointer types are possible. Today the engine provides
them for `Box<dyn …>`; adding `&T` or `Arc<T>` later stays possible only while the traits take
`&self`. `&mut self` breaks both: the bindings could no longer share a matcher.

**What it helps you catch.** A cache, counter or lazy field added to an input "for speed". Put
interior mutability somewhere that does not cost the sharing, or don't.

**Why it helps.** It is a hard line with a clear reason, so it ends the discussion instead of
starting one.

### Validate at construction; trust at evaluation

**Principle.** All checking happens when a config loads: depth, budgets, pattern lengths, type
compatibility. Evaluation is then infallible and never panics.

**Why.** A bad config should fail once, loudly, before it serves a request, not on the
thousandth request in production. That is how Envoy treats config, and it lets the hot path
skip every check.

**What it helps you catch.** Two paths with different guarantees. A matcher loaded through the
registry is validated; a matcher built by hand or by a domain compiler is only as safe as the
checks that path calls. Know which path you are on, and prefer making the unchecked path
impossible (see the next principle) over documenting it.

**Why it helps.** "This cannot fail at runtime" becomes a property of the types, which the
maintainer can trust without re-reading the evaluator.

### Make illegal states unrepresentable, in this order

**Principle.** When an invariant must hold, reach for these, strongest first:

1. **An enum instead of optional fields.** `OnMatch` is `Action(A)` or `Matcher(Box<…>)`, so
   "both" and "neither" cannot be written.
2. **Consume `self` to change state.** `RegistryBuilder::build(self)` makes "register after
   build" a move error. The general form is a typestate: `Thing<Unvalidated>` with a
   `validate(self) -> Result<Thing<Validated>, _>`, and the useful methods only on
   `Thing<Validated>`.
3. **A runtime check at one chokepoint.** The weakest, because it only holds on paths that go
   through it. Use it when the constraint depends on data and cannot be typed.

A comment saying "remember to" is not on the list.

**Why.** The higher mechanisms make the wrong code fail to compile; the lower ones rely on
every caller cooperating.

**What it helps you catch.** A new invariant enforced by a check that one constructor skips.

**Why it helps.** It gives a reviewer one question: which rung is this on, and why not higher?

### Monomorphise at registration, erase behind a closure

**Principle.** Generic code runs once, when a type is registered:
`builder.input::<PathInput>(type_url)`. After `build()`, the registry holds only boxed
factory closures, and loading a config involves no generics at all.

**Why.** It keeps compile time and binary size in check (one generic instantiation per
registered type, not per call site) and gives the bindings a single, non-generic entry point.
Where a generic function's body grows large, keep a thin generic wrapper that calls a
non-generic inner function.

**What it helps you catch.** A generic parameter leaking into the registry's public API, which
the bindings would then have to instantiate.

**Why it helps.** The registry stays boring, which is exactly what a seam that freezes at
publish should be.

### `MatchingData` is the contract between inputs and matchers

**Principle.** Every input produces `MatchingData`; every matcher consumes it. Never bypass it.
Absence is `None` from the input, not a value inside `MatchingData`.

**Why.** This indirection is what lets one `ExactMatcher` serve HTTP, Claude Code hooks and
plain maps alike. Its `Custom` variant exists for extension and should be used sparingly; it
carries `Arc<dyn CustomMatchData>` so values can be compared by identity and validated at load.

**What it helps you catch.** A domain-specific shortcut from an input straight to a matcher, or
a sentinel value standing in for "missing".

**Why it helps.** Adding a domain stays a matter of writing inputs, never of touching matchers.

### Two type parameters, no more

**Principle.** `Matcher<Ctx, A>` has a context and an action type. A third parameter that would
spread through every type in the stack should be erased at a boundary instead.

**Why.** Every type parameter on a central type infects everything that touches it. Mature
libraries have removed such parameters at real cost; adding one is far easier than removing it.

**What it helps you catch.** A "small" generic added for one domain's convenience.

**Why it helps.** It protects the public surface the maintainer is about to freeze.

### Measure before you optimise

**Principle.** No performance change without a measurement that shows the problem and a second
that shows the fix. Every performance number in tracked markdown cites a committed benchmark
artifact; there are no remembered or estimated numbers.

**Why.** The bottleneck is rarely where intuition puts it. In this repository the engine's own
work has been a small fraction of a binding call; most of the cost was the boundary. An
optimisation of the engine would have been invisible.

**What it helps you catch.** "This will be faster" without a profile or a benchmark, and any
performance figure written into docs or skills from memory.

**Why it helps.** The maintainer can accept or reject a performance change on evidence rather
than on how persuasive the description is.

---

## 3. Protocol obligations

Every change must keep these. Each one exists because breaking it is either a security hole or
a lie.

| # | Obligation | What breaks if it is violated |
|---|---|---|
| 1 | An input that returns `None` makes its predicate false. For an input marked `required`, `None` leaves the whole evaluation undecided: no action, and no fallback. | Missing data could match, which in a gate means allow. |
| 2 | First match wins. | A different action is returned. |
| 3 | A trace's result always equals the plain evaluation's result. | The debugging output lies. |
| 4 | `OnMatch` is an action or a nested matcher, never both. | Ambiguous semantics. |
| 5 | The registry is immutable after `build()`. | Unsynchronised mutation across threads. |
| 6 | Depth and per-evaluation budgets are enforced when the config loads. | Stack exhaustion or unbounded work from a hostile config. |
| 7 | A predicate trace records every child predicate; a matcher trace stops at the first match. | Hidden branches, or a trace that disagrees with obligation 2. |
| 8 | Regex uses the linear-time `regex` crate only. | A config can hang the process. |

The matcher-level trace stopping early is required by obligations 2 and 3, not an oversight.
Do not "restore symmetry" with the predicate trace.

---

## 4. The binding boundary

### A guarantee stops at the seam unless something on the far side checks it

**Principle.** Inside one Rust crate, the compiler checks your guarantees: types, lifetimes,
`Send`, exhaustive matches. The moment a value crosses into Python, JavaScript, a build tool
or a generated stub, that check is gone, and what replaces it is usually a sentence in a
README that no test reads. So every guarantee the bindings advertise needs an observer on the
far side: a test that runs in the foreign runtime and fails when the guarantee breaks.

**Why.** This is the most consistent finding across mature Rust interop libraries: at every
seam, a mechanically checked guarantee is replaced by prose. The drift that results takes
three forms, and they need different responses:
- **Nobody is watching.** Example: the PyO3 code and the maturin build disagree, and the drift
  is visible from inside neither project. Adding a check fixes this one.
- **A check fires but cannot say why.** Example: a schema hash mismatch that names no field.
- **Everything is present and still ambiguous.** Example: a missing case that could be
  deliberate or a bug, and the source does not say which.

The fix is not to formalise the seam in more types. It is to make the far side observable.

**What it helps you catch.**
- A wheel that builds but fails `import xuma_crust`.
- A `.pyi` or `.d.ts` that no longer matches what the module exports.
- A JavaScript result whose shape changed while every Rust test stayed green.

The conformance suite running through each binding is the main observer x.uma has. Generated
type stubs and the shape of JavaScript results need their own.

**Why it helps.** "The bindings behave like the engine" becomes something CI proves on every
change, not something the maintainer has to take on trust.

### The binding interprets nothing

**Principle.** A binding converts a config document and a context in, and a result out. It
does not decide what a config means, and it holds no matching logic.

**Why.** Every line of logic in a binding is a second implementation that can drift from the
engine. This repository retired two complete reimplementations of the engine for exactly that
reason: every spec change had to be made three times, and the copies disagreed.

**What it helps you catch.** Validation, defaulting or domain knowledge creeping into a crust.
If a binding needs to know something, the engine should expose it.

**Why it helps.** One conformance suite, run through every binding, then proves all three agree,
and the maintainer has one place to look when they don't.

### A type URL means the same thing in every binding

**Principle.** The engine and both bindings resolve each type URL through the same
registration. Never register a type URL a second way for one runtime.

**Why.** This repository once had `PathInput` registered twice with different semantics: the
native path stripped the query string and the bindings did not, so `/admin?x=1` matched
`{Exact: "/admin"}` in Rust and failed to match through the wheel. For a path gate that is
fail-open, and nothing reported it. Two registrations for one type URL is the mechanism; one
registration removes the class of defect.

**What it helps you catch.** A "simple" registration helper for the bindings, a per-runtime
feature flag that swaps an input's implementation, or a crust-local input type.

**Why it helps.** "Do the bindings behave like the engine?" becomes a structural fact, not a
test-coverage question.

### Normalise a type URL once, and compare names

**Principle.** Strip the `type.googleapis.com/` prefix (or any prefix up to the last `/`) at
one boundary and compare the fully qualified message name. Never compare raw type URL strings.

**Why.** The same message has more than one valid type URL spelling, and widely used protobuf
libraries emit different ones. Comparing strings makes two correct configs disagree.

**What it helps you catch.** A registry keyed by whatever string the caller passed.

**Why it helps.** It removes a class of "works with one tool, fails with another" reports
before they exist.

### Constants and limits come from the engine

**Principle.** A binding imports every limit it enforces from `rumi` (pattern lengths, depth,
budgets). It never declares its own copy.

**Why.** A copied constant has no compile-time link to the original. When the engine's limit
changes, the copy silently keeps the old one, and the binding accepts or rejects configs the
engine would not.

**What it helps you catch.** A literal number in a crust that also appears in core.

**Why it helps.** Changing a limit stays a one-line change the maintainer can review in one
place.

### The boundary costs more than the match

**Principle.** Design the binding API so one match is one crossing. Pass the context in one
call, return one result object, and never let the host walk the engine's structures. Inside the
crossing, avoid per-call work that does not depend on the input.

**Why.** In this repository, a single wasm `evaluate` once spent most of its time re-creating
six constant property-name strings on every call, each one allocated in JavaScript and
transcoded from UTF-8 to UTF-16, while the engine's own work was a small fraction of the total.
wasm-bindgen's interning feature did not help, because `JsValue::from_str` bypasses the intern
cache; the fix was to create the keys once and reuse them.

Two more costs belong here:
- **Per-call values cross through the runtime's own object API, not through JSON text.** Both
  the PyO3 and the wasm-bindgen ecosystems moved away from serialising to a JSON string and
  parsing it on the other side, because the parse costs more than the call. A config is
  different: it is loaded once, and protojson text is its wire format.
- **Zero-copy depends on the path.** Borrowed slices can cross without copying. Owned
  transfers, round trips of mutable slices, and anything built from `JsValue`s copy or
  allocate. Do not assume a crossing is free because one path is.

**What it helps you catch.**
- A chatty API: one call per field, per rule or per step.
- Values rebuilt on every call that could be built once.
- A per-call context passed as a JSON string.

**Why it helps.** It points performance work at where the cost actually is, which a benchmark
of the engine alone would never show.

### Say what unit a length or offset is in

**Principle.** Any length, limit or position that crosses the boundary states its unit: bytes,
UTF-16 code units, or characters.

**Why.** Rust strings are UTF-8 and measured in bytes. JavaScript strings are UTF-16, and
`.length` counts code units. Python counts code points. The same text has three different
lengths. Two independent collaborative-editing engines hit this as an interoperability bug:
their Rust and JavaScript builds disagreed on positions, and the two sides had to agree on one
text encoding before positions meant anything.

**What it helps you catch.** A pattern-length limit that a JavaScript user reads as characters
but the engine enforces in bytes, and an error message reporting a byte offset that a Python
user treats as an index.

**Why it helps.** It takes "my 4000-character pattern was rejected as too long" off the list
of reports the maintainer will get.

### Keep the representation inside the binding

**Principle.** The host names context values by meaning (slot or field names). How the binding
packs them for the engine is the binding's business, never part of its API.

**Why.** A packed layout exposed to hosts freezes the first encoding chosen. Kept inside, it can
be changed in a patch release.

**What it helps you catch.** An API that asks Python or JavaScript for slot indices, a byte
buffer or a field order.

**Why it helps.** Speed work stays reversible.

### Errors cross as types

**Principle.** Each engine error kind becomes a distinct exception type in Python and a
distinguishable error in JavaScript, carrying its details as fields. No caller should parse a
message to learn what went wrong.

**Why.** A binding that turns every error into `ValueError(str)` or `JsError(string)` makes the
message text the contract, which can then never be improved.

**What it helps you catch.** `PyValueError::new_err(format!(…))` and `JsError::new(&format!(…))`
on any path a caller might branch on.

**Why it helps.** Users get a table of failure modes; the maintainer gets error messages they
are free to reword.

### Never panic across the boundary; fail closed at the host

**Principle.** Invalid input produces an error, not a panic. Treat any panic reaching a binding
as a bug. Where a host reads "no answer" as "yes", the adapter answers "no" on every failure.

**Why.** The two runtimes handle panics differently. PyO3 turns a Rust panic into a
`PanicException` that ordinary `except Exception` does not catch. In wasm a panic is a trap
that aborts the call, and the module's state afterwards should be treated as unreliable. Either
way the host gets something it was not designed to handle. Separately, some hosts treat an
error as permission: Claude Code lets a tool call proceed on most failure exit codes, which is
why the hook adapter uses its block code for every failure.

**What it helps you catch.** `unwrap()` or `expect()` on data the host supplied, and any
adapter whose error path a host would read as success.

**Why it helps.** "It cannot fail open" stays true at the edge, where it matters.

### Thread safety at the boundary

**Principle.** Leave PyO3's default on: a `#[pyclass]` must be `Send + Sync`, and PyO3 checks
this at compile time. Never use `unsendable`. Make result and matcher classes `frozen`.

**Why.** `unsendable` replaces the compile-time check with a runtime panic when the object is
touched from another thread. `frozen` removes the runtime borrow tracking for objects that
never change, which x.uma's never do. What is genuinely not `Send` is `Python<'py>`, the
token proving the GIL is held, so never store it. In wasm, `JsValue` is only non-`Send` when
atomics (threads) are enabled, so the wasm crust's assumptions change if threaded wasm is ever
added.

Whether to release the GIL during a match is a measurement question. A match is short, and
releasing and reacquiring the GIL has its own cost; do not add it without a benchmark showing a
multi-threaded host gains.

**What it helps you catch.** An `unsendable` added to silence a compile error (fix the type
instead), and a `Python<'py>` stored in a struct.

**Why it helps.** The thread-safety guarantee the bindings advertise stays a compiler fact.

### Know your serializer's defaults at the wasm boundary

**Principle.** Where a value's JavaScript shape is part of the contract, serialize it with
`serde_wasm_bindgen::Serializer::json_compatible()`, not the default `to_value`.

**Why.** The default serializer turns a Rust map into a JavaScript `Map`, not a plain object
(so `obj.key` is `undefined` and `JSON.stringify` drops it), turns `None` and `()` into
`undefined` rather than `null`, and turns byte slices into `Uint8Array`. The
`json_compatible()` preset changes all three. Integers beyond JavaScript's safe range still do
not round-trip. A Rust-side round-trip test will not catch any of this, because the
deserializer accepts both shapes. Test the `JsValue` itself: assert it is not a `Map`, and that
the fields a caller reads are present.

**What it helps you catch.** A result or trace object whose missing fields arrive as
`undefined` in one place and `null` in another, and any map crossing to JavaScript through the
default serializer.

**Why it helps.** The frozen result object has one shape in JavaScript, and a test proves it.

### Pin by what the coupling is

**Principle.** Pin `wasm-bindgen` exactly, and build the library and its CLI from one tree.
Keep `pyo3` on a minor-version range and bump it together with maturin. Make the wheel's
declared Python range match what is actually built: either build one wheel per supported
Python version, or build against the stable ABI (`abi3`).

**Why.** The wasm-bindgen library and the wasm-bindgen CLI agree through a versioned binary
schema, and their versions must match. `wasm-pack` does not check that the version in
`Cargo.lock` matches the CLI it runs. `serde-wasm-bindgen` itself pins wasm-bindgen exactly,
for the same reason. An exact pin rules the mismatch out instead of leaving you to diagnose
it. PyO3's generated code targets PyO3's public API, so a mismatch there is a compile
error, not a silent fault. A wheel built for one interpreter but declaring support for several
installs and then fails to import.

**What it helps you catch.** A caret `wasm-bindgen = "0.2"`, and a `requires-python` that
promises more versions than the release matrix builds.

**Why it helps.** Install failures are caught by the release rehearsal instead of by users.

### Register explicitly; distributed registration does not survive wasm

**Principle.** Extension types are registered by explicit calls at startup. Do not use
link-time distributed registration (`inventory`, `linkme`, `typetag`) for anything the wasm
crust needs. If such a feature is ever added, make it a compile error on `wasm32`.

**Why.** Those crates rely on static initialisers or linker sections. On `wasm32` the
initialisers are not reliably run, so the registry can come up silently empty: every lookup
fails, and nothing fails to compile. The explicit builder is the wasm-safe design; its
explicitness is the safety property.

**What it helps you catch.** A proposal to make domains "self-register" as part of the
registration seam, without saying how the wasm crust gets them.

**Why it helps.** A whole class of "works in Python, empty in the browser" bugs never ships.

### Extension-module build mechanics

- PyO3's `extension-module` feature stops the crate linking against libpython, so plain
  `cargo test` cannot run the Python crust's tests. The workspace's `default-members` leave the
  crusts out; test them through `maturin develop` and pytest, and the wasm crust through its
  own test runner.
- The Python crust is `["cdylib", "lib"]`; the wasm crust is `cdylib` only, because nothing
  depends on it as a Rust library.
- Both crusts are `publish = false` as Cargo crates, which is not the same as unpublished: they
  ship to PyPI and npm through maturin and wasm-pack.
- The wasm module must be initialised (`await init()`) before first use; document it wherever
  the TypeScript API is shown.

---

## 5. Errors, dependencies and the CLI

### Hand-written errors that stay useful

**Principle.** Errors are enums with hand-written `Display` and `Error`. A variant that wraps
another error returns it from `source()`. Every error enum a caller can match on is
`#[non_exhaustive]`.

**Why.** Without `source()`, a handler that prints the chain loses the cause. Without
`#[non_exhaustive]`, adding an error kind after publish breaks every caller that matched
exhaustively.

The rule points both ways. Mark the enums *callers* receive `#[non_exhaustive]`, so they can
grow. *Inside* the workspace, match every variant explicitly and avoid `_ =>`. An exhaustive
match does not make the code more complete, but it makes the gaps readable: when a variant is
added, the compiler lists every place that has not decided what to do with it.

**What it helps you catch.**
- An error type with a field named `source` and no `source()` impl.
- A public error enum without `#[non_exhaustive]`.
- A wildcard arm inside the engine that would silently absorb a new variant.

**Why it helps.** Error kinds can grow after the freeze without a major release, and nothing
inside the engine forgets to handle them.

### Adding a dependency

**Principle.** Assume the answer is no. A dependency in `rumi-core` needs a guarantee nobody
can hand-write cheaply, the way the regex crate's linear-time matching is one. Domain crates
may carry heavier dependencies that only their users need.

**Why.** A dependency in the engine is exposure for every user and every binding, and its
breaking releases become yours. Foundation crates should change last.

**What it helps you catch.** A utility crate added to save twenty lines.

**Why it helps.** The maintainer reviews a guarantee, not a convenience.

### The CLI

**Principle.** The library prints nothing. In `rumi`, results go to stdout and commentary to
stderr. The exit code is decided in exactly one place. A closed output pipe (`rumi … | head`)
ends the program quietly, not with a panic.

**Why.** Exit codes and output are contract for scripts and for hosts like Claude Code. When
the code is set in several places, two failures can exit differently, and a host reading the
code cannot tell them apart.

**What it helps you catch.** `std::process::exit` called from more than one path, diagnostics
printed to stdout, and output written with `println!` where a broken pipe would panic.

**Why it helps.** A host's integration can be specified as a table of exit codes, and trusted.

---

## 6. Verification habits

- **Anything CI does not run is unverified.** A phase of this project was marked done for
  months while one of its crates had never compiled, because nothing built it. If you add code
  CI cannot reach, say so, and add the CI step.
- **Lint and format before committing:** clippy with `-W clippy::pedantic`, with `--fix`, then
  `cargo fmt`.
- **A search that finds nothing needs a control.** A failed command and an empty result look
  the same. Before claiming something does not exist, show the same search finding something
  that does.
- **Fix the code, not the test.** When a conformance fixture and a security property disagree,
  the fixture is the thing that is wrong.

---

## 7. What this skill does not know

Silence here is absence of evidence, not a default.

- **Semver tooling.** The discipline of not breaking published APIs is well established; which
  tool enforces it best for this workspace (`cargo-semver-checks`, `cargo-public-api`) is
  untested here.
- **Test harnesses** for snapshot, property and wasm testing have not been compared for this
  repository.
- **Bundle size.** The workspace has no `[profile.release]` tuned for the wasm module, and its
  size has not been measured. Measure before changing settings such as `panic = "abort"`,
  which also changes how the error path behaves.
- **Preserving unknown fields** on a round trip, as opposed to accepting or rejecting them, has
  no worked precedent here.
