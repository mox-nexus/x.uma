# TypeScript Quick Start

Build an HTTP route matcher in 10 lines.

x.uma's TypeScript implementation (`bumi`) translates Gateway API route configuration into efficient runtime matchers. Routes defined at config time become compiled trees evaluated at request time.

## Install

Add `bumi` to your project:

```bash
bun add @x.uma/bumi
```

**Requires Bun runtime** — bumi uses Bun's fast startup and zero-overhead module system. It will not run on Node.js without transpilation.

Alternatively, import from source if you're working in the x.uma monorepo:

```typescript
import { Matcher, compile_route_matches } from "bumi";
```

## Your First Matcher

Match GET requests to `/api/*` and POST requests to `/admin/*`:

```typescript
import {
	type HttpRouteMatch,
	HttpRequest,
	compileRouteMatches,
} from "@x.uma/bumi/http";

// Define routes using Gateway API syntax
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

// Compile routes into a matcher
const matcher = compileRouteMatches(
	routes,
	"allowed",  // action when any route matches
	"denied",   // action when no routes match
);

// Evaluate against requests
let request = new HttpRequest("GET", "/api/users");
let result = matcher.evaluate(request);
console.assert(result === "allowed");

request = new HttpRequest("DELETE", "/api/users");
result = matcher.evaluate(request);
console.assert(result === "denied"); // DELETE not in routes
```

The `compileRouteMatches` function is the high-level API. It takes a list of `HttpRouteMatch` configs and produces a `Matcher<HttpRequest, A>` that runs in microseconds.

## How Compilation Works

Gateway API route configuration is declarative. You specify what to match, not how to evaluate it:

```typescript
import type {
	HttpRouteMatch,
	HttpPathMatch,
	HttpHeaderMatch,
} from "@x.uma/bumi/http";

const route: HttpRouteMatch = {
	path: { type: "PathPrefix", value: "/api" },
	method: "GET",
	headers: [
		{
			type: "Exact",
			name: "content-type",
			value: "application/json",
		},
	],
};
```

All conditions within a single `HttpRouteMatch` are ANDed together. When you pass multiple `HttpRouteMatch` entries to `compileRouteMatches`, they are ORed.

The compiler produces a tree of predicates:

```
Matcher
└── FieldMatcher
    └── Or
        ├── And(path=/api, method=GET, header=content-type)
        └── And(path=/admin, method=POST)
```

At evaluation time, the tree walks first-match-wins until a predicate succeeds.

## Under the Hood: Manual Construction

The Gateway API compiler is syntactic sugar. Here's what it generates:

```typescript
import {
	Matcher,
	FieldMatcher,
	Action,
	SinglePredicate,
	And,
} from "@x.uma/bumi";
import {
	ExactMatcher,
	PrefixMatcher,
} from "@x.uma/bumi";
import {
	HttpRequest,
	PathInput,
	MethodInput,
} from "@x.uma/bumi/http";

// Manual construction of the same matcher
const matcher = new Matcher<HttpRequest, string>([
	new FieldMatcher(
		new And([
			new SinglePredicate(
				new PathInput(),
				new PrefixMatcher("/api"),
			),
			new SinglePredicate(
				new MethodInput(),
				new ExactMatcher("GET"),
			),
		]),
		new Action("allowed"),
	),
	new FieldMatcher(
		new SinglePredicate(
			new PathInput(),
			new PrefixMatcher("/admin"),
		),
		new Action("allowed"),
	),
], new Action("denied"));
```

This is verbose but explicit. You control the exact tree structure.

**When to use manual construction:**
- Building matchers programmatically from non-Gateway-API configs
- Implementing custom `DataInput` or `InputMatcher` interfaces
- Debugging compilation behavior

**When to use the compiler:**
- Standard HTTP routing (99% of use cases)
- Gateway API configurations from Kubernetes
- Less code, same performance

## The HttpRequest Context

`HttpRequest` is a readonly class with parsed query parameters and lowercased headers:

```typescript
import { HttpRequest } from "@x.uma/bumi/http";

// Query string parsed from rawPath
const request = new HttpRequest(
	"GET",
	"/search?q=hello&lang=en",
	{ "Content-Type": "application/json" },
);

console.assert(request.path === "/search");
console.assert(request.queryParam("q") === "hello");
console.assert(request.queryParam("lang") === "en");

// Headers are case-insensitive
console.assert(request.header("content-type") === "application/json");
console.assert(request.header("Content-Type") === "application/json");
```

The query string is parsed once at construction time (O(n) scan). Every `queryParam()` lookup is O(1) dictionary access.

Headers are stored lowercased to avoid repeated string operations during matching.

## The Readonly Pattern

bumi uses readonly classes instead of interfaces to ensure immutability:

```typescript
export class Matcher<Ctx, A> {
	constructor(
		readonly matcherList: readonly FieldMatcher<Ctx, A>[],
		readonly onNoMatch: Action<A> | null = null,
	) {}

	evaluate(ctx: Ctx): A | null {
		// ...
	}
}
```

Every field is `readonly`. Arrays are `readonly T[]`. This prevents accidental mutation after construction.

TypeScript's type system enforces immutability at compile time. At runtime, there's no additional overhead compared to mutable objects.

## Actions: Beyond Strings

The examples use `string` actions for simplicity. In production, actions are often objects or discriminated unions:

```typescript
type RouteAction =
	| { type: "forward"; backend: string; weight: number }
	| { type: "redirect"; location: string; status: number }
	| { type: "deny"; reason: string };

const matcher = compileRouteMatches(
	routes,
	{ type: "forward", backend: "api-service", weight: 100 },
	{ type: "deny", reason: "no route matched" },
);

const result = matcher.evaluate(request);
if (result?.type === "forward") {
	console.log(`Forwarding to ${result.backend}`);
}
```

The action type can be any TypeScript type. The matcher returns it by reference when a match succeeds.

## Validation and Safety

Matchers enforce safety constraints:

- **Depth limit**: Nested matchers cannot exceed 32 levels (prevents stack overflow)
- **Immutability**: All types are readonly (no accidental mutation)
- **Type safety**: TypeScript catches type mismatches at compile time

Check depth limits with `validate()`:

```typescript
try {
	matcher.validate();
	console.log("Matcher is valid");
} catch (e) {
	if (e instanceof MatcherError) {
		console.error(`Validation failed: ${e.message}`);
	}
}
```

The Gateway API compiler produces valid trees. Manual construction can violate depth limits if you nest `NestedMatcher` recursively.

## Performance Notes

At 200 routing rules, `bumi` evaluates worst-case (last rule matches) in 2.1 microseconds on Apple M1 Max. That's 475,000 requests per second per core.

**Why is TypeScript faster than Rust?** This isn't a language speed contest. The Rust implementation uses `Box<dyn InputMatcher>` for extensibility, which requires vtable dispatch. The TypeScript JIT (JavaScriptCore in Bun) sees the monomorphic call site and inlines the comparison directly.

For simple operations (exact string match), bumi is 3.5x faster than rumi. For regex matching or complex predicates, the gap narrows. At ReDoS-vulnerable patterns, Rust's linear-time regex is 6.5 million times faster.

**Thread safety**: Bun is single-threaded. If you need parallelism, spawn worker threads:

```typescript
import { Worker } from "bun";

const worker = new Worker("./matcher-worker.ts");
worker.postMessage({ method: "GET", path: "/api/users" });
```

Each worker gets its own JavaScript heap and its own matcher instance.

For ReDoS protection with untrusted regex input, use `bumi-crusty` instead of pure `bumi`. See [ReDoS Protection](../performance/redos.md).

## Integration Examples

### Cloudflare Workers

```typescript
import { compileRouteMatches, HttpRequest } from "@x.uma/bumi/http";

const matcher = compileRouteMatches([
	{ path: { type: "PathPrefix", value: "/api" }, method: "GET" },
], "allowed", "denied");

export default {
	async fetch(request: Request): Promise<Response> {
		const url = new URL(request.url);
		const req = new HttpRequest(
			request.method,
			url.pathname,
			Object.fromEntries(request.headers),
		);

		const action = matcher.evaluate(req);

		if (action === "denied") {
			return new Response("Access denied", { status: 403 });
		}

		return new Response("OK");
	},
};
```

### Bun HTTP Server

```typescript
import { compileRouteMatches, HttpRequest } from "@x.uma/bumi/http";

const matcher = compileRouteMatches([
	{ path: { type: "PathPrefix", value: "/api" }, method: "GET" },
], "allowed", "denied");

Bun.serve({
	port: 3000,
	fetch(req) {
		const url = new URL(req.url);
		const request = new HttpRequest(
			req.method,
			url.pathname + url.search,
			Object.fromEntries(req.headers),
		);

		const action = matcher.evaluate(request);

		if (action === "denied") {
			return new Response("Access denied", { status: 403 });
		}

		return new Response("OK");
	},
});
```

## Next Steps

- [Build an HTTP Router](../tutorials/http-router.md) — full routing patterns
- [Predicate Composition](../concepts/predicates.md) — AND/OR/NOT logic
- [Benchmark Results](../performance/benchmarks.md) — performance deep dive
- [TypeScript API Reference](../reference/typescript.md) — complete type documentation
