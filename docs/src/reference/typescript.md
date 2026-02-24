# TypeScript API Reference

## Installation

```bash
bun add xuma
```

Requires Bun runtime. Dependency: `re2js` for linear-time regex.

## Package: xuma

```typescript
import {
  // Core types
  type DataInput, type InputMatcher, type MatchingData,
  // Predicates
  SinglePredicate, And, Or, Not, type Predicate,
  // Matcher tree
  Matcher, FieldMatcher, Action, NestedMatcher, type OnMatch,
  // String matchers
  ExactMatcher, PrefixMatcher, SuffixMatcher, ContainsMatcher, RegexMatcher,
  // Registry
  RegistryBuilder, Registry,
  // Config
  type MatcherConfig, parseMatcherConfig,
  // Constants
  MAX_DEPTH, MAX_FIELD_MATCHERS, MAX_PREDICATES_PER_COMPOUND,
  MAX_PATTERN_LENGTH, MAX_REGEX_PATTERN_LENGTH,
  // Errors
  MatcherError, UnknownTypeUrlError, InvalidConfigError,
  TooManyFieldMatchersError, TooManyPredicatesError, PatternTooLongError,
} from "xuma";
```

## Core Types

### MatchingData

```typescript
type MatchingData = string | number | boolean | Uint8Array | null;
```

### DataInput

```typescript
interface DataInput<Ctx> {
  get(ctx: Ctx): MatchingData;
}
```

### InputMatcher

```typescript
interface InputMatcher {
  matches(value: MatchingData): boolean;
}
```

### Matcher

```typescript
class Matcher<Ctx, A> {
  constructor(matchers: FieldMatcher<Ctx, A>[], onNoMatch?: OnMatch<Ctx, A>);
  evaluate(ctx: Ctx): A | null;
}
```

### FieldMatcher

```typescript
class FieldMatcher<Ctx, A> {
  constructor(predicate: Predicate<Ctx>, onMatch: OnMatch<Ctx, A>);
}
```

### OnMatch

```typescript
type OnMatch<Ctx, A> = Action<A> | NestedMatcher<Ctx, A>;

class Action<A> { constructor(value: A); }
class NestedMatcher<Ctx, A> { constructor(matcher: Matcher<Ctx, A>); }
```

### Predicates

```typescript
class SinglePredicate<Ctx> { constructor(input: DataInput<Ctx>, matcher: InputMatcher); }
class And<Ctx> { constructor(predicates: Predicate<Ctx>[]); }
class Or<Ctx> { constructor(predicates: Predicate<Ctx>[]); }
class Not<Ctx> { constructor(predicate: Predicate<Ctx>); }

type Predicate<Ctx> = SinglePredicate<Ctx> | And<Ctx> | Or<Ctx> | Not<Ctx>;
```

## String Matchers

| Class | Constructor | Matches |
|-------|------------|---------|
| `new ExactMatcher(value)` | `new ExactMatcher("hello")` | Exact equality |
| `new PrefixMatcher(prefix)` | `new PrefixMatcher("/api")` | Starts with |
| `new SuffixMatcher(suffix)` | `new SuffixMatcher(".json")` | Ends with |
| `new ContainsMatcher(sub)` | `new ContainsMatcher("admin")` | Contains |
| `new RegexMatcher(pattern)` | `new RegexMatcher("^Bearer .+$")` | RE2 regex |

All types use `readonly` fields.

## HTTP Domain

```typescript
import {
  HttpRequest, PathInput, MethodInput, HeaderInput, QueryParamInput,
  compileRouteMatch, compileRouteMatches, register,
  type HttpRouteMatch, type HttpPathMatch, type HttpHeaderMatch, type HttpQueryParamMatch,
} from "xuma/http";
```

### HttpRequest

```typescript
class HttpRequest {
  constructor(method: string, rawPath: string, headers?: Record<string, string>,
              queryParams?: Record<string, string>);
}
```

### Inputs

| Class | Extracts | Returns |
|-------|----------|---------|
| `new PathInput()` | Request path | `string` |
| `new MethodInput()` | HTTP method | `string` |
| `new HeaderInput(name)` | Header value | `string \| null` |
| `new QueryParamInput(name)` | Query parameter | `string \| null` |

### Gateway API Compiler

```typescript
interface HttpRouteMatch {
  readonly path?: HttpPathMatch;
  readonly method?: string;
  readonly headers?: readonly HttpHeaderMatch[];
  readonly queryParams?: readonly HttpQueryParamMatch[];
}

interface HttpPathMatch {
  readonly type: "Exact" | "PathPrefix" | "RegularExpression";
  readonly value: string;
}

interface HttpHeaderMatch {
  readonly type: "Exact" | "RegularExpression";
  readonly name: string;
  readonly value: string;
}

interface HttpQueryParamMatch {
  readonly type: "Exact" | "RegularExpression";
  readonly name: string;
  readonly value: string;
}

function compileRouteMatches<A>(matches: HttpRouteMatch[], action: A, onNoMatch?: A): Matcher<HttpRequest, A>;
function compileRouteMatch<A>(routeMatch: HttpRouteMatch, action: A): Matcher<HttpRequest, A>;
```

## Registry

```typescript
const builder = new RegistryBuilder<MyContext>();
// Register inputs and matchers...
const registry = builder.build();
const matcher = registry.loadMatcher(config);
```

### RegistryBuilder

```typescript
class RegistryBuilder<Ctx> {
  input(typeUrl: string, factory: InputFactory<Ctx>): this;
  matcher(typeUrl: string, factory: MatcherFactory): this;
  build(): Registry<Ctx>;
}
```

### Registry

```typescript
class Registry<Ctx> {
  loadMatcher<A>(config: MatcherConfig<A>): Matcher<Ctx, A>;
  containsInput(typeUrl: string): boolean;
  containsMatcher(typeUrl: string): boolean;
}
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

```typescript
function matcherFromPredicate<Ctx, A>(predicate: Predicate<Ctx>, action: A, onNoMatch?: A): Matcher<Ctx, A>;
function andPredicate<Ctx>(predicates: Predicate<Ctx>[], fallback: Predicate<Ctx>): Predicate<Ctx>;
function orPredicate<Ctx>(predicates: Predicate<Ctx>[], fallback: Predicate<Ctx>): Predicate<Ctx>;
function evaluatePredicate<Ctx>(predicate: Predicate<Ctx>, ctx: Ctx): boolean;
function predicateDepth<Ctx>(predicate: Predicate<Ctx>): number;
```
