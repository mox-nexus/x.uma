# Predicate Composition

Predicates combine conditions with Boolean logic. AND, OR, NOT — compose them freely.

## SinglePredicate

The building block. Pairs a `DataInput` (extract data) with an `InputMatcher` (match data):

```python
from xuma import SinglePredicate, PrefixMatcher
from xuma.http import PathInput

# "Does the path start with /api?"
predicate = SinglePredicate(input=PathInput(), matcher=PrefixMatcher("/api"))
```

Evaluation: extract the path, check if it starts with `/api`, return `bool`.

## And

All conditions must be true. Short-circuits on the first `false`.

```python
from xuma import And, SinglePredicate, PrefixMatcher, ExactMatcher
from xuma.http import PathInput, MethodInput

# "Is it GET /api/*?"
predicate = And((
    SinglePredicate(PathInput(), PrefixMatcher("/api")),
    SinglePredicate(MethodInput(), ExactMatcher("GET")),
))
```

**Rust:**
```rust,ignore
let predicate = Predicate::And(vec![
    Predicate::Single(SinglePredicate::new(
        Box::new(SimplePathInput), Box::new(PrefixMatcher::new("/api")),
    )),
    Predicate::Single(SinglePredicate::new(
        Box::new(SimpleMethodInput), Box::new(ExactMatcher::new("GET")),
    )),
]);
```

**TypeScript:**
```typescript
const predicate = new And([
  new SinglePredicate(new PathInput(), new PrefixMatcher("/api")),
  new SinglePredicate(new MethodInput(), new ExactMatcher("GET")),
]);
```

## Or

At least one condition must be true. Short-circuits on the first `true`.

```python
from xuma import Or, SinglePredicate, PrefixMatcher
from xuma.http import PathInput

# "Does the path start with /api or /admin?"
predicate = Or((
    SinglePredicate(PathInput(), PrefixMatcher("/api")),
    SinglePredicate(PathInput(), PrefixMatcher("/admin")),
))
```

## Not

Negate any predicate. Useful for exclusion rules.

```python
from xuma import Not, SinglePredicate, ExactMatcher
from xuma.http import MethodInput

# "Is it NOT a POST request?"
predicate = Not(SinglePredicate(MethodInput(), ExactMatcher("POST")))
```

## Nesting

Predicates nest arbitrarily up to `MAX_DEPTH` (32 levels):

```python
from xuma import And, Or, SinglePredicate, PrefixMatcher, ExactMatcher
from xuma.http import PathInput, MethodInput, HeaderInput

# "(GET or POST) to /api/* with an auth header"
predicate = And((
    SinglePredicate(PathInput(), PrefixMatcher("/api")),
    Or((
        SinglePredicate(MethodInput(), ExactMatcher("GET")),
        SinglePredicate(MethodInput(), ExactMatcher("POST")),
    )),
    SinglePredicate(HeaderInput("authorization"), PrefixMatcher("Bearer ")),
))
```

Trees exceeding 32 levels raise `MatcherError` at construction time. Most real matchers use 3-5 levels.

## Empty And / Or

Edge cases with empty predicate lists:

- **Empty `And`** returns `true` (vacuous truth — no conditions to fail).
- **Empty `Or`** returns `false` (no conditions to pass).

Standard Boolean algebra. Rarely constructed directly.

## The None-to-False Rule

If a `DataInput` returns `None`/`null`, the `SinglePredicate` evaluates to `false` without calling the matcher. The matcher never sees missing data.

```python
from xuma import SinglePredicate, ExactMatcher
from xuma.http import HttpRequest, HeaderInput

predicate = SinglePredicate(
    input=HeaderInput("x-api-key"),
    matcher=ExactMatcher("secret"),
)

# Header not present → None → false
request = HttpRequest(headers={})
assert predicate.evaluate(request) == False
```

This is a security invariant: missing data never accidentally matches.

## Cross-Language Types

Same predicate types across all implementations:

**Rust:**
```rust,ignore
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
type Predicate<Ctx> = SinglePredicate<Ctx> | And<Ctx> | Or<Ctx> | Not<Ctx>;
```

Same structure. Same short-circuit behavior. Same depth limits.

## Next

- [The Matching Pipeline](pipeline.md) — where predicates fit in the flow
- [First-Match-Wins Semantics](semantics.md) — how matchers use predicates to choose actions
- [Build an HTTP Router](../tutorials/http-router.md) — predicates in practice
