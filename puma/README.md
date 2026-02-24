# xuma — Pure Python xDS Matcher

**v0.0.2** — Part of the [x.uma](https://github.com/mox-nexus/x.uma) matcher engine.

xuma is a pure Python implementation of the xDS Unified Matcher API. Match structured data (HTTP requests, events, messages) against rule trees with first-match-wins semantics. Python 3.12+.

## Installation

```bash
pip install xuma
# or with uv
uv add xuma
```

## Examples

### Example 1: Match a Dictionary Value

Start with the simplest case — extract a value from a dict and match it against a pattern.

```python
from xuma import Matcher, FieldMatcher, SinglePredicate, ExactMatcher, Action
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
from xuma.http import HttpRequest, HttpPathMatch, HttpRouteMatch, compile_route_matches

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

### Example 3: Config-Driven Matchers

Load matcher configuration from YAML/JSON at runtime:

```python
import yaml
from xuma import RegistryBuilder, parse_matcher_config
from xuma.testing import register

config = yaml.safe_load("""
matchers:
  - predicate:
      type: single
      input:
        type_url: "xuma.test.v1.StringInput"
        config:
          key: "method"
      value_match:
        Exact: "GET"
    on_match:
      type: action
      action: "route-get"
on_no_match:
  type: action
  action: "fallback"
""")

builder = RegistryBuilder()
builder = register(builder)
registry = builder.build()
matcher = registry.load_matcher(parse_matcher_config(config))

matcher.evaluate({"method": "GET"})     # "route-get"
matcher.evaluate({"method": "DELETE"})   # "fallback"
```

## Security

`RegexMatcher` uses `google-re2` (linear-time, ReDoS-safe).

## Requirements

- Python 3.12+
- `google-re2`

## License

MIT OR Apache-2.0
