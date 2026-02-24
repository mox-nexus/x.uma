# Python API Reference

## Installation

```bash
uv add xuma
```

Requires Python 3.12+. Dependency: `google-re2` for linear-time regex.

## Package: xuma

```python
from xuma import (
    # Protocols
    DataInput, InputMatcher, MatchingData,
    # Predicates
    SinglePredicate, And, Or, Not, Predicate,
    # Matcher tree
    Matcher, FieldMatcher, Action, NestedMatcher, OnMatch,
    # String matchers
    ExactMatcher, PrefixMatcher, SuffixMatcher, ContainsMatcher, RegexMatcher,
    # Registry
    RegistryBuilder, Registry, register_core_matchers,
    # Config
    MatcherConfig, parse_matcher_config,
    # Constants
    MAX_DEPTH, MAX_FIELD_MATCHERS, MAX_PREDICATES_PER_COMPOUND,
    MAX_PATTERN_LENGTH, MAX_REGEX_PATTERN_LENGTH,
    # Errors
    MatcherError, UnknownTypeUrlError, InvalidConfigError,
    TooManyFieldMatchersError, TooManyPredicatesError, PatternTooLongError,
)
```

## Core Types

### MatchingData

```python
type MatchingData = str | int | bool | bytes | None
```

### DataInput Protocol

```python
class DataInput[Ctx]:
    def get(self, ctx: Ctx) -> MatchingData: ...
```

### InputMatcher Protocol

```python
class InputMatcher:
    def matches(self, value: MatchingData) -> bool: ...
```

### Matcher

```python
Matcher(
    matcher_list: tuple[FieldMatcher[Ctx, A], ...],
    on_no_match: OnMatch[Ctx, A] | None = None,
)

matcher.evaluate(ctx: Ctx) -> A | None
```

### FieldMatcher

```python
FieldMatcher(
    predicate: Predicate[Ctx],
    on_match: OnMatch[Ctx, A],
)
```

### OnMatch

```python
type OnMatch[Ctx, A] = Action[A] | NestedMatcher[Ctx, A]

Action(value: A)
NestedMatcher(matcher: Matcher[Ctx, A])
```

### Predicates

```python
SinglePredicate(input: DataInput[Ctx], matcher: InputMatcher)
And(predicates: tuple[Predicate[Ctx], ...])
Or(predicates: tuple[Predicate[Ctx], ...])
Not(predicate: Predicate[Ctx])

type Predicate[Ctx] = SinglePredicate[Ctx] | And[Ctx] | Or[Ctx] | Not[Ctx]
```

## String Matchers

| Class | Constructor | Matches |
|-------|------------|---------|
| `ExactMatcher(value)` | `ExactMatcher("hello")` | Exact string equality |
| `PrefixMatcher(prefix)` | `PrefixMatcher("/api")` | Starts with |
| `SuffixMatcher(suffix)` | `SuffixMatcher(".json")` | Ends with |
| `ContainsMatcher(substring)` | `ContainsMatcher("admin")` | Contains |
| `RegexMatcher(pattern)` | `RegexMatcher("^Bearer .+$")` | RE2 regex |

All matchers are frozen dataclasses.

## HTTP Domain

```python
from xuma.http import (
    HttpRequest, PathInput, MethodInput, HeaderInput, QueryParamInput,
    HttpRouteMatch, HttpPathMatch, HttpHeaderMatch, HttpQueryParamMatch,
    compile_route_matches, register,
)
```

### HttpRequest

```python
HttpRequest(
    method: str = "GET",
    raw_path: str = "/",
    headers: dict[str, str] | None = None,
    query_params: dict[str, str] | None = None,
)
```

### Inputs

| Class | Extracts | Returns |
|-------|----------|---------|
| `PathInput()` | Request path | `str` |
| `MethodInput()` | HTTP method | `str` |
| `HeaderInput(name)` | Header value | `str | None` |
| `QueryParamInput(name)` | Query parameter | `str | None` |

### Gateway API Compiler

```python
HttpRouteMatch(
    path: HttpPathMatch | None = None,
    method: str | None = None,
    headers: list[HttpHeaderMatch] = [],
    query_params: list[HttpQueryParamMatch] = [],
)

HttpPathMatch(type: "Exact" | "PathPrefix" | "RegularExpression", value: str)
HttpHeaderMatch(type: "Exact" | "RegularExpression", name: str, value: str)
HttpQueryParamMatch(type: "Exact" | "RegularExpression", name: str, value: str)

compile_route_matches(matches, action, on_no_match=None) -> Matcher
```

## Registry

```python
builder = RegistryBuilder()
builder = register_core_matchers(builder)
registry = builder.build()

matcher = registry.load_matcher(config)
```

### RegistryBuilder

```python
RegistryBuilder()
    .input(type_url, factory)       # Register a DataInput factory
    .matcher(type_url, factory)     # Register an InputMatcher factory
    .build() -> Registry
```

### Registry

```python
registry.load_matcher(config: MatcherConfig) -> Matcher
registry.contains_input(type_url: str) -> bool
registry.contains_matcher(type_url: str) -> bool
```

## Config Loading

```python
from xuma import parse_matcher_config

config = parse_matcher_config(json_dict)  # From dict
```

## Constants

| Constant | Value |
|----------|-------|
| `MAX_DEPTH` | 32 |
| `MAX_FIELD_MATCHERS` | 256 |
| `MAX_PREDICATES_PER_COMPOUND` | 256 |
| `MAX_PATTERN_LENGTH` | 8192 |
| `MAX_REGEX_PATTERN_LENGTH` | 4096 |

## Helpers

```python
matcher_from_predicate(predicate, action, on_no_match=None) -> Matcher
and_predicate(predicates, fallback) -> Predicate
or_predicate(predicates, fallback) -> Predicate
predicate_depth(predicate) -> int
```
