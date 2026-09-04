/**
 * Security tests — prototype pollution and error type consistency.
 *
 * These tests verify fixes from the arch-guild review (2026-02-08).
 */

import { describe, expect, it } from "bun:test";
import { TooManyFieldMatchersError, TooManyPredicatesError } from "../src/errors.ts";
import { compileRouteMatch, compileRouteMatches } from "../src/http/gateway.ts";
import type { HttpRouteMatch } from "../src/http/gateway.ts";
import { HttpRequest } from "../src/http/request.ts";
import { MAX_FIELD_MATCHERS, MAX_PREDICATES_PER_COMPOUND } from "../src/limits.ts";
import { Action, FieldMatcher, Matcher, MatcherError } from "../src/matcher.ts";
import { SinglePredicate } from "../src/predicate.ts";
import { ExactMatcher, RegexMatcher } from "../src/string-matchers.ts";
import { DictInput } from "../src/testing.ts";

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

/**
 * The gateway compiler enforces the widths, not just the loader.
 *
 * rumi moved these onto `Matcher::validate` in #32 so that every construction
 * path inherited them. bumi was not carried across: until 2026-08-23
 * `compileRouteMatches` accepted a compound predicate of any width, because
 * `validate()` checked depth only and the width limits lived in `registry.ts`.
 * A 257-route config compiled without complaint.
 */
describe("compiler width limits", () => {
	const route = (i: number): HttpRouteMatch => ({
		path: { type: "Exact", value: `/r${i}` },
	});

	it("rejects more routes than the limit", () => {
		const routes = Array.from({ length: MAX_PREDICATES_PER_COMPOUND + 1 }, (_, i) => route(i));
		expect(() => compileRouteMatches(routes, "hit")).toThrow(TooManyPredicatesError);
	});

	it("the width guard is not inert", () => {
		// A compiler that rejected everything would pass the test above.
		const routes = Array.from({ length: MAX_PREDICATES_PER_COMPOUND - 1 }, (_, i) => route(i));
		const matcher = compileRouteMatches(routes, "hit");
		expect(matcher.evaluate(new HttpRequest("GET", "/r0"))).toBe("hit");
	});

	it("matcher list width is enforced at construction", () => {
		const matchers = Array.from(
			{ length: MAX_FIELD_MATCHERS + 1 },
			(_, i) =>
				new FieldMatcher<Record<string, string>, string>(
					new SinglePredicate(new DictInput("k"), new ExactMatcher(`v${i}`)),
					new Action("hit"),
				),
		);
		expect(() => new Matcher(matchers)).toThrow(TooManyFieldMatchersError);
	});
});
