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
import { HttpRequest, compileRouteMatches } from "xuma/http";
import type { HttpRouteMatch } from "xuma/http";

// Route matches are plain objects — HttpRouteMatch and HttpPathMatch are
// interfaces, so there is nothing to construct.
const routes: HttpRouteMatch[] = [
    { path: { type: "PathPrefix", value: "/api" }, method: "GET" },
    { path: { type: "PathPrefix", value: "/admin" } },
];

const matcher = compileRouteMatches(routes, "matched", "404");

matcher.evaluate(new HttpRequest("GET", "/api/users"));  // "matched"
matcher.evaluate(new HttpRequest("POST", "/api/users")); // "404"
```

### Example 3: Config-Driven Matchers

```typescript
import { parseProtojson, RegistryBuilder } from "xuma";
import { register } from "xuma/testing";

const config = parseProtojson({
    matcherList: {
        matchers: [{
            predicate: {
                singlePredicate: {
                    input: {
                        name: "method",
                        typedConfig: { "@type": "type.googleapis.com/xuma.kv.v1.MapInput", key: "method" },
                    },
                    valueMatch: { exact: "GET" },
                },
            },
            onMatch: {
                action: {
                    name: "route-get",
                    typedConfig: { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", name: "route-get" },
                },
            },
        }],
    },
    onNoMatch: {
        action: {
            name: "fallback",
            typedConfig: { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", name: "fallback" },
        },
    },
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
