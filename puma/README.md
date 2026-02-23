# puma — Pure Python xDS Matcher

**Alpha (v0.1.0)** — Part of the [x.uma](https://github.com/mox-labs/x.uma) research project.

puma is a pure Python implementation of the xDS Unified Matcher API. Match structured data (HTTP requests, events, messages) against rule trees with first-match-wins semantics. Python 3.12+.

## Installation

```bash
pip install puma
# or with uv
uv add puma
```

## Examples

### Example 1: Match a Dictionary Value

Start with the simplest case — extract a value from a dict and match it against a pattern.

```python
from puma import Matcher, FieldMatcher, SinglePredicate, ExactMatcher, Action
from dataclasses import dataclass

# 1. Define a data input (extraction port)
@dataclass(frozen=True, slots=True)
class DictInput:
    key: str

    def get(self, ctx: dict[str, str], /) -> str | None:
        return ctx.get(self.key)

# 2. Build a matcher tree
matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(
                input=DictInput("name"),
                matcher=ExactMatcher("alice")
            ),
            on_match=Action("admin")
        ),
        FieldMatcher(
            predicate=SinglePredicate(
                input=DictInput("name"),
                matcher=ExactMatcher("bob")
            ),
            on_match=Action("user")
        ),
    ),
    on_no_match=Action("guest")
)

# 3. Evaluate
matcher.evaluate({"name": "alice"})  # "admin"
matcher.evaluate({"name": "bob"})    # "user"
matcher.evaluate({"name": "eve"})    # "guest"
matcher.evaluate({})                 # "guest"
```

**What happened here:**
- `DictInput` extracts a value from the dict (the extraction port)
- `ExactMatcher` checks if that value matches (the matching port)
- `SinglePredicate` combines extraction + matching
- `Matcher` evaluates predicates in order, returns the first match

This is the core pattern. Everything else builds on it.

### Example 2: HTTP Route Matching

Now the same pattern applied to HTTP — match requests against route rules.

```python
from puma.http import HttpRequest, HttpPathMatch, HttpRouteMatch, compile_route_matches

# Define route rules (Gateway API style)
api_route = HttpRouteMatch(
    path=HttpPathMatch(type="PathPrefix", value="/api"),
    method="GET"
)

admin_route = HttpRouteMatch(
    path=HttpPathMatch(type="PathPrefix", value="/admin"),
)

# Compile to a matcher
matcher = compile_route_matches(
    matches=[api_route, admin_route],
    action="matched",
    on_no_match="404"
)

# Evaluate requests
req1 = HttpRequest(method="GET", raw_path="/api/users")
matcher.evaluate(req1)  # "matched" (api_route)

req2 = HttpRequest(method="POST", raw_path="/api/users")
matcher.evaluate(req2)  # "404" (wrong method)

req3 = HttpRequest(raw_path="/admin/settings")
matcher.evaluate(req3)  # "matched" (admin_route)
```

Under the hood, `compile_route_matches` builds the same `Matcher` tree you saw in Example 1, using `PathInput`, `MethodInput`, etc.

## API Overview

### Core Types

| Type | Purpose |
|------|---------|
| `DataInput[Ctx]` | Protocol — extract a value from context `Ctx` |
| `InputMatcher` | Protocol — match a `MatchingData` (str, int, bool, bytes, None) |
| `MatchingData` | Type alias — the erased data type returned by inputs |
| `SinglePredicate[Ctx]` | Combines a `DataInput` with an `InputMatcher` |
| `Matcher[Ctx, A]` | Top-level matcher tree — evaluates to action `A` or `None` |
| `FieldMatcher[Ctx, A]` | Pairs a predicate with an outcome (action or nested matcher) |
| `OnMatch[Ctx, A]` | Type alias — `Action[A]` or `NestedMatcher[Ctx, A]` |

### Predicates (Boolean Logic)

| Type | Semantics |
|------|-----------|
| `SinglePredicate[Ctx]` | Extract + match |
| `And[Ctx]` | All predicates must match (short-circuits on False) |
| `Or[Ctx]` | Any predicate must match (short-circuits on True) |
| `Not[Ctx]` | Invert the inner predicate |
| `Predicate[Ctx]` | Type alias — union of the above |

### String Matchers

All matchers support optional `ignore_case` (except `RegexMatcher`).

| Matcher | Matches | Example |
|---------|---------|---------|
| `ExactMatcher(value)` | Exact equality | `"hello"` matches `"hello"` |
| `PrefixMatcher(prefix)` | String starts with | `"/api"` matches `"/api/users"` |
| `SuffixMatcher(suffix)` | String ends with | `".json"` matches `"data.json"` |
| `ContainsMatcher(substring)` | Substring present | `"world"` matches `"hello world"` |
| `RegexMatcher(pattern)` | Regex search | `r"\d+"` matches `"abc123"` |

### HTTP Domain

| Type | Purpose |
|------|---------|
| `HttpRequest` | Request context (method, path, headers, query params) |
| `PathInput` | Extract request path |
| `MethodInput` | Extract HTTP method |
| `HeaderInput(name)` | Extract header by name (case-insensitive) |
| `QueryParamInput(name)` | Extract query param by name |
| `HttpRouteMatch` | Gateway API route config (path, method, headers, query params) |
| `compile_route_matches()` | Compile route configs into a `Matcher` |

## How to Extend: Custom Domains

puma uses hexagonal architecture. To add a new domain, implement the `DataInput` protocol for your context type.

```python
from dataclasses import dataclass
from puma import DataInput, MatchingData, Matcher, FieldMatcher, SinglePredicate, ExactMatcher, Action

# Your custom context
@dataclass
class CloudEvent:
    type: str
    source: str
    subject: str | None = None

# DataInput for event type
@dataclass(frozen=True, slots=True)
class EventTypeInput:
    def get(self, ctx: CloudEvent, /) -> MatchingData:
        return ctx.type

# DataInput for event subject
@dataclass(frozen=True, slots=True)
class EventSubjectInput:
    def get(self, ctx: CloudEvent, /) -> MatchingData:
        return ctx.subject  # None if not present

# Build a matcher
matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(
                input=EventTypeInput(),
                matcher=ExactMatcher("com.example.user.created")
            ),
            on_match=Action("handle_user_created")
        ),
    ),
)

# Evaluate
event = CloudEvent(type="com.example.user.created", source="api")
matcher.evaluate(event)  # "handle_user_created"
```

The same string matchers, predicates, and matcher tree logic work across all domains.

## Architecture: Ports & Adapters

puma follows hexagonal architecture (ports & adapters). The core is domain-agnostic. Domains plug in at the edges.

```
┌─────────────────────────────────┐
│       Domain Adapters           │
│   HTTP  CloudEvent  Custom      │
│ (PathInput, EventTypeInput, ...)│
└───────────────┬─────────────────┘
                │
┌───────────────▼─────────────────┐
│           PORTS                 │
│  DataInput[Ctx] → MatchingData │
│  InputMatcher → bool            │
└───────────────┬─────────────────┘
                │
┌───────────────▼─────────────────┐
│           CORE                  │
│  Matcher, Predicate, Actions    │
│    (domain-agnostic)            │
└─────────────────────────────────┘
```

**DataInput** is the extraction port — domain-specific, generic over context `Ctx`.

**InputMatcher** is the matching port — domain-agnostic, non-generic. The same `ExactMatcher` works for HTTP, CloudEvent, or any other domain. This is the key design insight from Envoy's matcher architecture.

## Semantics (xDS)

puma implements xDS Unified Matcher semantics:

1. **First-match-wins** — Matchers evaluate `field_matchers` in order, returning the action from the first matching predicate. Later matches are never consulted.

2. **OnMatch exclusivity** — Each `OnMatch` is either an `Action` (terminal) or a `NestedMatcher` (recurse), never both.

3. **Nested matcher failure propagates** — If a nested matcher returns `None`, evaluation continues to the next `field_matcher` (no implicit fallback).

4. **on_no_match fallback** — If no predicate matches, `on_no_match` is consulted. If absent, the matcher returns `None`.

5. **None → false invariant** — If a `DataInput` returns `None`, the predicate evaluates to `False` without consulting the matcher.

6. **Depth validation** — Matcher trees exceeding `MAX_DEPTH` (32 levels) are rejected at construction with `MatcherError`.

## Security

`RegexMatcher` uses `google-re2` (linear-time, ReDoS-safe). See [SECURITY.md](SECURITY.md) for regex restrictions, error types, and depth limits.

## Requirements

- Python 3.12+ (uses PEP 695 type parameter syntax)
- `google-re2`

## Status

puma is alpha software (v0.1.0). The API is under active development and will change.

Part of the x.uma matcher ecosystem — implementing the xDS Unified Matcher API across Rust (rumi), Python (puma), and TypeScript (bumi). All implementations pass the same conformance test suite.

See the [x.uma README](https://github.com/mox-labs/x.uma) for the full project roadmap.

## License

MIT
