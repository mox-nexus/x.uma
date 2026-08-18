import type { MatchingData } from "../types.ts";
import type { HttpRequest } from "./request.ts";

/** Extract the path (without query string) from an HTTP request. */
export class PathInput {
	get(ctx: HttpRequest): MatchingData {
		return ctx.path;
	}
}

/** Extract the HTTP method from a request. */
export class MethodInput {
	get(ctx: HttpRequest): MatchingData {
		return ctx.method;
	}
}

/** Extract a header value by name (case-insensitive). */
export class HeaderInput {
	constructor(readonly name: string) {}

	get(ctx: HttpRequest): MatchingData {
		return ctx.header(this.name);
	}
}

/** Extract a query parameter value by name. */
export class QueryParamInput {
	constructor(readonly name: string) {}

	get(ctx: HttpRequest): MatchingData {
		return ctx.queryParam(this.name);
	}
}

/**
 * Extract the `:authority` pseudo-header (Host in HTTP/1).
 *
 * Read from the headers, which is where rumi's `HttpMessage` reads it from
 * too. Without this and `SchemeInput`, four of the six `xuma.http.v1.*` type
 * URLs resolved here and six resolved in rumi — a type URL that loads in one
 * implementation and not another.
 */
export class AuthorityInput {
	get(ctx: HttpRequest): MatchingData {
		return ctx.header(":authority");
	}
}

/** Extract the `:scheme` pseudo-header (http/https). */
export class SchemeInput {
	get(ctx: HttpRequest): MatchingData {
		return ctx.header(":scheme");
	}
}
