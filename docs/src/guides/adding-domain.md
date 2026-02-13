# Adding a Domain

Create domain-specific matchers for HTTP, gRPC, CloudEvents, or your custom protocol.

A "domain" in x.uma is a context type with associated `DataInput` implementations and a compiler that turns config into matchers.

**Every domain ships four things:**

| Deliverable | What | Example (HTTP) |
|-------------|------|----------------|
| **Context type** | The runtime data structure | `HttpRequest`, `HttpMessage` |
| **DataInputs** | Extract fields from context | `PathInput`, `HeaderInput` |
| **Config types** | User-friendly match specification | `HttpRouteMatch`, `HttpPathMatch` |
| **Compiler** | Config → Matcher tree | `compile_route_matches()` |

The compiler is the user-facing API. Without it, users must construct matcher trees by hand.

This guide shows how to add a domain in Rust (rumi), Python (puma), and TypeScript (bumi).

## Concept: CloudEvent Matching

We'll build a matcher for [CloudEvents](https://cloudevents.io/) — a CNCF spec for event metadata.

```json
{
  "specversion": "1.0",
  "type": "com.example.user.created",
  "source": "api-server",
  "id": "abc-123",
  "subject": "user/42"
}
```

**Goal:** Match events by type, source, or subject.

## Python Implementation (puma)

### Step 1: Define the Context Type

```python
# my_events.py
from dataclasses import dataclass

@dataclass(frozen=True, slots=True)
class CloudEvent:
    """CloudEvents v1.0 context."""
    type: str
    source: str
    id: str
    subject: str | None = None
```

### Step 2: Implement DataInput

```python
from puma import DataInput, MatchingValue

@dataclass(frozen=True, slots=True)
class EventTypeInput:
    """Extract event type."""
    def get(self, ctx: CloudEvent, /) -> MatchingValue:
        return ctx.type

@dataclass(frozen=True, slots=True)
class EventSourceInput:
    """Extract event source."""
    def get(self, ctx: CloudEvent, /) -> MatchingValue:
        return ctx.source

@dataclass(frozen=True, slots=True)
class EventSubjectInput:
    """Extract event subject (may be None)."""
    def get(self, ctx: CloudEvent, /) -> MatchingValue:
        return ctx.subject  # None triggers None → false invariant
```

### Step 3: Build Matchers

```python
from puma import (
    SinglePredicate, ExactMatcher, PrefixMatcher,
    and_predicate, matcher_from_predicate,
)

# Match user creation events from api-server
user_created_matcher = matcher_from_predicate(
    and_predicate(
        [
            SinglePredicate(EventTypeInput(), PrefixMatcher("com.example.user.")),
            SinglePredicate(EventSourceInput(), ExactMatcher("api-server")),
        ],
        SinglePredicate(EventTypeInput(), PrefixMatcher("")),  # catch-all
    ),
    "handle_user_event",
    on_no_match="ignore",
)

# Evaluate
event = CloudEvent(
    type="com.example.user.created",
    source="api-server",
    id="abc-123",
)
action = user_created_matcher.evaluate(event)
assert action == "handle_user_event"
```

### Step 4: Add a Compiler (User-Facing API)

The compiler is the user-facing API for your domain. Every x.uma domain ships one:

- HTTP: `compile_route_matches()` turns `HttpRouteMatch` config into a `Matcher`
- Claude: `compile_hook_matches()` turns `HookMatch` config into a `Matcher`

Without a compiler, users must construct matcher trees by hand. The compiler is what makes a domain usable.

```python
from dataclasses import dataclass
from puma import (
    Predicate, SinglePredicate, ExactMatcher, PrefixMatcher,
    Matcher, and_predicate, or_predicate, matcher_from_predicate,
)

def _catch_all() -> Predicate[CloudEvent]:
    return SinglePredicate(EventTypeInput(), PrefixMatcher(""))

@dataclass(frozen=True, slots=True)
class EventMatch:
    """Config-time event match specification."""
    type: str | None = None
    source: str | None = None
    subject: str | None = None

    def to_predicate(self) -> Predicate[CloudEvent]:
        """Compile to predicate tree (AND all present fields)."""
        predicates: list[SinglePredicate[CloudEvent]] = []

        if self.type is not None:
            predicates.append(
                SinglePredicate(EventTypeInput(), ExactMatcher(self.type))
            )
        if self.source is not None:
            predicates.append(
                SinglePredicate(EventSourceInput(), ExactMatcher(self.source))
            )
        if self.subject is not None:
            predicates.append(
                SinglePredicate(EventSubjectInput(), ExactMatcher(self.subject))
            )

        return and_predicate(predicates, _catch_all())

    def compile[A](self, action: A) -> Matcher[CloudEvent, A]:
        """Compile to matcher."""
        return matcher_from_predicate(self.to_predicate(), action)

def compile_event_matches[A](
    matches: list[EventMatch], action: A, on_no_match: A | None = None,
) -> Matcher[CloudEvent, A]:
    """Compile multiple event matches (ORed) into a single Matcher."""
    predicates = [m.to_predicate() for m in matches]
    return matcher_from_predicate(
        or_predicate(predicates, _catch_all()), action, on_no_match,
    )

# Usage
match_config = EventMatch(type="com.example.user.created", source="api-server")
matcher = match_config.compile("handle_user_event")
```

**The pattern is always the same across every domain:**
1. `to_predicate()` uses `and_predicate(fields, catch_all)` to AND the fields
2. `compile()` uses `matcher_from_predicate(predicate, action)` to wrap into a Matcher
3. `compile_*_matches()` uses `or_predicate(predicates, catch_all)` to OR multiple configs

That's it. The same string matchers, predicates, and matcher tree logic work for CloudEvents.

## Rust Implementation (rumi)

### Step 1: Define Proto (Optional)

If using xDS proto extension mechanism:

```protobuf
// proto/xuma/events/v1/inputs.proto
syntax = "proto3";
package xuma.events.v1;

message EventTypeInput {}
message EventSourceInput {}
message EventSubjectInput {}
```

Generate Rust bindings:
```bash
just gen
```

### Step 2: Define the Context Type

```rust
// rumi/ext/events/src/lib.rs
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CloudEvent {
    pub event_type: String,
    pub source: String,
    pub id: String,
    pub subject: Option<String>,
}
```

### Step 3: Implement DataInput

```rust
use rumi::{DataInput, MatchingData};

#[derive(Debug)]
pub struct EventTypeInput;

impl DataInput<CloudEvent> for EventTypeInput {
    fn get(&self, ctx: &CloudEvent) -> MatchingData {
        MatchingData::String(ctx.event_type.clone())
    }
}

#[derive(Debug)]
pub struct EventSourceInput;

impl DataInput<CloudEvent> for EventSourceInput {
    fn get(&self, ctx: &CloudEvent) -> MatchingData {
        MatchingData::String(ctx.source.clone())
    }
}

#[derive(Debug)]
pub struct EventSubjectInput;

impl DataInput<CloudEvent> for EventSubjectInput {
    fn get(&self, ctx: &CloudEvent) -> MatchingData {
        ctx.subject
            .as_ref()
            .map(|s| MatchingData::String(s.clone()))
            .unwrap_or(MatchingData::None)
    }
}
```

### Step 4: Build Matchers

```rust
use rumi::prelude::*;

// Match user creation events from api-server
let user_created_matcher = Matcher::from_predicate(
    Predicate::from_all(
        vec![
            Predicate::Single(SinglePredicate::new(
                Box::new(EventTypeInput),
                Box::new(PrefixMatcher::new("com.example.user.")),
            )),
            Predicate::Single(SinglePredicate::new(
                Box::new(EventSourceInput),
                Box::new(ExactMatcher::new("api-server")),
            )),
        ],
        catch_all(),
    ),
    "handle_user_event",
    Some("ignore"),
)?;

// Evaluate
let event = CloudEvent {
    event_type: "com.example.user.created".into(),
    source: "api-server".into(),
    id: "abc-123".into(),
    subject: None,
};

let action = user_created_matcher.evaluate(&event);
assert_eq!(action, Some("handle_user_event"));

fn catch_all() -> Predicate<CloudEvent> {
    Predicate::Single(SinglePredicate::new(
        Box::new(EventTypeInput),
        Box::new(PrefixMatcher::new("")),
    ))
}
```

### Step 5: Add a Compiler (User-Facing API)

```rust
#[derive(Debug, Clone)]
pub struct EventMatch {
    pub event_type: Option<String>,
    pub source: Option<String>,
    pub subject: Option<String>,
}

impl EventMatch {
    pub fn compile<A>(self, action: A) -> Result<Matcher<CloudEvent, A>, MatcherError>
    where
        A: Clone + Send + Sync + 'static,
    {
        Ok(Matcher::from_predicate(self.to_predicate(), action, None))
    }

    fn to_predicate(&self) -> Predicate<CloudEvent> {
        let mut predicates = Vec::new();

        if let Some(ref t) = self.event_type {
            predicates.push(Predicate::Single(SinglePredicate::new(
                Box::new(EventTypeInput),
                Box::new(ExactMatcher::new(t)),
            )));
        }
        if let Some(ref s) = self.source {
            predicates.push(Predicate::Single(SinglePredicate::new(
                Box::new(EventSourceInput),
                Box::new(ExactMatcher::new(s)),
            )));
        }
        if let Some(ref subj) = self.subject {
            predicates.push(Predicate::Single(SinglePredicate::new(
                Box::new(EventSubjectInput),
                Box::new(ExactMatcher::new(subj)),
            )));
        }

        Predicate::from_all(predicates, catch_all())
    }
}

/// Compile multiple event matches (ORed) into a single Matcher.
pub fn compile_event_matches<A>(
    matches: &[EventMatch],
    action: A,
    on_no_match: Option<A>,
) -> Result<Matcher<CloudEvent, A>, MatcherError>
where
    A: Clone + Send + Sync + 'static,
{
    let predicates: Vec<_> = matches.iter().map(EventMatch::to_predicate).collect();
    Ok(Matcher::from_predicate(
        Predicate::from_any(predicates, catch_all()),
        action,
        on_no_match,
    ))
}

fn catch_all() -> Predicate<CloudEvent> {
    Predicate::Single(SinglePredicate::new(
        Box::new(EventTypeInput),
        Box::new(PrefixMatcher::new("")),
    ))
}

// Usage
let match_config = EventMatch {
    event_type: Some("com.example.user.created".into()),
    source: Some("api-server".into()),
    subject: None,
};
let matcher = match_config.compile("handle_user_event")?;
```

## TypeScript Implementation (bumi)

The same pattern applies in TypeScript. Classes with `readonly` fields replace dataclasses.

### Steps 1-3: Context, DataInputs, and Matchers

```typescript
import {
    SinglePredicate, ExactMatcher, PrefixMatcher,
    andPredicate, matcherFromPredicate,
    type DataInput, type MatchingData,
} from "bumi";

// Step 1: Context type
class CloudEvent {
    constructor(
        readonly type: string,
        readonly source: string,
        readonly id: string,
        readonly subject: string | null = null,
    ) {}
}

// Step 2: DataInputs
class EventTypeInput implements DataInput<CloudEvent> {
    get(ctx: CloudEvent): MatchingData { return ctx.type; }
}

class EventSourceInput implements DataInput<CloudEvent> {
    get(ctx: CloudEvent): MatchingData { return ctx.source; }
}

class EventSubjectInput implements DataInput<CloudEvent> {
    get(ctx: CloudEvent): MatchingData { return ctx.subject; }  // null → false invariant
}

function catchAll() {
    return new SinglePredicate(new EventTypeInput(), new PrefixMatcher(""));
}

// Step 3: Build matchers
const matcher = matcherFromPredicate(
    andPredicate(
        [
            new SinglePredicate(new EventTypeInput(), new PrefixMatcher("com.example.user.")),
            new SinglePredicate(new EventSourceInput(), new ExactMatcher("api-server")),
        ],
        catchAll(),
    ),
    "handle_user_event",
    "ignore",
);
```

### Steps 4-5: Config Types and Compiler

```typescript
import {
    SinglePredicate, ExactMatcher, PrefixMatcher,
    andPredicate, orPredicate, matcherFromPredicate,
} from "bumi";
import type { Predicate, Matcher } from "bumi";

interface EventMatch {
    readonly type?: string;
    readonly source?: string;
    readonly subject?: string;
}

function eventMatchToPredicate(config: EventMatch): Predicate<CloudEvent> {
    const predicates: Predicate<CloudEvent>[] = [];

    if (config.type !== undefined) {
        predicates.push(new SinglePredicate(new EventTypeInput(), new ExactMatcher(config.type)));
    }
    if (config.source !== undefined) {
        predicates.push(new SinglePredicate(new EventSourceInput(), new ExactMatcher(config.source)));
    }
    if (config.subject !== undefined) {
        predicates.push(new SinglePredicate(new EventSubjectInput(), new ExactMatcher(config.subject)));
    }

    return andPredicate(predicates, catchAll());
}

function compileEventMatches<A>(
    configs: readonly EventMatch[],
    action: A,
    onNoMatch?: A,
): Matcher<CloudEvent, A> {
    const predicates = configs.map(eventMatchToPredicate);
    return matcherFromPredicate(orPredicate(predicates, catchAll()), action, onNoMatch);
}

// Usage
const matcher = compileEventMatches(
    [{ type: "com.example.user.created", source: "api-server" }],
    "handle_user_event",
);
```

## Add Conformance Tests

All three implementations should pass the same test fixtures.

```yaml
# spec/tests/events/type_exact.yaml
name: "CloudEvent type exact match"
context:
  type: "com.example.user.created"
  source: "api-server"
  id: "abc-123"
cases:
  - description: "matches event type"
    matcher:
      input: { event_type: {} }
      matcher: { exact: "com.example.user.created" }
    expected: { matches: true }

  - description: "rejects wrong type"
    matcher:
      input: { event_type: {} }
      matcher: { exact: "com.example.order.created" }
    expected: { matches: false }
```

**Test runners:**
- Rust: Parse YAML, construct `CloudEvent` and `Matcher<CloudEvent, _>`, assert
- Python: Parse YAML, construct `CloudEvent` and `Matcher[CloudEvent, _]`, assert
- TypeScript: Parse YAML, construct `CloudEvent` and `Matcher<CloudEvent, _>`, assert

## Key Patterns

### None Handling

When a `DataInput` returns `None`/`null`, the predicate evaluates to `false` without consulting the matcher. This is the **null → false invariant**.

```rust
// Rust — return MatchingData::None
impl DataInput<CloudEvent> for EventSubjectInput {
    fn get(&self, ctx: &CloudEvent) -> MatchingData {
        ctx.subject
            .as_ref()
            .map(|s| MatchingData::String(s.clone()))
            .unwrap_or(MatchingData::None)  // Triggers null → false
    }
}
```

```python
# Python — return None
@dataclass(frozen=True, slots=True)
class EventSubjectInput:
    def get(self, ctx: CloudEvent, /) -> MatchingValue:
        return ctx.subject  # None triggers null → false
```

```typescript
// TypeScript — return null
class EventSubjectInput implements DataInput<CloudEvent> {
    get(ctx: CloudEvent): MatchingData { return ctx.subject; }  // null triggers null → false
}
```

### Immutability

All context types should be immutable:

| Language | Pattern |
|----------|---------|
| Rust | Don't provide `&mut` access, clone when needed |
| Python | `@dataclass(frozen=True)` |
| TypeScript | `readonly` fields |

### Compiler Composition Pipeline

Every domain compiler follows the same three-step pipeline:

```
Domain provides                          Core composes
─────────────                            ──────────────
(input, spec) pairs per field     →  and_predicate(fields, catch_all)  →  one Predicate per config
                                  →  or_predicate(configs, catch_all)  →  one Predicate total
                                  →  matcher_from_predicate(pred, action, fallback) → Matcher
```

The only domain-specific code is `to_predicate()` — mapping config fields to `(DataInput, StringMatcher)` pairs. Everything else is structural composition provided by the core helpers.

### Type Safety

The compiler ensures:
- `DataInput<CloudEvent>` can only be used with `Matcher<CloudEvent, A>`
- You can't accidentally mix HTTP and CloudEvent inputs in the same matcher
- `OnMatch` is exclusive — action XOR nested matcher, never both

## See Also

- [Rust API Reference](../reference/rust.md) — rumi types
- [Python API Reference](../reference/python.md) — puma types
- [TypeScript API Reference](../reference/typescript.md) — bumi types
- [HTTP Domain](../reference/http.md) — Real-world example
- [Architecture](../explain/architecture.md) — Type system deep dive
