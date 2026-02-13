# First-Match-Wins Semantics

Evaluation order matters. The first match wins, and nested failures propagate.

## The Rules

x.uma follows xDS matcher semantics. Six rules govern evaluation:

1. First-match-wins
2. OnMatch exclusivity
3. Nested matcher failure propagation
4. on_no_match fallback
5. None → false invariant
6. Depth validation

Each rule affects what happens when you evaluate a matcher tree. Get these wrong, and your routes won't behave as expected.

## Rule 1: First-Match-Wins

Matchers evaluate field matchers in order. The first matching predicate wins. Later matches are never consulted.

```python
from puma import Matcher, FieldMatcher, SinglePredicate, PrefixMatcher, Action
from puma.http import HttpRequest, PathInput

matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/api")),
            on_match=Action("api_backend")
        ),
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/api/v2")),
            on_match=Action("api_v2_backend")  # NEVER REACHED!
        ),
    )
)

request = HttpRequest(raw_path="/api/v2/users")
result = matcher.evaluate(request)
assert result == "api_backend"  # First match wins
```

The path `/api/v2/users` matches both rules, but the first rule wins. The second rule is shadowed.

**Order matters.** More specific rules must come first:

```python
# Correct order: specific before general
matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/api/v2")),
            on_match=Action("api_v2_backend")
        ),
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/api")),
            on_match=Action("api_backend")
        ),
    )
)

request = HttpRequest(raw_path="/api/v2/users")
result = matcher.evaluate(request)
assert result == "api_v2_backend"  # Correct!
```

## Rule 2: OnMatch Exclusivity

Each `OnMatch` is either an action (terminal) or a nested matcher (continue evaluation), never both.

```python
from puma import Action, NestedMatcher

# Valid: action
on_match = Action("route_here")

# Valid: nested matcher
on_match = NestedMatcher(matcher=sub_matcher)

# Invalid: can't have both
# (Not representable in the type system)
```

The type system enforces this. You can't accidentally create an `OnMatch` that does both. This prevents ambiguity: when a predicate matches, the outcome is clear.

**Rust makes this explicit with an enum:**
```rust
pub enum OnMatch<Ctx, A> {
    Action(A),
    Matcher(Box<Matcher<Ctx, A>>),
}
```

**Python uses a union type:**
```python
type OnMatch[Ctx, A] = Action[A] | NestedMatcher[Ctx, A]
```

Same constraint, different syntax. Illegal states are unrepresentable.

## Rule 3: Nested Matcher Failure Propagation

If a nested matcher returns `None`, evaluation continues to the next field matcher. There is no implicit fallback to `on_no_match`.

```python
from puma import Matcher, FieldMatcher, NestedMatcher, SinglePredicate, ExactMatcher, Action
from puma.http import HttpRequest, MethodInput, PathInput

# Nested matcher that only matches POST
nested = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(MethodInput(), ExactMatcher("POST")),
            on_match=Action("create_resource")
        ),
    ),
    on_no_match=None  # No fallback!
)

# Parent matcher
matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/api")),
            on_match=NestedMatcher(nested)  # Continue into nested matcher
        ),
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/health")),
            on_match=Action("health_check")
        ),
    )
)

# GET /api → nested matcher evaluates, returns None, parent continues to next rule
request = HttpRequest(method="GET", raw_path="/api/users")
result = matcher.evaluate(request)
assert result == None  # /health doesn't match, no fallback

# GET /health → second rule matches
request = HttpRequest(method="GET", raw_path="/health")
result = matcher.evaluate(request)
assert result == "health_check"
```

**Common mistake:** Expecting nested matcher failure to use the parent's `on_no_match`. It doesn't. Nested failure means "this branch didn't match, try the next field matcher."

## Rule 4: on_no_match Fallback

If no predicate matches, the matcher consults `on_no_match`. If absent, the matcher returns `None`.

```python
from puma import Matcher, FieldMatcher, SinglePredicate, PrefixMatcher, Action
from puma.http import HttpRequest, PathInput

# With fallback
matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/api")),
            on_match=Action("api_backend")
        ),
    ),
    on_no_match=Action("default_backend")  # Fallback for non-matches
)

request = HttpRequest(raw_path="/other")
result = matcher.evaluate(request)
assert result == "default_backend"

# Without fallback
matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/api")),
            on_match=Action("api_backend")
        ),
    ),
    on_no_match=None  # No fallback
)

request = HttpRequest(raw_path="/other")
result = matcher.evaluate(request)
assert result == None
```

`on_no_match` applies only when no predicate in `matcher_list` matches. It doesn't apply when a nested matcher fails (Rule 3).

## Rule 5: None → False Invariant

If a `DataInput` returns `None`, the predicate evaluates to `False` without calling the matcher.

```python
from puma import SinglePredicate, ExactMatcher
from puma.http import HttpRequest, HeaderInput

predicate = SinglePredicate(
    input=HeaderInput("x-api-key"),
    matcher=ExactMatcher("secret")
)

# Header not present → input returns None → predicate returns False
request = HttpRequest(headers={})
assert predicate.evaluate(request) == False

# Header present → input returns str → predicate evaluates normally
request = HttpRequest(headers={"x-api-key": "wrong"})
assert predicate.evaluate(request) == False

request = HttpRequest(headers={"x-api-key": "secret"})
assert predicate.evaluate(request) == True
```

This simplifies matcher implementations. They only handle present values. Missing values are handled upstream by the predicate.

## Rule 6: Depth Validation

Matcher trees exceeding `MAX_DEPTH` (32 levels) are rejected at construction time.

```python
from puma import Matcher, FieldMatcher, NestedMatcher, SinglePredicate, ExactMatcher, Action, MatcherError
from puma.http import HttpRequest, PathInput

# Build a deeply nested matcher
def build_nested(depth):
    if depth == 0:
        return Matcher(
            matcher_list=(
                FieldMatcher(
                    predicate=SinglePredicate(PathInput(), ExactMatcher("/deep")),
                    on_match=Action("leaf")
                ),
            )
        )
    return Matcher(
        matcher_list=(
            FieldMatcher(
                predicate=SinglePredicate(PathInput(), ExactMatcher("/")),
                on_match=NestedMatcher(build_nested(depth - 1))
            ),
        )
    )

# Depth 32 is ok
matcher = build_nested(31)  # 32 levels total
assert matcher.depth() == 32

# Depth 33 raises error
try:
    matcher = build_nested(32)  # 33 levels total
    assert False, "Should have raised MatcherError"
except MatcherError as e:
    assert "depth exceeds MAX_DEPTH" in str(e)
```

Validation happens in `__post_init__` (Python) or constructor (Rust). You can't accidentally create invalid trees.

**Why 32?** Balance between flexibility and safety. Deep nesting risks stack overflow (when iterative evaluation is deferred) and signals overly complex config. Most real-world matchers use 3-5 levels.

## Evaluation Example

Combine all rules in one matcher:

```python
from puma import Matcher, FieldMatcher, NestedMatcher, SinglePredicate, And, PrefixMatcher, ExactMatcher, Action
from puma.http import HttpRequest, PathInput, MethodInput, HeaderInput

# Nested matcher: POST with auth header
auth_required = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=And((
                SinglePredicate(MethodInput(), ExactMatcher("POST")),
                SinglePredicate(HeaderInput("authorization"), PrefixMatcher("Bearer ")),
            )),
            on_match=Action("authenticated_api")
        ),
    ),
    on_no_match=None  # Fail if not POST with auth
)

# Top-level matcher
matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/api")),
            on_match=NestedMatcher(auth_required)  # Nested evaluation
        ),
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/health")),
            on_match=Action("health_check")
        ),
    ),
    on_no_match=Action("not_found")  # Fallback
)

# Test cases
# 1. POST /api with auth → nested matches
request = HttpRequest(method="POST", raw_path="/api/users", headers={"authorization": "Bearer token"})
assert matcher.evaluate(request) == "authenticated_api"

# 2. GET /api → nested fails (not POST), continues to next field matcher, then fallback
request = HttpRequest(method="GET", raw_path="/api/users")
assert matcher.evaluate(request) == "not_found"

# 3. GET /health → second rule matches
request = HttpRequest(method="GET", raw_path="/health")
assert matcher.evaluate(request) == "health_check"

# 4. GET /other → no match, uses fallback
request = HttpRequest(method="GET", raw_path="/other")
assert matcher.evaluate(request) == "not_found"
```

Walk through case 2:
1. First field matcher: path matches `/api` → predicate is `True`
2. OnMatch is nested matcher → evaluate `auth_required`
3. Nested matcher: method is GET (not POST) → predicate is `False`
4. Nested matcher returns `None` (Rule 4: no `on_no_match`)
5. Nested failure propagates (Rule 3) → continue to next field matcher
6. Second field matcher: path doesn't match `/health` → predicate is `False`
7. No more field matchers → use `on_no_match` (Rule 4)
8. Return `"not_found"`

## Next Steps

- [The Matching Pipeline](pipeline.md) — Where evaluation fits in the flow
- [Predicate Composition](predicates.md) — Building the conditions that drive evaluation
- [Build an HTTP Router](../tutorials/http-router.md) — Apply these semantics to real routing
