import { describe, expect, it } from "bun:test";
import { MatcherError } from "../src/matcher.ts";
import {
	ContainsMatcher,
	ExactMatcher,
	PrefixMatcher,
	RegexMatcher,
	SuffixMatcher,
} from "../src/string-matchers.ts";

describe("ExactMatcher", () => {
	it("matches exact string", () => {
		expect(new ExactMatcher("hello").matches("hello")).toBe(true);
	});

	it("rejects non-match", () => {
		expect(new ExactMatcher("hello").matches("world")).toBe(false);
	});

	it("is case-sensitive by default", () => {
		expect(new ExactMatcher("hello").matches("Hello")).toBe(false);
	});

	it("supports ignore_case", () => {
		const m = new ExactMatcher("hello", true);
		expect(m.matches("HELLO")).toBe(true);
		expect(m.matches("Hello")).toBe(true);
		expect(m.matches("hello")).toBe(true);
	});

	it("returns false for null", () => {
		expect(new ExactMatcher("hello").matches(null)).toBe(false);
	});

	it("returns false for non-string", () => {
		expect(new ExactMatcher("42").matches(42)).toBe(false);
	});

	it("handles empty string", () => {
		const m = new ExactMatcher("");
		expect(m.matches("")).toBe(true);
		expect(m.matches("a")).toBe(false);
	});

	it("rejects partial match", () => {
		expect(new ExactMatcher("hello").matches("hello world")).toBe(false);
	});
});

describe("PrefixMatcher", () => {
	it("matches prefix", () => {
		expect(new PrefixMatcher("/api").matches("/api/users")).toBe(true);
	});

	it("exact is prefix", () => {
		expect(new PrefixMatcher("/api").matches("/api")).toBe(true);
	});

	it("rejects non-match", () => {
		expect(new PrefixMatcher("/api").matches("/other")).toBe(false);
	});

	it("supports ignore_case", () => {
		expect(new PrefixMatcher("/API", true).matches("/api/users")).toBe(true);
	});

	it("returns false for null", () => {
		expect(new PrefixMatcher("/api").matches(null)).toBe(false);
	});

	it("empty prefix matches anything", () => {
		expect(new PrefixMatcher("").matches("anything")).toBe(true);
	});
});

describe("SuffixMatcher", () => {
	it("matches suffix", () => {
		expect(new SuffixMatcher(".json").matches("data.json")).toBe(true);
	});

	it("rejects non-match", () => {
		expect(new SuffixMatcher(".json").matches("data.xml")).toBe(false);
	});

	it("supports ignore_case", () => {
		expect(new SuffixMatcher(".JSON", true).matches("data.json")).toBe(true);
	});

	it("returns false for null", () => {
		expect(new SuffixMatcher(".json").matches(null)).toBe(false);
	});
});

describe("ContainsMatcher", () => {
	it("matches substring", () => {
		expect(new ContainsMatcher("world").matches("hello world")).toBe(true);
	});

	it("rejects non-match", () => {
		expect(new ContainsMatcher("xyz").matches("hello world")).toBe(false);
	});

	it("supports ignore_case", () => {
		expect(new ContainsMatcher("WORLD", true).matches("hello world")).toBe(true);
	});

	it("returns false for null", () => {
		expect(new ContainsMatcher("x").matches(null)).toBe(false);
	});

	it("empty substring matches anything", () => {
		expect(new ContainsMatcher("").matches("anything")).toBe(true);
	});
});

describe("RegexMatcher", () => {
	it("matches regex", () => {
		expect(new RegexMatcher("^\\d+$").matches("12345")).toBe(true);
	});

	it("rejects non-match", () => {
		expect(new RegexMatcher("^\\d+$").matches("abc")).toBe(false);
	});

	it("searches anywhere (not full match)", () => {
		expect(new RegexMatcher("\\d+").matches("abc123def")).toBe(true);
	});

	it("returns false for null", () => {
		expect(new RegexMatcher("\\d+").matches(null)).toBe(false);
	});

	it("throws MatcherError on invalid regex", () => {
		expect(() => new RegexMatcher("[invalid")).toThrow(MatcherError);
	});

	it("includes pattern in error message", () => {
		expect(() => new RegexMatcher("[unclosed")).toThrow(/\[unclosed/);
	});

	it("rejects backreferences (RE2 linear-time guarantee)", () => {
		expect(() => new RegexMatcher("(a)\\1")).toThrow(MatcherError);
	});
});
