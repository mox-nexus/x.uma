# Predicate Composition

Real matchers combine conditions. AND, OR, NOT — compose them freely.

## SinglePredicate: The Building Block

A predicate is extraction plus matching in one step:

```python
from puma import SinglePredicate, PrefixMatcher
from puma.http import PathInput

# "Does the path start with /api?"
predicate = SinglePredicate(
    input=PathInput(),
    matcher=PrefixMatcher("/api")
)
```

When evaluated, it extracts the path and checks if it starts with `/api`. One step.

## AND: All Must Match

Combine multiple conditions. All must be true.

```python
from puma import And, SinglePredicate, PrefixMatcher, ExactMatcher
from puma.http import PathInput, MethodInput

# "Is it GET /api/*?"
predicate = And((
    SinglePredicate(PathInput(), PrefixMatcher("/api")),
    SinglePredicate(MethodInput(), ExactMatcher("GET")),
))
```

**Short-circuits:** Stops evaluating on the first false condition. If the path check fails, the method check never runs.

**Rust equivalent:**
```rust
use rumi::prelude::*;
use rumi_http::{PathInput, MethodInput};

let predicate = Predicate::and(vec![
    SinglePredicate::new(PathInput, PrefixMatcher::new("/api")),
    SinglePredicate::new(MethodInput, ExactMatcher::new("GET")),
]);
```

**TypeScript equivalent:**
```typescript
import { And, SinglePredicate, PrefixMatcher, ExactMatcher } from '@x.uma/buma';
import { PathInput, MethodInput } from '@x.uma/buma/http';

const predicate = new And([
  new SinglePredicate(new PathInput(), new PrefixMatcher('/api')),
  new SinglePredicate(new MethodInput(), new ExactMatcher('GET')),
]);
```

## OR: Any Must Match

At least one condition must be true.

```python
from puma import Or, SinglePredicate, PrefixMatcher
from puma.http import PathInput

# "Does the path start with /api or /admin?"
predicate = Or((
    SinglePredicate(PathInput(), PrefixMatcher("/api")),
    SinglePredicate(PathInput(), PrefixMatcher("/admin")),
))
```

**Short-circuits:** Stops evaluating on the first true condition. If the first prefix matches, the second check never runs.

## NOT: Invert the Result

Negate any predicate.

```python
from puma import Not, SinglePredicate, ExactMatcher
from puma.http import MethodInput

# "Is it NOT a POST request?"
predicate = Not(
    SinglePredicate(MethodInput(), ExactMatcher("POST"))
)
```

Useful for exclusion rules (match everything except X).

## Nesting: Combine Freely

Predicates nest arbitrarily up to `MAX_DEPTH` (32 levels).

```python
from puma import And, Or, SinglePredicate, PrefixMatcher, ExactMatcher
from puma.http import PathInput, MethodInput, HeaderInput

# "(GET /api/* OR POST /api/*) AND has auth header"
predicate = And((
    Or((
        And((
            SinglePredicate(MethodInput(), ExactMatcher("GET")),
            SinglePredicate(PathInput(), PrefixMatcher("/api")),
        )),
        And((
            SinglePredicate(MethodInput(), ExactMatcher("POST")),
            SinglePredicate(PathInput(), PrefixMatcher("/api")),
        )),
    )),
    SinglePredicate(HeaderInput("authorization"), PrefixMatcher("Bearer ")),
))
```

This reads like: "Accept GET or POST to `/api/*`, but only if the request has an authorization header starting with `Bearer `."

**Depth validation:** Trees exceeding 32 levels raise `MatcherError` at construction time. This protects against stack overflow and enforces a reasonable complexity limit.

## Gateway API Compiler

Manual predicate construction is verbose. The HTTP domain provides a compiler that builds predicates from Gateway API config:

```python
from puma.http import HttpRouteMatch, HttpPathMatch, HttpHeaderMatch

# Human-friendly config
route = HttpRouteMatch(
    path=HttpPathMatch(type="PathPrefix", value="/api"),
    method="GET",
    headers=[
        HttpHeaderMatch(type="Exact", name="content-type", value="application/json")
    ],
)

# Compiler builds the predicate tree for you
predicate = route.to_predicate()
```

This generates an `And` with three conditions (path, method, header). You don't write the nesting manually.

**Rust equivalent:**
```rust
use rumi_http::{HttpRouteMatch, HttpPathMatch, HttpHeaderMatch};

let route = HttpRouteMatch {
    path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
    method: Some("GET".into()),
    headers: Some(vec![
        HttpHeaderMatch::Exact {
            name: "content-type".into(),
            value: "application/json".into(),
        }
    ]),
    ..Default::default()
};

let predicate = route.to_predicate();
```

## Empty AND and OR

Edge cases with empty lists:

**Empty AND:** Returns `true` (vacuous truth). No conditions to fail means success.

```python
from puma import And

predicate = And(())  # Empty tuple
assert predicate.evaluate(request) == True
```

**Empty OR:** Returns `false`. No conditions to pass means failure.

```python
from puma import Or

predicate = Or(())  # Empty tuple
assert predicate.evaluate(request) == False
```

These match standard boolean algebra. In practice, you rarely construct empty predicates directly.

## The None → False Invariant

If a `DataInput` returns `None`, the predicate evaluates to `False` without calling the matcher.

```python
from puma import SinglePredicate, ExactMatcher
from puma.http import HttpRequest, HeaderInput

predicate = SinglePredicate(
    input=HeaderInput("x-api-key"),
    matcher=ExactMatcher("secret")
)

# Header not present → DataInput returns None → predicate returns False
request = HttpRequest(headers={})
assert predicate.evaluate(request) == False
```

The matcher never sees `None`. The predicate handles it upstream. This simplifies matcher implementations (they only handle present values).

## Cross-Language Predicate Types

All three implementations provide the same predicate types with identical semantics.

**Rust:**
```rust
pub enum Predicate<Ctx> {
    Single(SinglePredicate<Ctx>),
    And(Vec<Predicate<Ctx>>),
    Or(Vec<Predicate<Ctx>>),
    Not(Box<Predicate<Ctx>>),
}
```

**Python:**
```python
type Predicate[Ctx] = SinglePredicate[Ctx] | And[Ctx] | Or[Ctx] | Not[Ctx]
```

**TypeScript:**
```typescript
type Predicate<Ctx> =
  | SinglePredicate<Ctx>
  | And<Ctx>
  | Or<Ctx>
  | Not<Ctx>;
```

Same structure. Same short-circuit behavior. Same depth limits. Write once in any language, the logic transfers.

## Next Steps

- [The Matching Pipeline](pipeline.md) — Where predicates fit in the flow
- [First-Match-Wins Semantics](semantics.md) — How matchers use predicates
- [Build an HTTP Router](../tutorials/http-router.md) — Predicates in action
