# The Matching Pipeline

Every evaluation follows the same flow. Understanding this pipeline is understanding x.uma.

## The Flow

```text
Context (your data)
    ↓
DataInput.get()          ← extract a value from the context
    ↓
MatchingData             ← type-erased: string | int | bool | bytes | null
    ↓
InputMatcher.matches()   ← compare the value
    ↓
bool                     ← did it match?
    ↓
Predicate.evaluate()     ← combine with other conditions (AND/OR/NOT)
    ↓
bool                     ← combined result
    ↓
Matcher.evaluate()       ← find the first matching rule
    ↓
Action                   ← your decision (or null if nothing matched)
```

Two things to notice:

1. **The pipeline splits at `MatchingData`.** Everything above is domain-specific (knows about your context type). Everything below is domain-agnostic (works with any domain).

2. **The same `InputMatcher` works everywhere.** An `ExactMatcher` doesn't care whether the string came from an HTTP path or a Claude Code tool name. It matches strings.

## Concrete Example

Route `GET /api/users` to the API backend:

```rust,no_run
use rumi::prelude::*;
use rumi_http::*;

let predicate = Predicate::Single(SinglePredicate::new(
    Box::new(SimplePathInput),         // domain-specific
    Box::new(PrefixMatcher::new("/api")), // domain-agnostic
));

let matcher: Matcher<HttpRequest, &str> = Matcher::list(
    vec![FieldMatcher::new(predicate, OnMatch::Action("api_backend"))],
    None,
);

let request = HttpRequest::builder().method("GET").path("/api/users").build();
assert_eq!(matcher.evaluate(&request), Some("api_backend"));
```

The Python and TypeScript packages do not expose these types. A binding takes
the same rule as a config document, hands it to this engine, and hands the
decision back; the pipeline runs in Rust wherever it runs. See the
[config format](../reference/config.md) for the document, and the
[Python](../getting-started/python.md) and
[TypeScript](../getting-started/typescript.md) quick starts for the door.

## The Same Pipeline, Different Domain

The power of this split: the same `PrefixMatcher` works for HTTP paths and
custom event types.

```rust
use rumi::prelude::*;

// Your context.
struct CloudEvent {
    kind: String,
}

// Your DataInput: extract the event type. This is the only domain-specific
// piece, and it is the whole port.
#[derive(Debug)]
struct EventTypeInput;

impl DataInput<CloudEvent> for EventTypeInput {
    fn get(&self, ctx: &CloudEvent) -> MatchingData {
        MatchingData::from(ctx.kind.as_str())
    }
}

// The SAME PrefixMatcher. It knows nothing about CloudEvent.
let predicate = Predicate::Single(SinglePredicate::new(
    Box::new(EventTypeInput),
    Box::new(PrefixMatcher::new("com.example.")),
));

let matcher: Matcher<CloudEvent, &str> = Matcher::list(
    vec![FieldMatcher::new(predicate, OnMatch::Action("handle_event"))],
    None,
);

let event = CloudEvent { kind: "com.example.user.created".into() };
assert_eq!(matcher.evaluate(&event), Some("handle_event"));
```

`PrefixMatcher` operates on `MatchingData` (the erased string), not on
`CloudEvent` or `HttpRequest`. Domain adapters (`SimplePathInput`,
`EventTypeInput`) are context-specific. Matchers are universal.

## Pipeline Stages

| Stage | Role | Generic? | Examples |
|-------|------|----------|----------|
| **Context** | Your domain data | Yes (`Ctx`) | `HttpRequest`, `HookContext`, your type |
| **DataInput** | Extract a value | Yes (`Ctx`) | `PathInput`, `ToolNameInput`, your input |
| **MatchingData** | Type-erased value | No | `string`, `int`, `bool`, `bytes`, `null` |
| **InputMatcher** | Match the value | No | `ExactMatcher`, `PrefixMatcher`, `RegexMatcher` |
| **Predicate** | Boolean logic | Yes (`Ctx`) | `SinglePredicate`, `And`, `Or`, `Not` |
| **Matcher** | First-match-wins | Yes (`Ctx`, `A`) | Routes to actions |
| **Action** | Your decision | Yes (`A`) | Strings, enums, structs — anything |

The boundary at `MatchingData` is what makes the engine domain-agnostic. Cross it once, and every matcher works for every domain.

## Next

- [Type Erasure and Ports](type-erasure.md) — why `InputMatcher` is non-generic
