---
name: design-mastery
description: >
  Design judgment for anything in x.uma that other people will build on. Load it before
  changing or adding something that freezes at publish: the config schema, a type URL, a
  binding's classes or methods, the result object, an error type, a CLI command, flag or exit
  code, a public trait. Load it too when adding an input, action or domain, deciding whether
  something belongs in the engine or in an extension, deciding how a receiver treats input it
  does not recognise, or reviewing a design proposal. Covers the one-way door of publishing,
  designing around likely change, extension points, core versus periphery, and what happens at
  a design's boundaries. rust-mastery covers how to write it in Rust; protocol-mastery covers
  proto and xDS; maintainer covers where code goes.
---

# Design mastery

This skill is about the decisions that are expensive to take back. x.uma is a matcher engine
with two language bindings, a CLI and a config format, and once it is published every one of
those becomes something strangers depend on. Most of the judgment below comes from a small body
of design literature that has been checked against its sources, and from mistakes this
repository has already made. It is written so you can apply it without reading either.

Read it as reasons, not rules. Each principle says what it is, why it holds, what it lets you
catch, and why it makes working with the maintainer better. When a principle and the situation
disagree, the reasons tell you which way to bend. When you bend one, write down why.

A note on strength. Almost everything here is experienced practitioners' judgment, not
measurement, and its authors said so. Keep the hedges they kept. Where this skill says "tends
to" or "usually", it means it.

---

## 1. Publishing is a one-way door

### You can always add; you can never remove

**Principle.** Until first publish, every name, type URL, field, command, flag, exit code and
error is free to change. After it, each one is close to permanent. When in doubt, leave it out.

**Why.** Removing something breaks whoever used it, and you cannot know who did. Removal is
still possible, but only by a signalled path: deprecate in a visible release, wait a stated
period, remove in a major version. Adding is cheaper, but it is not free either. An addition is
compatible only if code that knows nothing of it behaves exactly as before:
- no new required fields;
- defaults that reproduce the old behaviour;
- care with new enum values, which break callers that matched every case.

So the cost of an omission is a later feature request. The cost of an inclusion is a
deprecation cycle at best.

**What it helps you catch.** "Let's expose it while we're here." A method on a binding, a CLI
flag for a niche case, a config field "for flexibility". Each needs a user before it ships.

**Why it helps the collaboration.** Proposing the smaller surface, and naming what you left out
and why, gives the maintainer a decision they can make in one read instead of a cleanup they
inherit after the freeze.

### Every behaviour someone can observe becomes contract

**Principle.** With enough users, whatever your interface does will be depended on, whether you
promised it or not: ordering, timing, error wording, which field comes first, what happens on
an empty input.

**Why.** Users build on what they see, not on what you wrote down. Agent callers make this
stronger: a model reads every error message and every `detail` field, and acts on it.

**What it helps you catch.** Unspecified behaviour that is about to freeze by accident: an
error string a caller could parse, an iteration order that happens to be stable, a trace that
lists steps in an undocumented order. Either specify it, or make it visibly unspecified.

**Why it helps.** It moves the conversation from "nobody should rely on that" to "here is what
we promise", which is the only form of the question the maintainer can actually rule on.

### An interface is every assumption, not the signature

**Principle.** The interface between two parts is everything each side must assume to be
correct: types, yes, but also limits, ordering, what "no answer" means, and what happens on
failure.

**Why.** Two components can agree on every type and still be wrong about each other. This
repository has a live example. The Claude Code hook mode exits 0 to mean "allow", and its docs
say so. Claude Code's own contract says exit 0 with no output means "no decision". So the hook
can block a call but cannot approve one. The types are fine; the assumption is wrong. Until the
hook adapter is changed, treat its "allow" as "no objection".

**What it helps you catch.** Before wiring x.uma to a host (Claude Code, Envoy, a Python
program), read the host's contract for silence, errors and partial results, not only its
input shape. Before publishing an x.uma surface, write down its limits, budgets and failure
behaviour. If you cannot, the interface is not finished.

**Why it helps.** A contract page the maintainer can read turns "I think it works like this"
into something they can check, and is where the next disagreement gets settled.

### Failure is part of the contract

**Principle.** What a component does when it cannot do its job is as much a promise as what it
does when it can. Fail visibly. A handler that swallows an error and returns normally deceives
its caller. Where "I could not tell" would be read as "yes", fail closed.

**Why.** Silent failure moves the error somewhere it is harder to find. In a matcher used as a
gate, it is worse: a missing value that quietly evaluates to "allow" is a security hole.

**What it helps you catch.** Every place absence, malformation or an internal error could fall
through to a success value. In x.uma: a `DataInput` returning `None` makes its predicate false,
a `required` input that is absent leaves the result undecided rather than falling back, and the
hook exits with its block code on every failure, because Claude Code lets a call proceed on any
other failure code.

**Why it helps.** The maintainer can trust a "this cannot fail open" claim only if each path is
named. Listing them is the evidence.

### Errors have identity, not just text

**Principle.** Give every error a kind a program can branch on, with its details as fields. No
caller should need to parse a message.

**Why.** If the only way to tell two errors apart is their wording, the wording is the
contract, and you can never improve it. Structured kinds also let a binding raise a distinct
exception per kind instead of one generic error.

**What it helps you catch.** `ValueError(str)`, `JsError::new(format!(...))`, a CLI that
reports every failure the same way. Each is a surface that freezes the message.

**Why it helps.** It gives users, and the maintainer reviewing, one table of what can go
wrong, instead of a search through strings.

### A list the caller receives must be able to grow

**Principle.** Any enumeration a caller matches on (result states, error kinds, trace step
kinds, decisions) will gain members. Design it so that adding one does not break callers who
handled all of today's.

**Why.** An exhaustive match on a published enum is a promise that the set is closed. You will
want to break that promise the first time a host adds a case. In Rust, mark such enums
`#[non_exhaustive]`; in the bindings, document that unknown values can appear.

**What it helps you catch.** A closed list that mirrors something outside your control. The
Claude domain hard-codes nine hook events. Claude Code has dozens, and an unknown one makes the
hook block. The way out is to stop enumerating: read the payload's fields by name, the way a
header matcher reads a header, so new events and fields need no release.

**Why it helps.** It saves the maintainer from a breaking release whose only cause was the
outside world moving.

---

## 2. Design around the changes you expect

### Decompose by the decisions likely to change

**Principle.** Start a design from the list of decisions that are hard or likely to change, and
give each one a single home that hides it. Interfaces carry only what is unlikely to change.

**Why.** A change that touches one module is cheap; a change that touches an interface touches
everyone. You cannot make all changes cheap, but you can choose which ones are, and that choice
is the design.

**What it helps you catch.** A proposal shaped by today's data flow rather than tomorrow's
changes. For x.uma, ask of every surface: what will a host add next year? New Claude Code
fields, new HTTP inputs, new action kinds, a third binding. Each should land without touching
the engine's interfaces. That is why domains plug in at `TypedExtensionConfig`. Apply the same
test to the bindings: a design in which each new domain needs a new class in every binding
fails it.

**Why it helps.** It turns "is this design good?" into "which changes does it make cheap?",
which the maintainer can answer from what they know about where x.uma is going.

### Keep the representation inside

**Principle.** How data is laid out internally is a decision likely to change. Do not let it
become an interface. Name things by what they mean, and keep the encoding private.

**Why.** An exposed layout freezes the first encoding you chose, which is rarely the fastest or
the clearest. Hidden, it can be tuned in a patch release.

**What it helps you catch.** A binding API that asks the host for a packed buffer, slot
indices or an internal ordering. The host should name slots by meaning; packing is the
binding's business.

**Why it helps.** Performance work stays reversible, so the maintainer can say yes to it
without a design review.

### Hide what is undesirable, not what is powerful

**Principle.** Hide the details users should not depend on. Do not bury the capabilities they
need.

**Why.** "Hide everything" produces an interface so thin that users route around it, and then
depend on the workaround. The point of hiding is to protect a decision, not to withhold power.

**What it helps you catch.** Removing the trace because it exposes internals: the trace is how
users debug a match, so define what it promises instead. Or exposing the internal IR because a
power user asked: expose the capability they need, not the representation.

**Why it helps.** It gives a direction when "simpler" and "more capable" pull against each
other. It is not a decision rule: the tension between keeping a secret and exposing power is
named in the literature, but no source says how to settle it. Decide case by case, and write
down the reason.

### Simple means not tangled

**Principle.** A design is simple when its concerns are not interleaved, not when it is short
or easy to type. Two small things braided together are less simple than one larger thing that
stands alone.

**Why.** Interleaved concerns change together, so every change to one is a change to the other.
That is where the cost of maintenance comes from.

**What it helps you catch.** A CLI command whose behaviour flips on a flag between two jobs.
`rumi run claude` with and without `--stdin` is one name for "be a hook" and "try a rule by
hand", and the flag form does not work as a hook. Two concerns, one name.

**Why it helps.** It names what feels wrong about a design in terms the maintainer can test:
what changes together?

---

## 3. Extension points

### Do not freeze an extension point no one has used

**Principle.** Before an extension point is published, independent users should have built on
it. The usual form of the rule is three: write one and it probably will not support another;
write two and it will support more, with difficulty; write three and it will work. Two is the
floor, not the target.

**Why.** The author of an extension point cannot see the assumptions it bakes in, because they
share them. Independent users are the cheapest way to find them, and they are only cheap
before the freeze.

**What it helps you catch.** A registration seam, a binding surface or a context format that
only its own tests exercise. A conformance suite proves the engine agrees with itself; it does
not prove a stranger can build on it. Look for one consumer of the client side (a host that
loads configs and matches) and one of the provider side (an integration that registers its own
inputs or actions). Until the provider side has one, it has been used only by its author. One
consumer per side is short of the rule. If that is all there is at the freeze, say so in the
release decision rather than calling the seam proven.

**Why it helps.** It gives the maintainer a concrete gate ("consumers green") instead of a
judgment call about readiness, and it is honest about how much the gate proves.

### Design extensibility in, with explicit rules for what is new

**Principle.** A good extension mechanism says, in advance, how a receiver handles a new field,
case or version, and how it handles input that is malformed. Then new elements are expected,
not tolerated.

**Why.** Extensibility and leniency are different things. "Accept whatever comes and hope" is
not an extension strategy, and retrofitting one later is harder than designing it in. A
mechanism with clear rules lets a new feature ship knowing its effect on existing receivers.

**What it helps you catch.** Every place a new value might arrive: an unknown type URL, an
unknown payload field, a new hook event, a newer config version. Each needs a stated rule:
ignore, read through a generic path, or reject.

**Why it helps.** It replaces "what happens if…?" in review with a rule anyone can look up.

### Tolerate what was designed in; refuse what is wrong

**Principle.** Be accepting of variation the design anticipated: new fields read by path, new
events matched by name, extension types resolved by URL. Be strict about input that is actually
faulty: malformed documents, invalid configs, values outside their type. Silently accepting
faulty input is almost always the wrong response, even when the specification is imperfect.

**Why.** Leniency toward errors feels kind, but it has a feedback cycle. Others start sending
the faulty input because it works; the error becomes entrenched; eventually every
implementation must reproduce the flaw to interoperate. The honest response to a broken input
is to reject it visibly and fix the source. There are bounded exceptions: early in a design's
life, or where receivers genuinely cannot be updated, temporary tolerance can be justified, and
should be written down as temporary.

Rejecting unknown input has a consequence you must state: it fixes the upgrade order. If an
x.uma config loader rejects fields it does not know, then a config written for a newer
version fails to load on an older one, so readers must upgrade before writers. That is the
right trade for a security matcher, because failing to load is loud and a silently ignored rule
is not. But it is a decision, and users need to be told it.

**What it helps you catch.** A config loader that silently ignores an unknown field, or rejects
one without the upgrade order being documented. A hook that treats an unparseable payload as
"allow". A fixture that requires every
implementation to accept a fail-open: this repository once had a conformance test requiring an
empty route list to match everything, so every implementation was conformant by failing open.
When a test and a safety property disagree, the test is wrong.

**Why it helps.** It gives one rule for two opposite instincts, "be robust" and "be strict", so
the maintainer is not asked to choose between them case by case.

### Exercise your extension points

**Principle.** An extension point that only ever sees today's values stops working for
tomorrow's, because the code around it quietly comes to assume today's set. Test it with values
it has never seen.

**Why.** This follows from the two principles above: what users observe becomes contract, and
tolerance that is never tested decays. It is this skill's inference from them, not a claim from
the sources behind it.

**What it helps you catch.** An input that reads a host's payload by field name and works for
every field the host sends today, but has never been fed a field rumi has never heard of. A
registry that has only ever loaded built-in types.

**Why it helps.** It turns "it should handle new fields" into a test that fails if it doesn't.

### Know which kind of growth your design makes cheap

**Principle.** A system can make it cheap to add new cases (new inputs, new actions, new
domains) or cheap to add new operations over all cases (new verbs). It is hard to make both
cheap at once.

**Why.** Cases and operations form a table. One axis is usually open and the other closed,
and the open axis is the one you built for. Growth along the closed axis touches every existing
case.

**What it helps you catch.** x.uma makes cases cheap: a new input or action is a type URL and a
factory. So a new operation over every input, such as `reads()` to name the slots a config
reads, or a new trace step kind, touches every implementation. Those are the operations to get
in before the freeze, because after it each one is a breaking change to every provider.

**Why it helps.** It tells the maintainer which proposals are cheap after publish and which
must land before it, which is the question the release plan turns on.

### A substitute must keep the contract, not just the signature

**Principle.** One implementation can stand in for another only if everything provable about
the original still holds for the substitute: its invariants, its history, the errors it can
raise. Matching signatures proves only that nothing will fail to type-check.

**Why.** Two components can share every method and still be unusable in each other's place. A
substitute that raises an error the caller has never heard of breaks a caller written against
the original. A substitute that does something the original never could breaks callers who
relied on it never happening.

**What it helps you catch.** A `DataInput` that returns a value where the contract says absent.
A binding that registers a type URL with a different meaning from the engine. This repository
had `PathInput` bound to two meanings, so `/admin?x=1` matched `{Exact: "/admin"}` natively and
did not match through the wheel. For a path gate, that is fail-open. A type URL must mean one
thing everywhere.

**Why it helps.** It makes "does this extension behave?" checkable by conformance fixtures run
in every implementation, instead of by inspection.

---

## 4. Core and periphery

### The core supplies mechanism; policy lives above it

**Principle.** The engine selects. It does not decide what a selection means. Allow and deny,
routing and logging are policies, and they live in actions and the processors that apply them.

**Why.** A policy built into the core works for the average case and fails the special cases
users care about. Policies outside the core are easier to adapt, and different users can run
different policies over the same engine at the same time. The generic action type in
`Matcher<Ctx, A>` is the fence that keeps this true.

**What it helps you catch.** Any proposal that teaches the engine what an action means: a
"Policy" type in core, an engine-level notion of deny, a decision vocabulary inside the
matcher. A host's decision vocabulary (Claude Code's allow, deny, ask, defer) belongs in that
domain's action type and its adapter, not in the engine.

**Why it helps.** "Matcher engine, not policy engine" is a standing decision. Holding the fence
keeps the maintainer from re-arguing it every time a domain wants a shortcut.

### Separation is a judgment with costs; admit only what must be inside

**Principle.** Admit something to the core only if it cannot be done properly outside it. Even
the strongest advocates of separation kept some policy in the kernel, and said why: to
arbitrate fairly between competing parts that do not trust each other, and, in one paging
design, because moving the decision out would leave a shared resource under-used.

**Why.** Separation has costs: indirection, crossing a boundary, a harder interface to get
right. Principled designs pay them by default and record their exceptions. The literature
offers two admission tests, and they disagree:
- **The functionality test:** something belongs inside only if moving it out would prevent
  something the system must do, such as letting competing implementations coexist.
- **The efficiency test:** most kernel designs also admit things for speed.

Apply the first by default. Admit by the second only with a measurement.

**What it helps you catch.** Both errors. Pulling the hot path out of the engine for purity,
when the measured cost says otherwise. And letting domain logic drift into the core because it
was convenient. Performance is a reason to keep something inside only when it has been
measured.

**Why it helps.** It gives the maintainer a test to apply rather than a slogan to defend, and
makes the exceptions visible as decisions.

### The core is everyone's single point of failure

**Principle.** Everything built on the engine relies on all of it. Every line added to the
core is a line every user is exposed to. An extension outside the core is relied on only by
those who choose it.

**Why.** A flaw in a shared platform reaches every program on it. You make that less likely
mainly by making the platform smaller and cleaner; you cannot make it zero.

**What it helps you catch.** Features added to `rumi-core` that one domain needs. Dependencies
added to core for convenience: each is supply-chain exposure for every user. The engine keeps
its dependencies to a few, and each one earns its place: a linear-time regex crate, because its
guarantee against runaway matching protects everyone, and serde for config loading, on by
default because loading a config is the feature the engine is for.

**Why it helps.** It explains, rather than asserts, why core changes get more scrutiny.

### A provider should not make its clients rely on each other

**Principle.** A shared component should keep giving correct service to well-behaved clients,
whatever its other clients do. Then a client depends on the provider, not on every other user
of the provider.

**Why.** If one caller can corrupt shared state, every caller depends on every other caller
behaving. That is a hidden coupling no interface shows.

**What it helps you catch.** Mutable shared state in the engine. x.uma's registry is immutable
after it is built, and its core types are thread-safe, so one host thread cannot disturb
another's match.

**Why it helps.** It turns "is this safe to share?" into a property you can state and test.

### Trust sits with the host; an extension gets only what it is handed

**Principle.** The host decides what an extension may touch. An input sees the context it is
given and nothing else. An action is data the engine returns, not code the engine runs.
Third-party code gets only the authority passed to it, never ambient access.

**Why.** An extension interface that hands out more than an extension needs makes every user
trust every extension. Authority passed explicitly can be reasoned about; ambient authority
cannot.

**What it helps you catch.**
- An input that reaches outside its context: the filesystem, the environment, a global.
- A proposal for actions the engine executes rather than returns.

**Why it helps.** The security review of an extension shrinks to the authority it was handed.

### No part may starve the others

**Principle.** Isolation against errors is not isolation against resource exhaustion. Every
config is bounded in what one evaluation can cost, checked when it loads.

**Why.** A component can be correct and still deny service to everything sharing its process,
by taking all the time or all the memory. Defensive consistency does not cover that; a budget
does. x.uma budgets depth and the work one evaluation can reach.

**What it helps you catch.** A new matcher, input or tree shape whose cost is not counted by
the load-time budget.

**Why it helps.** "A hostile config cannot hang the host" stays a checked property as the
engine grows.

### The core should not promise what the ends must check anyway

**Principle.** If only the endpoints can do a check completely, the core may offer it as a
convenience or a speed-up, but not as a guarantee. Clients that do not need a function still
pay for it if it lives in the shared core.

**Why.** A guarantee the core cannot fully deliver gives callers false confidence, and every
client pays for it. The host knows what its data means; the engine does not.

**What it helps you catch.** The engine validating domain semantics that only the host can
know, or growing a feature one host needs at every host's cost.

**Why it helps.** It keeps the line between engine and host where the knowledge is.

### The host checks an extension when it registers

**Principle.** An extension is checked when it is registered and when a config referencing it
loads, not when it first runs. The registry states what happens when two registrations claim
the same type URL.

**Why.** A problem found at registration is found by the developer. One found at match time is
found by a user. A duplicate that silently wins by registration order is a change in meaning
that nobody chose.

**What it helps you catch.** A registry that lets the last registration win silently, and an
extension hook that exists but is never checked.

**Why it helps.** Extension errors surface where they can be fixed.

### Declaring a boundary does not hold it

**Principle.** A boundary holds only where something checks it at the point of change: a
compile error, a CI check, a conformance fixture. A rule written in a document drifts.

**Why.** Boundaries decay through small, reasonable exceptions, each invisible on its own. This
repository has examples of checks that exist precisely because a documented rule had already
been broken: which crates are publishable, which fixtures each runner must read, which code
samples must compile.

**What it helps you catch.** A new architectural rule that lands as prose with no check behind
it.

**Why it helps.** The maintainer gets rules that defend themselves, not rules they must police.

### Write the caller's code before the implementation

**Principle.** Before implementing an interface, write the code that will use it: the config
fixture, the README example, the host snippet. Then build to that.

**Why.** An interface designed from the inside fits its implementation. One designed from the
caller's side fits its use, and the awkwardness shows up while it is still free to fix.

**What it helps you catch.** A binding method nobody has tried to call, and a config field that
reads badly in YAML.

**Why it helps.** The maintainer can review the design as a user would meet it, before any
implementation exists to defend.

### Identifiers handed out should not encode implementation choices

**Principle.** Names, type URLs and IDs that users hold say what a thing is, not how rumi
implements it.

**Why.** An identifier that encodes an implementation detail freezes that detail. Changing the
implementation then means renaming something users depend on.

**What it helps you catch.** A type URL named after an internal struct, or a result field named
after an algorithm.

**Why it helps.** Implementation changes stay invisible to users.

---

## 5. Boundaries and surprise

These principles come from research on how adaptive systems (organisations, control systems,
hospitals) cope with surprise. That research is not about software; its author took the name
"graceful extensibility" partly from software engineering. Use it as a lens, not a rule book.

### Every design has bounds; plan what happens at them

**Principle.** Any system handles a finite range of situations well. Events will fall outside
it. What matters is how it behaves at and beyond the edge, and there are two different good
answers:
- **Degrading gracefully:** losing function in a controlled, visible way. Refusing, failing
  closed, returning "undecided".
- **Extending gracefully:** stretching to handle what was not planned for. A new payload field
  read by path, a new domain registered without touching the engine.

Know which one each edge is meant to do.

**Why.** A system optimised for the situations it was designed for tends to fail suddenly at
the boundary, not gradually. Behaviour at the boundary has to be designed; it does not emerge.
Most of x.uma's edges should degrade: a security matcher that is unsure must refuse. Its
extension points are where it should stretch.

**What it helps you catch.** Missing edge behaviour: what the engine does at its depth limit,
at a budget, when a required value is absent, when a payload is malformed, when a host sends an
event it has never seen. Each should have a stated, tested answer, and the answer should say
whether it degrades or extends.

**Why it helps.** It gives the maintainer a short list of edges to rule on, instead of a
surprise in production.

### Extension capacity looks like waste until it is needed

**Principle.** Spare capacity for adaptation (a seam, an indirection, a generic path) looks
excessive before the event that needs it, and so it is the first thing cut in pursuit of
efficiency. Pursuing base performance in isolation is likely to undermine the ability to
extend.

**Why.** The two kinds of value trade against each other: efficiency in the expected case, and
room to stretch in the unexpected one. Optimising only the first quietly spends the second.

**What it helps you catch.** A performance change that removes an extension seam: hard-coding
the built-in inputs to skip type-URL resolution, or exposing a packed layout to save a copy.
Look for a way to get the speed behind the seam instead.

**Why it helps.** It names the cost a benchmark cannot show, so the maintainer weighs it
explicitly.

### Improving the base moves surprise; it does not remove it

**Principle.** Widening what a system handles changes which events challenge its new
boundaries. It does not shrink the unknown.

**Why.** Each improvement invites new uses, which bring new edges.

**What it helps you catch.** "Now that we handle X, we don't need the fallback." Keep the
boundary behaviour after every improvement.

**Why it helps.** It keeps the fail-closed paths in the design after the feature that first
justified them is done.

---

## 6. Limits of this guidance

- **It is judgment, not measurement.** No controlled study backs any principle above. Where a
  claim can be measured (performance, compatibility), measure it: a benchmark with a criterion
  written before the run, a conformance fixture, a public-surface snapshot.
- **It covers trust only as structure, and performance not at all.** The trust principles here
  are about how authority and resources are handed to extensions. They are not a threat model;
  use a security review for that, and the benchmarks for speed. A design that passes this skill
  can still be exploitable or slow.
- **Authorities disagree on several live questions:** demanding or tolerant contracts, sealed or
  open by default, exceptions or error codes, generalise now or later, when to break
  compatibility. None has a rule. Decide per case, and record the decision and its reason.
- **A decision is not made until it is written down.** In x.uma that means `DECISIONS.md`. New
  vocabulary is defined there before it appears anywhere else.

---

## Before you change something that freezes

1. Is this addition needed by a real user now? If not, leave it out.
2. Does code that knows nothing of it behave exactly as before?
3. What does it promise, including limits, ordering, silence and failure? Write it down.
4. Which likely change does it make cheap, and which does it make expensive?
5. What happens when it meets a value it has never seen? Is that rule stated and tested?
6. Does every failure path fail visibly, and closed where "unknown" would read as "yes"?
7. Can the lists a caller receives grow without breaking them?
8. Does it keep policy out of the engine, keep the core small, and hand extensions only the
   authority and budget they need?
9. How many independent consumers have used it, and does the release decision say so?
10. What checks the boundary it creates?
11. Is the decision in `DECISIONS.md`?
