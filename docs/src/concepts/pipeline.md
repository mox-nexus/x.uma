# The Matching Pipeline

Every matcher is a pipeline. Data flows in one end, a decision comes out the other. Understanding this flow is understanding x.uma.

## The Flow

Here's what happens when you evaluate a request:

```text
HttpRequest
    ↓
PathInput.get()        ← "Extract the path"
    ↓
"/api/users"          ← MatchingValue (type-erased)
    ↓
PrefixMatcher.matches() ← "Does it start with /api?"
    ↓
true                  ← boolean result
    ↓
Predicate.evaluate()   ← "Combine with other conditions"
    ↓
true                  ← combined result
    ↓
Matcher.evaluate()     ← "Find the first matching rule"
    ↓
"api_backend"         ← Action (your decision)
```

Each step is a port. Domain-specific adapters (like `PathInput`) plug in at the edges. The core (like `PrefixMatcher`) is domain-agnostic and reusable.

## HTTP Example

Route GET requests to `/api/*` to the API backend:

**Python:**
```python
from puma import SinglePredicate, PrefixMatcher, Matcher, FieldMatcher, Action
from puma.http import HttpRequest, PathInput

# Step 1: Define extraction + matching
predicate = SinglePredicate(
    input=PathInput(),
    matcher=PrefixMatcher("/api")
)

# Step 2: Build the matcher tree
matcher = Matcher(
    matcher_list=(
        FieldMatcher(predicate=predicate, on_match=Action("api_backend")),
    )
)

# Step 3: Evaluate against requests
request = HttpRequest(method="GET", raw_path="/api/users")
result = matcher.evaluate(request)
assert result == "api_backend"
```

**Rust:**
```rust
use rumi::prelude::*;
use rumi_http::{HttpMessage, PathInput};

// Step 1: Define extraction + matching
let predicate = SinglePredicate::new(
    PathInput,
    PrefixMatcher::new("/api"),
);

// Step 2: Build the matcher tree
let matcher = Matcher::new(
    vec![FieldMatcher::new(predicate, OnMatch::Action("api_backend"))],
    None,
);

// Step 3: Evaluate against requests
let result = matcher.evaluate(&http_message);
assert_eq!(result, Some("api_backend"));
```

## The Same Pipeline, Different Domain

The power of type erasure: the same `PrefixMatcher` works for HTTP paths and CloudEvent types.

**CloudEvent example (custom domain):**

```python
from dataclasses import dataclass
from puma import SinglePredicate, PrefixMatcher, Matcher, FieldMatcher, Action, DataInput, MatchingData

# Define your context type
@dataclass
class CloudEvent:
    type: str
    source: str
    data: dict

# Define your extraction adapter
@dataclass
class EventTypeInput:
    def get(self, ctx: CloudEvent) -> MatchingData:
        return ctx.type

# Use the SAME PrefixMatcher
predicate = SinglePredicate(
    input=EventTypeInput(),
    matcher=PrefixMatcher("com.example.")  # Same matcher, different domain!
)

matcher = Matcher(
    matcher_list=(
        FieldMatcher(predicate=predicate, on_match=Action("route_to_handler")),
    )
)

# Evaluate against events
event = CloudEvent(type="com.example.user.created", source="api", data={})
result = matcher.evaluate(event)
assert result == "route_to_handler"
```

The `PrefixMatcher` doesn't know or care whether it's matching HTTP paths or event types. It operates on `MatchingData` (the type-erased value), not the original context type.

**This is the core insight:** domain adapters (`PathInput`, `EventTypeInput`) are context-specific. Core matchers (`PrefixMatcher`, `ExactMatcher`) are domain-agnostic. The pipeline connects them.

## Pipeline Stages

| Stage | Role | Generic? | Examples |
|-------|------|----------|----------|
| **Context** | Your domain data | Yes (`Ctx`) | `HttpRequest`, `CloudEvent`, `GrpcRequest` |
| **DataInput** | Extract data | Yes (`Ctx`) | `PathInput`, `EventTypeInput`, `HeaderInput` |
| **MatchingValue** | Type-erased data | No | `str`, `int`, `bool`, `bytes`, `None` |
| **InputMatcher** | Match the value | No | `ExactMatcher`, `PrefixMatcher`, `RegexMatcher` |
| **Predicate** | Boolean composition | Yes (`Ctx`) | `SinglePredicate`, `And`, `Or`, `Not` |
| **Matcher** | First-match-wins tree | Yes (`Ctx`, `A`) | Routes to actions |
| **Action** | Your decision | Yes (`A`) | `"api_backend"`, `42`, custom types |

The middle stage (`MatchingValue` → `InputMatcher` → `bool`) is where domain-agnostic magic happens. Same matchers, different domains.

## Next Steps

- [Type Erasure and Ports](type-erasure.md) — Why InputMatcher is non-generic
- [Predicate Composition](predicates.md) — Combining conditions with AND/OR/NOT
- [First-Match-Wins Semantics](semantics.md) — Evaluation order and nested matchers
