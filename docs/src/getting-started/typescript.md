# TypeScript Quick Start

Build an HTTP route matcher with `xuma` in 10 lines.

## Install

```bash
bun add xuma
```

Requires Bun runtime. The only runtime dependency is `re2js` for linear-time regex.

## Your First Matcher

Match requests by path prefix:

```typescript
import { Matcher, FieldMatcher, SinglePredicate, Action, PrefixMatcher } from "xuma";
import { HttpRequest, PathInput } from "xuma/http";

// Build a predicate: path starts with /api
const predicate = new SinglePredicate(
  new PathInput(),
  new PrefixMatcher("/api"),
);

// Build the matcher tree
const matcher = new Matcher<HttpRequest, string>(
  [new FieldMatcher(predicate, new Action("api_backend"))],
  new Action("default_backend"),
);

// Evaluate
const request = new HttpRequest("GET", "/api/users");
console.assert(matcher.evaluate(request) === "api_backend");

// No match falls through
const other = new HttpRequest("GET", "/other");
console.assert(matcher.evaluate(other) === "default_backend");
```

`Matcher` takes a list of `FieldMatcher`s (tried in order) and an optional fallback. First match wins.

## The Gateway API Compiler

The HTTP domain ships a compiler that builds matchers from Gateway API config:

```typescript
import { compileRouteMatches, HttpRequest } from "xuma/http";
import type { HttpRouteMatch } from "xuma/http";

// Declarative config
const routes: HttpRouteMatch[] = [
  {
    path: { type: "PathPrefix", value: "/api" },
    method: "GET",
  },
  {
    path: { type: "PathPrefix", value: "/admin" },
    method: "POST",
  },
];

// One call compiles all routes
const matcher = compileRouteMatches(routes, "allowed", "denied");

console.assert(matcher.evaluate(new HttpRequest("GET", "/api/users")) === "allowed");
console.assert(matcher.evaluate(new HttpRequest("DELETE", "/api/users")) === "denied");
```

Within a single `HttpRouteMatch`, all conditions are ANDed. Multiple routes are ORed. First match wins.

## Adding Header Conditions

```typescript
const route: HttpRouteMatch = {
  path: { type: "PathPrefix", value: "/api" },
  method: "POST",
  headers: [
    {
      type: "RegularExpression",
      name: "authorization",
      value: "^Bearer .+$",
    },
  ],
};
```

Regex uses `re2js` — linear time, no ReDoS vulnerability.

## Custom Action Types

Use discriminated unions or any TypeScript type:

```typescript
type RouteAction =
  | { type: "forward"; backend: string }
  | { type: "deny"; reason: string };

const matcher = compileRouteMatches<RouteAction>(
  routes,
  { type: "forward", backend: "api-service" },
  { type: "deny", reason: "no route matched" },
);

const result = matcher.evaluate(request);
if (result?.type === "forward") {
  console.log(`Forwarding to ${result.backend}`);
}
```

## Integration: Bun HTTP Server

```typescript
import { compileRouteMatches, HttpRequest } from "xuma/http";

const matcher = compileRouteMatches(
  [{ path: { type: "PathPrefix", value: "/api" }, method: "GET" }],
  "allowed",
  "denied",
);

Bun.serve({
  port: 3000,
  fetch(req) {
    const url = new URL(req.url);
    const request = new HttpRequest(
      req.method,
      url.pathname + url.search,
      Object.fromEntries(req.headers),
    );
    if (matcher.evaluate(request) === "denied") {
      return new Response("Not found", { status: 404 });
    }
    return new Response("OK");
  },
});
```

## Safety

- **ReDoS protection** — `re2js` guarantees linear-time regex matching.
- **Immutable** — all types use `readonly` fields.
- **Depth limits** — nested matchers capped at 32 levels.
- **Fail-closed** — missing data from `DataInput` returns `null`, which makes the predicate evaluate to `false`.

## Next Steps

- [The Matching Pipeline](../concepts/pipeline.md) — how data flows through the matcher
- [Build an HTTP Router](../tutorials/http-router.md) — full routing with headers and query params
- [HTTP Matching](../domains/http.md) — all inputs, config types, and the compiler
