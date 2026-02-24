# Rust API Reference

## Crates

| Crate | Package | Description |
|-------|---------|-------------|
| `rumi` | `rumi-core` | Core matcher engine |
| `rumi-http` | `rumi-http` | HTTP domain (simple + ext-proc) |

```toml
[dependencies]
rumi = "0.0.2"
rumi-http = "0.0.2"

# With Claude domain
rumi = { version = "0.0.2", features = ["claude"] }

# With config/registry
rumi = { version = "0.0.2", features = ["registry"] }
```

## Core Types

### Prelude

```rust,ignore
use rumi::prelude::*;
```

Imports: `Matcher`, `FieldMatcher`, `OnMatch`, `Predicate`, `SinglePredicate`, `MatchingData`, `DataInput`, `InputMatcher`, `ExactMatcher`, `PrefixMatcher`, `SuffixMatcher`, `ContainsMatcher`, `StringMatcher`, `BoolMatcher`, `MatcherError`, `EvalTrace`, `EvalStep`.

### MatchingData

```rust,ignore
pub enum MatchingData {
    None,
    String(String),
    Int(i64),
    Bool(bool),
    Bytes(Vec<u8>),
    Custom(Box<dyn CustomMatchData>),
}
```

The type-erased bridge. `DataInput` returns it, `InputMatcher` consumes it.

### DataInput

```rust,ignore
pub trait DataInput<Ctx>: Send + Sync + Debug {
    fn get(&self, ctx: &Ctx) -> MatchingData;
}
```

Domain-specific: extracts a value from the context. Generic over `Ctx`. Must be `Send + Sync` for thread safety.

### InputMatcher

```rust,ignore
pub trait InputMatcher: Send + Sync + Debug {
    fn matches(&self, value: &MatchingData) -> bool;
}
```

Domain-agnostic: matches a `MatchingData` value. Non-generic — the same implementation works across all domains.

### Matcher

```rust,ignore
pub struct Matcher<Ctx, A> { /* ... */ }

impl<Ctx, A: Clone> Matcher<Ctx, A> {
    pub fn new(matchers: Vec<FieldMatcher<Ctx, A>>, on_no_match: Option<OnMatch<Ctx, A>>) -> Self;
    pub fn evaluate(&self, ctx: &Ctx) -> Option<A>;
    pub fn evaluate_with_trace(&self, ctx: &Ctx) -> (Option<A>, EvalTrace);
}
```

### OnMatch

```rust,ignore
pub enum OnMatch<Ctx, A> {
    Action(A),
    Matcher(Box<Matcher<Ctx, A>>),
}
```

Action XOR nested matcher. Illegal states unrepresentable.

### Predicate

```rust,ignore
pub enum Predicate<Ctx> {
    Single(SinglePredicate<Ctx>),
    And(Vec<Predicate<Ctx>>),
    Or(Vec<Predicate<Ctx>>),
    Not(Box<Predicate<Ctx>>),
}
```

## String Matchers

| Type | Match Behavior |
|------|---------------|
| `ExactMatcher::new("value")` | Exact string equality |
| `PrefixMatcher::new("/api")` | Starts with |
| `SuffixMatcher::new(".json")` | Ends with |
| `ContainsMatcher::new("admin")` | Contains substring |
| `StringMatcher::new("^pat$")` | RE2 regex (linear time) |
| `BoolMatcher::new(true)` | Boolean equality |

## HTTP Types (rumi-http)

### Simple Module (always available)

```rust,ignore
use rumi_http::simple::*;

let request = HttpRequest::builder()
    .method("GET")
    .path("/api/users")
    .header("authorization", "Bearer token")
    .query_param("page", "1")
    .build();
```

| Type | Description |
|------|-------------|
| `HttpRequest` | Simple HTTP request context |
| `HttpRequestBuilder` | Builder for `HttpRequest` |
| `SimplePathInput` | Extracts request path |
| `SimpleMethodInput` | Extracts HTTP method |
| `SimpleHeaderInput::new(name)` | Extracts header (case-insensitive) |
| `SimpleQueryParamInput::new(name)` | Extracts query parameter |

## Claude Types (feature = "claude")

```rust,ignore
use rumi::claude::prelude::*;
```

| Type | Description |
|------|-------------|
| `HookContext` | Claude Code hook event context |
| `HookEvent` | Enum of 9 hook event types |
| `HookMatch` | Declarative hook match config |
| `HookMatchExt` | Extension trait (`compile`, `trace`) |
| `EventInput` | Extracts event type |
| `ToolNameInput` | Extracts tool name |
| `ArgumentInput::new(name)` | Extracts tool argument |
| `SessionIdInput` | Extracts session ID |
| `CwdInput` | Extracts working directory |
| `GitBranchInput` | Extracts git branch |

## Registry (feature = "registry")

```rust,ignore
use rumi::{RegistryBuilder, Registry, IntoDataInput, register_core_matchers};

let registry: Registry<MyContext> = RegistryBuilder::new()
    .input::<MyInput>("myapp.v1.MyInput")
    .build();

let matcher = registry.load_matcher(config)?;
```

| Type | Description |
|------|-------------|
| `RegistryBuilder<Ctx>` | Mutable builder for type registration |
| `Registry<Ctx>` | Immutable registry (Send + Sync) |
| `IntoDataInput<Ctx>` | Trait for config-driven input construction |
| `MatcherConfig<A>` | Deserializable matcher configuration |
| `TypedConfig` | Type URL + config payload reference |

## Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `MAX_DEPTH` | 32 | Maximum nested matcher depth |
| `MAX_FIELD_MATCHERS` | 256 | Maximum field matchers per `Matcher` |
| `MAX_PREDICATES_PER_COMPOUND` | 256 | Maximum children per AND/OR |
| `MAX_PATTERN_LENGTH` | 8192 | Maximum non-regex pattern length |
| `MAX_REGEX_PATTERN_LENGTH` | 4096 | Maximum regex pattern length |

## Errors

```rust,ignore
pub enum MatcherError {
    DepthExceeded { depth, max },
    InvalidPattern { pattern, source },
    InvalidConfig { source },
    UnknownTypeUrl { type_url, registry, available },
    IncompatibleTypes { input_type, matcher_types },
    TooManyFieldMatchers { count, max },
    TooManyPredicates { count, max },
    PatternTooLong { len, max },
}
```

All errors are caught at construction time, not evaluation time. `MatcherError` implements `Display` with actionable messages.
