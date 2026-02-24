# Adding a Domain Adapter

x.uma ships with HTTP and Claude Code domains. You can add your own. A domain adapter is a set of `DataInput` implementations that extract fields from your context type, plus an optional compiler for ergonomic construction.

## What You Need

1. **A context type** — the data structure your matcher evaluates against
2. **DataInput implementations** — one per matchable field
3. **(Optional) A compiler** — transforms domain-specific config into matcher trees
4. **(Optional) Registry registration** — enables config-driven construction

## Step 1: Define Your Context

```python
from dataclasses import dataclass

@dataclass(frozen=True)
class CloudEvent:
    type: str
    source: str
    subject: str | None = None
    data: dict | None = None
```

In Rust:

```rust,ignore
#[derive(Debug, Clone)]
pub struct CloudEvent {
    pub event_type: String,
    pub source: String,
    pub subject: Option<String>,
    pub data: Option<serde_json::Value>,
}
```

## Step 2: Implement DataInput

One `DataInput` per field you want to match against:

```python
from dataclasses import dataclass
from xuma import MatchingData

@dataclass(frozen=True)
class EventTypeInput:
    def get(self, ctx: CloudEvent) -> MatchingData:
        return ctx.type

@dataclass(frozen=True)
class SourceInput:
    def get(self, ctx: CloudEvent) -> MatchingData:
        return ctx.source

@dataclass(frozen=True)
class SubjectInput:
    def get(self, ctx: CloudEvent) -> MatchingData:
        return ctx.subject  # None when absent → predicate returns false
```

In Rust:

```rust,ignore
use rumi::prelude::*;

#[derive(Debug, Clone)]
pub struct EventTypeInput;

impl DataInput<CloudEvent> for EventTypeInput {
    fn get(&self, ctx: &CloudEvent) -> MatchingData {
        MatchingData::String(ctx.event_type.clone())
    }
}

#[derive(Debug, Clone)]
pub struct SubjectInput;

impl DataInput<CloudEvent> for SubjectInput {
    fn get(&self, ctx: &CloudEvent) -> MatchingData {
        ctx.subject.as_ref()
            .map_or(MatchingData::None, |s| MatchingData::String(s.clone()))
    }
}
```

Key rules:
- Return `MatchingData::None` (or Python `None`) when data is absent
- The None-to-false rule ensures missing fields never match
- Inputs must be `Send + Sync` in Rust (required for thread safety)

## Step 3: Use Your Inputs

Your inputs work with all existing matchers immediately:

```python
from xuma import Matcher, FieldMatcher, SinglePredicate, And, Action
from xuma import PrefixMatcher, ExactMatcher

matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=And((
                SinglePredicate(EventTypeInput(), PrefixMatcher("com.example.")),
                SinglePredicate(SourceInput(), ExactMatcher("api")),
            )),
            on_match=Action("handle_api_event"),
        ),
    ),
    on_no_match=Action("ignore"),
)

event = CloudEvent(type="com.example.user.created", source="api")
assert matcher.evaluate(event) == "handle_api_event"
```

`PrefixMatcher` and `ExactMatcher` don't know about `CloudEvent`. They match erased `MatchingData` strings. That's the whole point of type erasure.

## Step 4: Add a Compiler (Optional)

A compiler transforms domain-specific config into matcher trees:

```python
from dataclasses import dataclass, field
from xuma import Matcher, FieldMatcher, SinglePredicate, And, Action
from xuma import PrefixMatcher, ExactMatcher, RegexMatcher, matcher_from_predicate, and_predicate

@dataclass(frozen=True)
class CloudEventMatch:
    event_type: str | None = None       # prefix match
    source: str | None = None           # exact match
    subject_pattern: str | None = None  # regex match

    def to_predicate(self):
        predicates = []
        if self.event_type is not None:
            predicates.append(SinglePredicate(EventTypeInput(), PrefixMatcher(self.event_type)))
        if self.source is not None:
            predicates.append(SinglePredicate(SourceInput(), ExactMatcher(self.source)))
        if self.subject_pattern is not None:
            predicates.append(SinglePredicate(SubjectInput(), RegexMatcher(self.subject_pattern)))
        # Empty predicates → catch-all (And of empty = true)
        return and_predicate(predicates, SinglePredicate(EventTypeInput(), PrefixMatcher("")))

    def compile(self, action):
        return matcher_from_predicate(self.to_predicate(), action)
```

Usage:

```python
match = CloudEventMatch(event_type="com.example.", source="api")
matcher = match.compile("handle")

event = CloudEvent(type="com.example.user.created", source="api")
assert matcher.evaluate(event) == "handle"
```

## Step 5: Register for Config Path (Optional)

To enable JSON/YAML config loading, implement `IntoDataInput` and register:

```rust,ignore
use rumi::{IntoDataInput, RegistryBuilder, UnitConfig, register_core_matchers};

impl IntoDataInput<CloudEvent> for EventTypeInput {
    type Config = UnitConfig;  // No configuration needed

    fn from_config(_: UnitConfig) -> Result<Box<dyn DataInput<CloudEvent>>, MatcherError> {
        Ok(Box::new(EventTypeInput))
    }
}

pub fn register(builder: RegistryBuilder<CloudEvent>) -> RegistryBuilder<CloudEvent> {
    register_core_matchers(builder)
        .input::<EventTypeInput>("myapp.events.v1.EventTypeInput")
        .input::<SourceInput>("myapp.events.v1.SourceInput")
        .input::<SubjectInput>("myapp.events.v1.SubjectInput")
}
```

Now configs can reference your inputs by type URL:

```json
{
  "matchers": [{
    "predicate": {
      "type": "single",
      "input": { "type_url": "myapp.events.v1.EventTypeInput" },
      "value_match": { "Prefix": "com.example." }
    },
    "on_match": { "type": "action", "action": "handle" }
  }]
}
```

## Checklist

When adding a domain adapter:

- [ ] Context type is immutable (frozen dataclass, `#[derive(Debug, Clone)]`)
- [ ] Each `DataInput` returns `None` for missing/absent data
- [ ] Inputs are `Send + Sync` in Rust
- [ ] Compiler ANDs conditions within a match, ORs across matches
- [ ] Type URLs follow namespace convention (`namespace.domain.version.TypeName`)
- [ ] Registry function registers core matchers AND domain inputs

## Next

- [Type Erasure and Ports](../concepts/type-erasure.md) — why this architecture works
- [The Matching Pipeline](../concepts/pipeline.md) — how your inputs fit in the flow
- [HTTP Matching](../domains/http.md) — reference implementation to study
