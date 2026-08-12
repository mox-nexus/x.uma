# Debug why something matched

A rule fired that you did not expect, or none fired at all. The engine can show
you the path it took.

## Use the trace

`evaluate` returns the decision. `evaluate_with_trace` returns the decision and
the route to it.

```rust
let trace = matcher.evaluate_with_trace(&ctx);

println!("{:?}", trace.result);
for step in &trace.steps {
    println!("{:?}", step);
}
```

The trace records every field matcher that was considered, the predicate result
for each, and which `on_match` was taken.

## Read it in the CLI

The `rumi` binary does this without writing code:

```bash
rumi eval --config routes.yaml --context method=GET --trace
```

## What the trace does and does not show

Two rules govern the output, and mistaking one for the other wastes time.

**Predicates are fully evaluated.** Inside a compound `and` or `or`, every child
predicate runs even after the outcome is decided. That is deliberate: a
short-circuiting trace would hide the branch you are trying to inspect.

**Matchers still stop at the first match.** The list is first-match-wins, and the
trace agrees with `evaluate`. Rules after the winner were never considered, so
they do not appear. If you expected a later rule to fire, the trace tells you
which earlier rule took the decision.

A known gap: when the fallback runs, a nested `on_no_match` sub-tree is not
recorded in the trace.

## Common causes

| Symptom | Usual cause |
|---|---|
| Nothing matched, input looks right | The input returned no data. A missing key is a non-match, not an error. |
| Wrong rule fired | An earlier rule matched first. Order is significant. |
| Config loaded but never matches | Type URL registered, but reading a different key than you think. |
| Load fails, evaluation never runs | A resource limit. Depth is capped at 32, pattern length at 8192. |

## Check the config before blaming the engine

Loading validates. If a config is structurally wrong, you get an error at load
time rather than a wrong answer at evaluation time:

```bash
rumi validate --config routes.yaml
```

Evaluation is infallible by design. Anything that can fail, fails at load.

## Next

- [Route on a header](route-by-header.md) for the evaluation model in one page.
