# Conformance Test Fixtures

YAML fixtures every x.uma implementation must pass. The suite is the source of
truth for correctness: Rust, Python and TypeScript all run the same files and
must produce the same answers.

## Read this first: there are four dialects, not one

The top-level key of a fixture selects which loader reads it, and the four are
**not interchangeable**. An earlier version of this file documented only the
first one, which is the one no user can write.

| Top-level key | Fixtures | What it exercises | Can a user write this? |
|---|---|---|---|
| `config:` | 7 | The **shipping config format** — what a user actually authors and what `rumi run` loads | **Yes** |
| `matcher:` | 14 | Native construction. Builds `Matcher` values directly, bypassing the config layer entirely | No |
| `http_route_match:` | 5 | One Gateway API route through the HTTP compiler | Via the compiler API, not config |
| `http_route_matches:` | 1 | Several Gateway API routes, ORed | Via the compiler API, not config |

Each has its own branch in each of three loaders:

- Rust — `rumi/ext/test/src/fixture.rs`
- Python — `puma/tests/conftest.py`
- TypeScript — `bumi/tests/helpers/fixture-loader.ts`

Adding a dialect means touching all three. Prefer `config:` unless you are
specifically testing a path that config cannot reach.

## The shipping format: `config:`

This is the one to copy. It is the format `rumi run` and `rumi check` accept,
and the only one that round-trips through the registry.

```yaml
name: "and_predicate"
description: "Compound AND predicate: all conditions must match"

config:
  matchers:
    - predicate:
        type: and
        predicates:
          - type: single
            input: { type_url: "xuma.kv.v1.MapInput", config: { key: "role" } }
            value_match: { Exact: "admin" }
          - type: single
            input: { type_url: "xuma.kv.v1.MapInput", config: { key: "org" } }
            value_match: { Prefix: "acme" }
      on_match: { type: action, action: "admin_acme" }

cases:
  - name: "both_match"
    context: { role: "admin", org: "acme-corp" }
    expect: "admin_acme"
  - name: "first_fails"
    context: { role: "viewer", org: "acme-corp" }
    expect: null
```

Notes that cost time if you learn them the hard way:

- **Casing is inconsistent and it is not a typo.** `type:` and `on_match:` are
  lowercase; `value_match` variants are PascalCase (`Exact`, `Prefix`,
  `Suffix`, `Contains`, `Regex`). That is a Rust serde enum default that leaked
  into a cross-language format.
- **`config: {}` is optional.** Inputs that take no configuration can omit it.
- **`expect: null` means no match**, including after `on_no_match` is consulted.

## The native dialect: `matcher:`

Fourteen fixtures build matchers directly, without going through config. They
exist to test engine semantics — predicate composition, first-match-wins,
`on_no_match` chains — independently of whether the config layer can express
them.

Its shape is *not* the config shape. `single:` rather than `type: single`,
`{ key: ... }` rather than a `type_url`, lowercase `exact:` rather than `Exact:`.
Copying it into a real config will not load.

```yaml
matcher:
  matchers:
    - predicate:
        single:
          input: { key: "field_name" }
          value_match: { exact: "expected_value" }
      on_match:
        action: "action_name"
  on_no_match:
    action: "default_action"
```

## The compiler dialects: `http_route_match:` / `http_route_matches:`

Gateway API `HttpRouteMatch` values fed through `compile_route_matches`. Plural
takes a list and ORs them. Neither is a config format — they are inputs to a
compiler API.

## Status

**These four are transitional.** The config format is moving to protojson, which
will replace the `config:` dialect and may or may not absorb `matcher:` — that
call belongs to the migration, not to this file. Key renames were deliberately
*not* done here, because renaming keys in three loaders before knowing which
dialects survive is work that gets thrown away.

Until then: `config:` is the format users write. The other three are test
apparatus, and this table is the only place that says so.

## Adding a fixture

1. Write the fixture first — this project is conformance-driven.
2. Use `config:` unless you are testing something config cannot express.
3. Run it in all three implementations before committing:
   `just test-fixtures`, `just puma-test`, `just bumi-test`.
4. A fixture that passes in one implementation and not another is a finding,
   not a fixture bug. Cross-language disagreement is what this suite is for.
