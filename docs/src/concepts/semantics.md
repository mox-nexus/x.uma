# First-Match-Wins Semantics

Evaluation order matters. x.uma follows xDS matcher semantics — six rules govern how a matcher tree produces a decision.

## The Six Rules

1. **First-match-wins** — first matching predicate's action is returned
2. **OnMatch exclusivity** — action XOR nested matcher, never both
3. **Nested matcher failure** — continues to next field matcher
4. **on_no_match fallback** — used only when nothing matches
5. **None-to-false** — missing data means predicate is false
6. **Depth validation** — max 32 levels, checked at construction

## Rule 1: First-Match-Wins

Matchers evaluate field matchers in order. The first matching predicate wins. Later matches are never consulted.

```python
from xuma import Matcher, FieldMatcher, SinglePredicate, PrefixMatcher, Action
from xuma.http import HttpRequest, PathInput

matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/api")),
            on_match=Action("api_backend"),
        ),
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/api/v2")),
            on_match=Action("api_v2_backend"),  # NEVER REACHED for /api/v2 paths
        ),
    ),
)

request = HttpRequest(raw_path="/api/v2/users")
assert matcher.evaluate(request) == "api_backend"  # First rule wins
```

The path `/api/v2/users` matches both rules, but the first one wins. **Order matters.** Put specific rules before general ones.

## Rule 2: OnMatch Exclusivity

Each `OnMatch` is either an action (terminal) or a nested matcher (continue evaluation). Never both. The type system enforces this:

**Rust:**
```rust,ignore
pub enum OnMatch<Ctx, A> {
    Action(A),
    Matcher(Box<Matcher<Ctx, A>>),
}
```

**Python:**
```python
type OnMatch[Ctx, A] = Action[A] | NestedMatcher[Ctx, A]
```

Illegal states are unrepresentable. When a predicate matches, the outcome is unambiguous.

## Rule 3: Nested Matcher Failure

If a predicate matches but its nested matcher returns `None`, evaluation continues to the next field matcher. There is no implicit fallback.

```python
from xuma import Matcher, FieldMatcher, NestedMatcher, SinglePredicate, ExactMatcher, Action
from xuma.http import HttpRequest, MethodInput, PathInput

# Nested matcher: only matches POST
nested = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(MethodInput(), ExactMatcher("POST")),
            on_match=Action("create"),
        ),
    ),
    on_no_match=None,  # No fallback inside nested
)

# Parent matcher
matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/api")),
            on_match=NestedMatcher(nested),
        ),
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/health")),
            on_match=Action("health_check"),
        ),
    ),
    on_no_match=Action("not_found"),
)

# GET /api → path matches, enters nested → method doesn't match → nested returns None
# → continues to /health → doesn't match → falls to on_no_match
request = HttpRequest(method="GET", raw_path="/api/users")
assert matcher.evaluate(request) == "not_found"
```

**Common mistake:** expecting nested failure to use the parent's `on_no_match`. It doesn't. Nested failure means "this branch didn't match, try the next field matcher."

## Rule 4: on_no_match Fallback

If no predicate in `matcher_list` matches, the matcher consults `on_no_match`. If absent, returns `None`/`null`.

```python
# With fallback
matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/api")),
            on_match=Action("api"),
        ),
    ),
    on_no_match=Action("default"),  # Used when no rules match
)

assert matcher.evaluate(HttpRequest(raw_path="/other")) == "default"
```

`on_no_match` applies only when no predicate matches. It does not apply when a nested matcher fails (Rule 3).

## Rule 5: None-to-False

Covered in [Predicate Composition](predicates.md). When `DataInput` returns `None`/`null`, the predicate evaluates to `false` without calling the matcher. Missing data never accidentally matches.

## Rule 6: Depth Validation

Matcher trees exceeding `MAX_DEPTH` (32 levels) are rejected at construction time.

```python
# Attempting to build a tree deeper than 32 levels raises MatcherError
```

Validation happens at construction, not evaluation. If a `Matcher` object exists, it's known to be valid. Parse, don't validate.

## Walkthrough

All six rules in one example:

```python
from xuma import Matcher, FieldMatcher, NestedMatcher, SinglePredicate, And
from xuma import PrefixMatcher, ExactMatcher, Action
from xuma.http import HttpRequest, PathInput, MethodInput, HeaderInput

# Nested: POST with auth header
auth_required = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=And((
                SinglePredicate(MethodInput(), ExactMatcher("POST")),
                SinglePredicate(HeaderInput("authorization"), PrefixMatcher("Bearer ")),
            )),
            on_match=Action("authenticated_api"),
        ),
    ),
    on_no_match=None,
)

# Top-level
matcher = Matcher(
    matcher_list=(
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/api")),
            on_match=NestedMatcher(auth_required),
        ),
        FieldMatcher(
            predicate=SinglePredicate(PathInput(), PrefixMatcher("/health")),
            on_match=Action("health_check"),
        ),
    ),
    on_no_match=Action("not_found"),
)

# POST /api with auth → nested matches → "authenticated_api"
r1 = HttpRequest(method="POST", raw_path="/api/users",
                 headers={"authorization": "Bearer token"})
assert matcher.evaluate(r1) == "authenticated_api"

# GET /api → nested fails (not POST) → continues → /health doesn't match → "not_found"
r2 = HttpRequest(method="GET", raw_path="/api/users")
assert matcher.evaluate(r2) == "not_found"

# GET /health → second rule matches → "health_check"
r3 = HttpRequest(method="GET", raw_path="/health")
assert matcher.evaluate(r3) == "health_check"
```

Walk through `GET /api`:
1. First field matcher: path matches `/api` (Rule 1)
2. OnMatch is nested matcher (Rule 2: exclusive)
3. Nested: method is GET, not POST — predicate false
4. Nested returns `None` — no `on_no_match` (Rule 4)
5. Nested failure propagates (Rule 3) — continue to next field matcher
6. Second field matcher: path `/api/users` doesn't start with `/health`
7. No more field matchers — use `on_no_match` (Rule 4)
8. Return `"not_found"`

## Next

- [The Matching Pipeline](pipeline.md) — where evaluation fits in the data flow
- [Predicate Composition](predicates.md) — the Boolean logic that drives evaluation
- [Build an HTTP Router](../tutorials/http-router.md) — apply these semantics to real routing
