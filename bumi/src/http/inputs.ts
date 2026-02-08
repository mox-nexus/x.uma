import type { MatchingValue } from "../types.ts";
import type { HttpRequest } from "./request.ts";

/** Extract the path (without query string) from an HTTP request. */
export class PathInput {
	get(ctx: HttpRequest): MatchingValue {
		return ctx.path;
	}
}

/** Extract the HTTP method from a request. */
export class MethodInput {
	get(ctx: HttpRequest): MatchingValue {
		return ctx.method;
	}
}

/** Extract a header value by name (case-insensitive). */
export class HeaderInput {
	constructor(readonly name: string) {}

	get(ctx: HttpRequest): MatchingValue {
		return ctx.header(this.name);
	}
}

/** Extract a query parameter value by name. */
export class QueryParamInput {
	constructor(readonly name: string) {}

	get(ctx: HttpRequest): MatchingValue {
		return ctx.queryParam(this.name);
	}
}
