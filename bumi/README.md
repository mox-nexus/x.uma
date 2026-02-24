# xuma — TypeScript xDS Matcher

**v0.0.2** — Part of the [x.uma](https://github.com/mox-nexus/x.uma) matcher engine.

xuma is a pure TypeScript implementation of the xDS Unified Matcher API. Match structured data (HTTP requests, events, messages) against rule trees with first-match-wins semantics. Runs on Bun.

## Installation

```bash
bun add xuma
```

## Examples

### Example 1: Match a Dictionary Value

```typescript
import { Matcher, FieldMatcher, SinglePredicate, ExactMatcher, Action } from "xuma";
import type { DataInput, MatchingData } from "xuma";

// 1. Define a data input (extraction port)
class DictInput implements DataInput<Record<string, string>> {
    constructor(private key: string) {}
    get(ctx: Record<string, string>): MatchingData {
        return ctx[this.key] ?? null;
    }
}

// 2. Build a matcher tree
const matcher = new Matcher(
    [
        new FieldMatcher(
            new SinglePredicate(new DictInput("name"), new ExactMatcher("alice")),
            new Action("admin"),
        ),
        new FieldMatcher(
            new SinglePredicate(new DictInput("name"), new ExactMatcher("bob")),
            new Action("user"),
        ),
    ],
    new Action("guest"),
);

// 3. Evaluate
matcher.evaluate({ name: "alice" }); // "admin"
matcher.evaluate({ name: "bob" });   // "user"
matcher.evaluate({ name: "eve" });   // "guest"
```

### Example 2: HTTP Route Matching

```typescript
import { HttpRequest, HttpRouteMatch, HttpPathMatch, compileRouteMatches } from "xuma/http";

const matcher = compileRouteMatches(
    [
        new HttpRouteMatch({ path: new HttpPathMatch("PathPrefix", "/api"), method: "GET" }),
        new HttpRouteMatch({ path: new HttpPathMatch("PathPrefix", "/admin") }),
    ],
    "matched",
    "404",
);

matcher.evaluate(new HttpRequest({ method: "GET", rawPath: "/api/users" })); // "matched"
matcher.evaluate(new HttpRequest({ method: "POST", rawPath: "/api/users" })); // "404"
```

### Example 3: Config-Driven Matchers

```typescript
import { parseMatcherConfig, RegistryBuilder } from "xuma";
import { register } from "xuma/testing";

const config = parseMatcherConfig({
    matchers: [{
        predicate: {
            type: "single",
            input: { type_url: "xuma.test.v1.StringInput", config: { key: "method" } },
            value_match: { Exact: "GET" },
        },
        on_match: { type: "action", action: "route-get" },
    }],
    on_no_match: { type: "action", action: "fallback" },
});

const builder = new RegistryBuilder<Record<string, string>>();
register(builder);
const matcher = builder.build().loadMatcher(config);

matcher.evaluate({ method: "GET" });    // "route-get"
matcher.evaluate({ method: "DELETE" }); // "fallback"
```

## Security

`RegexMatcher` uses `re2js` (linear-time, ReDoS-safe).

## Requirements

- Bun 1.0+

## License

MIT OR Apache-2.0
