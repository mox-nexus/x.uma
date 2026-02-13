/**
 * Security tests — prototype pollution and error type consistency.
 *
 * These tests verify fixes from the arch-guild review (2026-02-08).
 */

import { describe, expect, it } from "bun:test";
import { compileRouteMatch } from "../src/http/gateway.ts";
import { HttpRequest } from "../src/http/request.ts";
import { MatcherError } from "../src/matcher.ts";
import { RegexMatcher } from "../src/string-matchers.ts";

describe("prototype pollution", () => {
	it("query param __proto__ does not pollute prototype", () => {
		const req = new HttpRequest("GET", "/?__proto__=evil");
		expect(req.queryParam("__proto__")).toBe("evil");
		// Verify Object.prototype was not modified
		expect(({} as Record<string, unknown>).__proto__).not.toBe("evil");
	});

	it("header named constructor returns correct value", () => {
		const req = new HttpRequest("GET", "/", { constructor: "custom-value" });
		expect(req.header("constructor")).toBe("custom-value");
	});

	it("missing query param returns null (not inherited property)", () => {
		const req = new HttpRequest("GET", "/?a=1");
		expect(req.queryParam("toString")).toBeNull();
		expect(req.queryParam("hasOwnProperty")).toBeNull();
	});

	it("missing header returns null (not inherited property)", () => {
		const req = new HttpRequest("GET", "/", { "x-custom": "value" });
		expect(req.header("toString")).toBeNull();
		expect(req.header("hasOwnProperty")).toBeNull();
	});
});

describe("RegexMatcher error guard", () => {
	it("valid regex does not throw", () => {
		expect(() => new RegexMatcher("^\\d+$")).not.toThrow();
	});
});

describe("gateway error types", () => {
	it("unknown path match type throws MatcherError", () => {
		expect(() =>
			compileRouteMatch(
				// biome-ignore lint/suspicious/noExplicitAny: testing invalid input
				{ path: { type: "Unknown" as any, value: "/api" } },
				"action",
			),
		).toThrow(MatcherError);
	});
});
