# TypeScript API Reference

bumi implements the xDS Unified Matcher API in pure TypeScript. Zero runtime dependencies. Bun runtime. TypeScript 5+.

**Package:** `bumi` (from `bumi/` directory)

**Installation:**
```bash
bun add bumi
```

## Import Hierarchy

All public types exported flat from top level:

```typescript
import {
    // Protocols
    DataInput, InputMatcher, MatchingData,
    // Predicates
    SinglePredicate, And, Or, Not, Predicate, evaluatePredicate, predicateDepth,
    // Matcher
    Matcher, FieldMatcher, OnMatch, Action, NestedMatcher, MatcherError, MAX_DEPTH,
    // String matchers
    ExactMatcher, PrefixMatcher, SuffixMatcher, ContainsMatcher, RegexMatcher,
} from "bumi";

import {
    // Context
    HttpRequest,
    // DataInputs
    PathInput, MethodInput, HeaderInput, QueryParamInput,
    // Gateway API types
    HttpPathMatch, HttpHeaderMatch, HttpQueryParamMatch, HttpRouteMatch,
    compileRouteMatch, compileRouteMatches,
} from "bumi/http";
```

## Type Hierarchy

```
┌─────────────────────────────────────┐
│          Matcher<Ctx, A>            │
│   Top-level tree, returns A|null    │
└───┬─────────────────────────────────┘
    │ contains
    ├──► FieldMatcher<Ctx, A>
    │       predicate + onMatch
    │
    └──► OnMatch<Ctx, A>  (fallback)
         ├─ Action<A>
         └─ NestedMatcher<Ctx, A>

┌─────────────────────────────────────┐
│         Predicate<Ctx>              │
│      Boolean logic tree             │
└───┬─────────────────────────────────┘
    ├─ SinglePredicate<Ctx> → input + matcher
    ├─ And<Ctx> → all match
    ├─ Or<Ctx> → any match
    └─ Not<Ctx> → invert

┌─────────────────────────────────────┐
│    DataInput<Ctx> interface         │
│   extract MatchingData from Ctx     │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│    InputMatcher interface           │
│   match MatchingData → bool         │
└───┬─────────────────────────────────┘
    ├─ ExactMatcher
    ├─ PrefixMatcher
    ├─ SuffixMatcher
    ├─ ContainsMatcher
    └─ RegexMatcher
```

## Core Interfaces

### MatchingData

```typescript
type MatchingData = string | number | boolean | Uint8Array | null;
```

Type-erased value returned by `DataInput.get()`. Replaces Rust's `MatchingData` enum.

Returning `null` triggers the **null → false invariant**: predicate evaluates to `false` without consulting the matcher.

### `DataInput<Ctx>`

```typescript
interface DataInput<Ctx> {
    get(ctx: Ctx): MatchingData;
}
```

Domain-specific extraction port. Implementations:
- `PathInput` extracts HTTP path
- `HeaderInput` extracts HTTP header by name
- Custom: implement this interface for your domain

**Generic over Ctx** — accepts context of type `Ctx`.

### InputMatcher

```typescript
interface InputMatcher {
    matches(value: MatchingData): boolean;
}
```

Domain-agnostic matching port. Non-generic by design — same `ExactMatcher` works for HTTP, test contexts, or any custom domain.

## Predicates

### `SinglePredicate<Ctx>`

```typescript
class SinglePredicate<Ctx> {
    constructor(
        readonly input: DataInput<Ctx>,
        readonly matcher: InputMatcher,
    );

    evaluate(ctx: Ctx): boolean;
}
```

Combines extraction + matching. Enforces the **null → false invariant**.

**Example:**
```typescript
const pred = new SinglePredicate(
    new PathInput(),
    new PrefixMatcher("/api")
);
pred.evaluate(request); // true if path starts with /api
```

### `And<Ctx>`

```typescript
class And<Ctx> {
    constructor(readonly predicates: readonly Predicate<Ctx>[]);

    evaluate(ctx: Ctx): boolean;
}
```

All predicates must match. Short-circuits on first `false`. Empty array returns `true` (vacuous truth).

### `Or<Ctx>`

```typescript
class Or<Ctx> {
    constructor(readonly predicates: readonly Predicate<Ctx>[]);

    evaluate(ctx: Ctx): boolean;
}
```

Any predicate must match. Short-circuits on first `true`. Empty array returns `false`.

### `Not<Ctx>`

```typescript
class Not<Ctx> {
    constructor(readonly predicate: Predicate<Ctx>);

    evaluate(ctx: Ctx): boolean;
}
```

Inverts inner predicate result.

### `Predicate<Ctx>`

```typescript
type Predicate<Ctx> = SinglePredicate<Ctx> | And<Ctx> | Or<Ctx> | Not<Ctx>;
```

Discriminated union of all predicate types. Check variant with `instanceof`.

### evaluatePredicate()

```typescript
function evaluatePredicate<Ctx>(p: Predicate<Ctx>, ctx: Ctx): boolean;
```

Evaluate any predicate variant. Dispatches to the appropriate `.evaluate()` method.

### predicateDepth()

```typescript
function predicateDepth<Ctx>(p: Predicate<Ctx>): number;
```

Calculate nesting depth of predicate tree. Used by `Matcher.validate()` for depth limit enforcement.

## Matcher Tree

### `Matcher<Ctx, A>`

```typescript
class Matcher<Ctx, A> {
    constructor(
        readonly matchers: readonly FieldMatcher<Ctx, A>[],
        readonly onNoMatch: OnMatch<Ctx, A> | null = null,
    );

    evaluate(ctx: Ctx): A | null;
    validate(): void;
    depth(): number;
}
```

Top-level matcher tree. Evaluates `matchers` in order (first-match-wins). Returns action `A` or `null`.

**Auto-validation:** `validate()` is called in constructor. Trees exceeding `MAX_DEPTH` (32) throw `MatcherError`.

**Methods:**
- `evaluate(ctx)` — Returns first matching action or `null`
- `validate()` — Checks depth limit (called automatically)
- `depth()` — Returns total tree depth

**Example:**
```typescript
const matcher = new Matcher([
    new FieldMatcher(
        new SinglePredicate(new PathInput(), new PrefixMatcher("/api")),
        new Action("api")
    ),
], new Action("default"));

const action = matcher.evaluate(request); // "api" or "default"
```

### `FieldMatcher<Ctx, A>`

```typescript
class FieldMatcher<Ctx, A> {
    constructor(
        readonly predicate: Predicate<Ctx>,
        readonly onMatch: OnMatch<Ctx, A>,
    );
}
```

Pairs a predicate with an outcome (action or nested matcher).

### `OnMatch<Ctx, A>`

```typescript
type OnMatch<Ctx, A> = Action<A> | NestedMatcher<Ctx, A>;
```

xDS exclusivity — action XOR nested matcher, never both. Check variant with `instanceof`.

### `Action<A>`

```typescript
class Action<A> {
    constructor(readonly value: A);
}
```

Terminal outcome. Returns `value` when matched.

### `NestedMatcher<Ctx, A>`

```typescript
class NestedMatcher<Ctx, A> {
    constructor(readonly matcher: Matcher<Ctx, A>);
}
```

Continue evaluation into nested matcher. If nested matcher returns `null`, evaluation continues to next `FieldMatcher` (xDS nested matcher failure propagation).

### MatcherError

```typescript
class MatcherError extends Error {
    constructor(message: string);
}
```

Thrown when `validate()` detects depth exceeding `MAX_DEPTH`, or when compiling invalid regex patterns.

### MAX_DEPTH

```typescript
const MAX_DEPTH: number = 32;
```

Maximum allowed matcher tree depth. Enforced at construction time.

## String Matchers

All matchers are classes with `readonly` properties implementing `InputMatcher` interface. Return `false` for non-string or `null` input.

### ExactMatcher

```typescript
class ExactMatcher {
    constructor(
        readonly value: string,
        readonly ignoreCase: boolean = false,
    );

    matches(value: MatchingData): boolean;
}
```

Exact string equality. When `ignoreCase=true`, comparison uses `.toLowerCase()`.

**Optimization:** Pattern is pre-lowercased at construction time (private `cmpValue` field).

### PrefixMatcher

```typescript
class PrefixMatcher {
    constructor(
        readonly prefix: string,
        readonly ignoreCase: boolean = false,
    );

    matches(value: MatchingData): boolean;
}
```

String starts with prefix. Pre-lowercased at construction when `ignoreCase=true`.

### SuffixMatcher

```typescript
class SuffixMatcher {
    constructor(
        readonly suffix: string,
        readonly ignoreCase: boolean = false,
    );

    matches(value: MatchingData): boolean;
}
```

String ends with suffix. Pre-lowercased at construction when `ignoreCase=true`.

### ContainsMatcher

```typescript
class ContainsMatcher {
    constructor(
        readonly substring: string,
        readonly ignoreCase: boolean = false,
    );

    matches(value: MatchingData): boolean;
}
```

Substring search. Pre-lowercased at construction when `ignoreCase=true` (Knuth optimization: avoid repeated pattern lowercasing).

### RegexMatcher

```typescript
class RegexMatcher {
    constructor(readonly pattern: string);

    matches(value: MatchingData): boolean;
}
```

Regular expression search (not fullmatch). Pattern compiled at construction time. Uses `RegExp.test()`.

**Security:** Uses JavaScript's `RegExp` engine (backtracking, ReDoS-vulnerable). See Performance > ReDoS Protection in the docs. For adversarial input, use `bumi-crusty` (Phase 8).

## HTTP Domain

### HttpRequest

```typescript
class HttpRequest {
    constructor(
        readonly method: string = "GET",
        readonly rawPath: string = "/",
        readonly headers: Readonly<Record<string, string>> = {},
    );

    readonly path: string;
    readonly queryParams: Readonly<Record<string, string>>;

    header(name: string): string | null;
    queryParam(name: string): string | null;
}
```

HTTP request context for matching.

**Parsing:** Query string automatically parsed from `rawPath` at construction. Headers stored lowercased for case-insensitive lookup.

**Properties:**
- `path` — path without query string
- `queryParams` — parsed query parameters

**Methods:**
- `header(name)` — Case-insensitive header lookup
- `queryParam(name)` — Query parameter lookup

**Example:**
```typescript
const req = new HttpRequest(
    "GET",
    "/api/users?role=admin",
    { "Content-Type": "application/json" }
);

req.path;              // "/api/users"
req.queryParams;       // { role: "admin" }
req.header("content-type"); // "application/json" (case-insensitive)
req.queryParam("role"); // "admin"
```

### DataInputs

#### PathInput

```typescript
class PathInput {
    get(ctx: HttpRequest): MatchingData;
}
```

Extracts `ctx.path` (without query string).

#### MethodInput

```typescript
class MethodInput {
    get(ctx: HttpRequest): MatchingData;
}
```

Extracts HTTP method (case-sensitive).

#### HeaderInput

```typescript
class HeaderInput {
    constructor(readonly name: string);

    get(ctx: HttpRequest): MatchingData;
}
```

Extracts header value by name (case-insensitive lookup). Returns `null` if header not present.

#### QueryParamInput

```typescript
class QueryParamInput {
    constructor(readonly name: string);

    get(ctx: HttpRequest): MatchingData;
}
```

Extracts query parameter value by name. Returns `null` if parameter not present.

### Gateway API Types

Pure TypeScript types mirroring Gateway API spec (no Kubernetes dependency).

#### HttpPathMatch

```typescript
interface HttpPathMatch {
    readonly type: "Exact" | "PathPrefix" | "RegularExpression";
    readonly value: string;
}
```

Path match specification.

#### HttpHeaderMatch

```typescript
interface HttpHeaderMatch {
    readonly type: "Exact" | "RegularExpression";
    readonly name: string;
    readonly value: string;
}
```

Header match specification.

#### HttpQueryParamMatch

```typescript
interface HttpQueryParamMatch {
    readonly type: "Exact" | "RegularExpression";
    readonly name: string;
    readonly value: string;
}
```

Query parameter match specification.

#### HttpRouteMatch

```typescript
interface HttpRouteMatch {
    readonly path?: HttpPathMatch;
    readonly method?: string;
    readonly headers?: readonly HttpHeaderMatch[];
    readonly queryParams?: readonly HttpQueryParamMatch[];
}
```

Gateway API route match config. All conditions within a single `HttpRouteMatch` are ANDed.

### compileRouteMatch()

```typescript
function compileRouteMatch<A>(
    routeMatch: HttpRouteMatch,
    action: A,
): Matcher<HttpRequest, A>;
```

Compile a single `HttpRouteMatch` into a `Matcher`.

**Example:**
```typescript
const matcher = compileRouteMatch(
    { path: { type: "PathPrefix", value: "/api" } },
    "api_handler"
);
```

### compileRouteMatches()

```typescript
function compileRouteMatches<A>(
    matches: readonly HttpRouteMatch[],
    action: A,
    onNoMatch?: A,
): Matcher<HttpRequest, A>;
```

Compile multiple `HttpRouteMatch` entries into a single `Matcher`. Multiple matches are ORed per Gateway API semantics. Empty `matches` array creates a catch-all matcher.

**Example:**
```typescript
const apiRoute: HttpRouteMatch = {
    path: { type: "PathPrefix", value: "/api" },
    method: "GET"
};

const adminRoute: HttpRouteMatch = {
    headers: [{ type: "Exact", name: "x-admin", value: "true" }]
};

const matcher = compileRouteMatches(
    [apiRoute, adminRoute],
    "matched",
    "not_found"
);

matcher.evaluate(request); // "matched" or "not_found"
```

## Requirements

- **Bun runtime** — uses Bun's fast module resolution and test runner
- **TypeScript 5+** — uses modern TS features (generic classes, discriminated unions)
- **Zero runtime dependencies** — stdlib only (no lodash, no ramda)
- Development dependencies: Biome (lint/format), js-yaml (conformance tests)

## Security

See Performance > ReDoS Protection in the docs for regex security details.

**Summary:**
- `RegexMatcher` uses JavaScript's `RegExp` (backtracking, ReDoS-vulnerable)
- Safe for trusted patterns (your route config, known fixtures)
- For adversarial input, use `bumi-crusty` (Rust-backed via WASM, linear-time regex)
- Depth validation automatic at construction (max 32 levels)
- Query string parsing uses simple split (no complex URL parsing vulnerabilities)
- Headers stored with `Object.create(null)` to prevent prototype pollution
