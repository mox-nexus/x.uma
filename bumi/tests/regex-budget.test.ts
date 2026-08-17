/**
 * Regression: SEC1 / security review F-01 — the regex compile bomb.
 *
 * `re2js` implements neither of C++ RE2's compile-time guards. Measured on
 * re2js 0.4.3 before the fix, through bumi's own `RegexMatcher`:
 *
 * ```
 * a{100}                 6 chars     2ms     35MB
 * (a{100}){100}         13 chars     5ms     48MB
 * ((a{100}){100}){100}  20 chars   282ms    286MB
 * ```
 *
 * Twenty characters of config. One more nesting level reaches seconds and
 * gigabytes. `MAX_REGEX_PATTERN_LENGTH` bounds pattern *length*, which is the
 * wrong axis — cost tracks compiled program size.
 *
 * puma is immune (google-re2 rejects these) and Rust is immune (the `regex`
 * crate's 10 MB size limit). bumi was the only one without a guard.
 */

import { describe, expect, it } from "bun:test";
import { MAX_REGEX_PATTERN_LENGTH, PatternTooLongError } from "../src/limits.ts";
import { MatcherError } from "../src/matcher.ts";
import { MAX_REPEAT_PRODUCT, maxRepeatProduct } from "../src/regex-budget.ts";
import { RegexMatcher } from "../src/string-matchers.ts";

describe("maxRepeatProduct", () => {
	it("returns 1 for patterns with no counted repetition", () => {
		expect(maxRepeatProduct("^/api/.*$")).toBe(1);
		expect(maxRepeatProduct("a+b*c?")).toBe(1);
	});

	it("reads a single repeat", () => {
		expect(maxRepeatProduct("a{100}")).toBe(100);
		expect(maxRepeatProduct("a{2,50}")).toBe(50);
		expect(maxRepeatProduct("a{7,}")).toBe(7);
	});

	it("multiplies through nesting, which is the whole point", () => {
		expect(maxRepeatProduct("(a{100}){100}")).toBe(10_000);
		expect(maxRepeatProduct("((a{10}){10}){10}")).toBe(1_000);
	});

	it("takes the largest path, not the last one seen", () => {
		expect(maxRepeatProduct("(a{2}){2}(b{50}){50}")).toBe(2_500);
	});

	it("does not treat escaped braces or character classes as quantifiers", () => {
		expect(maxRepeatProduct("\\{100\\}")).toBe(1);
		expect(maxRepeatProduct("[{}]")).toBe(1);
		expect(maxRepeatProduct("[a-z]{50}")).toBe(50);
	});

	it("does not hang or throw on malformed input", () => {
		for (const p of ["(((", ")))", "a{", "a{}", "a{,}", "[", "\\"]) {
			expect(() => maxRepeatProduct(p)).not.toThrow();
		}
	});
});

describe("RegexMatcher compile budget", () => {
	it("rejects the 13-character bomb", () => {
		expect(() => new RegexMatcher("(a{100}){100}")).toThrow(MatcherError);
	});

	it("rejects the 20-character bomb that cost 282ms and 286MB", () => {
		const started = Date.now();
		expect(() => new RegexMatcher("((a{100}){100}){100}")).toThrow(MatcherError);
		// The point of the guard is that rejection is cheap. If this ever takes
		// hundreds of milliseconds, the pattern is being compiled before it is
		// checked and the guard has been reordered into uselessness.
		expect(Date.now() - started).toBeLessThan(100);
	});

	it("rejects on length as well as on product", () => {
		const long = "a".repeat(MAX_REGEX_PATTERN_LENGTH + 1);
		expect(() => new RegexMatcher(long)).toThrow(PatternTooLongError);
	});

	// The other half. A guard that rejects everything would pass every test
	// above and break every real config.
	it("accepts ordinary patterns", () => {
		for (const p of ["^/api/.*$", "user-\\d+", "[a-z]{1,8}", "(foo|bar)+"]) {
			expect(() => new RegexMatcher(p)).not.toThrow();
		}
	});

	it("accepts repetition right up to the budget", () => {
		expect(maxRepeatProduct("((a{10}){10}){10}")).toBe(MAX_REPEAT_PRODUCT);
		expect(() => new RegexMatcher("((a{10}){10}){10}")).not.toThrow();
	});

	it("still matches correctly after the guard runs", () => {
		const m = new RegexMatcher("^user-\\d+$");
		expect(m.matches("user-123")).toBe(true);
		expect(m.matches("user-abc")).toBe(false);
	});
});
