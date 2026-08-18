# Conformance Test Fixtures

YAML fixtures every x.uma implementation must pass. The suite is the source of
truth for correctness: Rust, Python and TypeScript run the same files and must
produce the same answers.

## `proto_matcher:` — the format

Canonical protojson: protobuf's own JSON mapping of
`xds.type.matcher.v3.Matcher`. It is what x.uma implements and what a user
writes, and `DECISIONS.md` D-026 explains why the alternative — a terse dialect
that existed nowhere but this repo — was retired.

```yaml
name: "protojson_simple_exact"
description: "One predicate, one action."

proto_matcher:
  matcherList:
    matchers:
      - predicate:
          singlePredicate:
            input:
              name: role
              typedConfig:
                "@type": type.googleapis.com/xuma.kv.v1.MapInput
                key: role
            valueMatch:
              exact: admin
        onMatch:
          action:
            name: allow
            typedConfig:
              "@type": type.googleapis.com/xuma.core.v1.NamedAction
              name: allow

cases:
  - name: "matches"
    context: { role: admin }
    expect: "allow"
  - name: "does_not_match"
    context: { role: viewer }
    expect: null
```

Things worth knowing before you write one:

- **Field names are `lowerCamelCase`**, and both that and the proto's own
  `snake_case` are accepted — protojson allows either. A third spelling is a
  load error.
- **`@type` must carry the full `type.googleapis.com/` prefix.** protojson
  requires it, and a bare name is refused rather than quietly accepted.
- **Unknown fields are load errors**, at every level and inside `@type`
  payloads. That is the point: a typo in a deny rule must not produce a rule
  that silently never fires.
- **`expect: null` means no match**, including after `onNoMatch` is consulted.

### Negative fixtures pin their reason

```yaml
expect_error: true
error_contains: "unknown field"
```

Without `error_contains` a fixture passes on *any* failure — so one that starts
failing earlier still looks green while no longer testing what it was written
for. That happened: a fixture meant to prove a both-set `oneof` is rejected was
failing on an unregistered type instead, and passing.

Because the string has to match in every implementation, it also holds the
error *wording* to cross-language agreement. When Rust and puma disagreed, the
messages were harmonized rather than the assertion weakened.

### `implementations:` — the migration ledger

Omit it and every implementation must run the fixture. That is the state today,
and the field can be deleted once nothing needs it.

A shorter list is an **expiring exception**, and CI holds it in *both*
directions: a listed implementation that fails is a failure, and one that is
**not** listed but succeeds is **also** a failure. The second half is the one
that matters — a skip that quietly starts working means the ledger is reporting
on work somebody already finished.

The property this protects is not "the suite is green". It is that **for every
fixture, every pair of implementations reaches the same verdict**, so a
disagreement still means something while a migration is in flight.

## `http_route_match:` / `http_route_matches:` — the compiler dialect

Gateway API `HttpRouteMatch` values fed through `compile_route_matches`; the
plural takes a list and ORs them. **Not a config format** — they are inputs to a
compiler API, which is why D-026 does not touch them and why they are still
here. Loaded by `puma/tests/conftest.py` and `bumi/tests/helpers/fixture-loader.ts`.

## Coverage is checked

`rumi/ext/test/tests/fixture_coverage.rs` fails when a message in
`proto/xuma/**` has no fixture, or a field of one is never set. Messages with no
fixture are listed there with a reason, and that list is checked for staleness
in both directions.

This exists because puma and bumi read protojson by hand rather than through
generated types (D-038), so their dependency on the schema is not an arrow a
build can see. The fixture corpus carries it instead — without the check, "all
three implementations agree" would mean only "all three agree about whatever
somebody remembered to fixture".

## Adding a fixture

1. Write the fixture first — this project is conformance-driven.
2. Run it everywhere: `just test-protojson`, `just puma-test`, `just bumi-test`.
3. A fixture that passes in one implementation and not another is a **finding**,
   not a fixture bug. Cross-language disagreement is what this suite is for.
