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

**Python:**
```python
from xuma import SinglePredicate, PrefixMatcher, Matcher, FieldMatcher, Action
from xuma.http import HttpRequest, PathInput

# DataInput: extract the path from the request
# InputMatcher: check if the path starts with /api
predicate = SinglePredicate(
    input=PathInput(),            # domain-specific
    matcher=PrefixMatcher("/api") # domain-agnostic
)

matcher = Matcher(
    matcher_list=(
        FieldMatcher(predicate=predicate, on_match=Action("api_backend")),
    ),
)

request = HttpRequest(method="GET", raw_path="/api/users")
assert matcher.evaluate(request) == "api_backend"
```

**Rust:**
```rust,ignore
use rumi::prelude::*;
use rumi_http::*;

let predicate = Predicate::Single(SinglePredicate::new(
    Box::new(SimplePathInput),         // domain-specific
    Box::new(PrefixMatcher::new("/api")), // domain-agnostic
));

let matcher: Matcher<HttpRequest, &str> = Matcher::new(
    vec![FieldMatcher::new(predicate, OnMatch::Action("api_backend"))],
    None,
);

let request = HttpRequest::builder().method("GET").path("/api/users").build();
assert_eq!(matcher.evaluate(&request), Some(&"api_backend"));
```

**TypeScript:**
```typescript
import { SinglePredicate, PrefixMatcher, Matcher, FieldMatcher, Action } from "xuma";
import { HttpRequest, PathInput } from "xuma/http";

const predicate = new SinglePredicate(
  new PathInput(),             // domain-specific
  new PrefixMatcher("/api"),   // domain-agnostic
);

const matcher = new Matcher(
  [new FieldMatcher(predicate, new Action("api_backend"))],
);

const request = new HttpRequest("GET", "/api/users");
console.assert(matcher.evaluate(request) === "api_backend");
```

Same structure in all three languages. Same result.

## The Same Pipeline, Different Domain

The power of this split: the same `PrefixMatcher` works for HTTP paths and custom event types.

```python
from dataclasses import dataclass
from xuma import SinglePredicate, PrefixMatcher, Matcher, FieldMatcher, Action, MatchingData

# Custom context
@dataclass(frozen=True)
class CloudEvent:
    type: str
    source: str

# Custom DataInput — extract the event type
@dataclass(frozen=True)
class EventTypeInput:
    def get(self, ctx: CloudEvent) -> MatchingData:
        return ctx.type

# Use the SAME PrefixMatcher — it doesn't know about CloudEvent
predicate = SinglePredicate(
    input=EventTypeInput(),
    matcher=PrefixMatcher("com.example."),
)

matcher = Matcher(
    matcher_list=(
        FieldMatcher(predicate=predicate, on_match=Action("handle_event")),
    ),
)

event = CloudEvent(type="com.example.user.created", source="api")
assert matcher.evaluate(event) == "handle_event"
```

`PrefixMatcher` operates on `MatchingData` (the erased string), not on `CloudEvent` or `HttpRequest`. Domain adapters (`PathInput`, `EventTypeInput`) are context-specific. Matchers are universal.

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
