import { describe, expect, it } from "bun:test";
import { And, Not, Or, SinglePredicate, predicateDepth } from "../src/predicate.ts";
import { ExactMatcher } from "../src/string-matchers.ts";
import { DictInput } from "../src/testing.ts";

describe("SinglePredicate", () => {
	it("matches when input matches", () => {
		const p = new SinglePredicate(new DictInput("name"), new ExactMatcher("alice"));
		expect(p.evaluate({ name: "alice" })).toBe(true);
	});

	it("rejects when input does not match", () => {
		const p = new SinglePredicate(new DictInput("name"), new ExactMatcher("alice"));
		expect(p.evaluate({ name: "bob" })).toBe(false);
	});

	it("returns false when input is null (missing key)", () => {
		const p = new SinglePredicate(new DictInput("missing"), new ExactMatcher("alice"));
		expect(p.evaluate({ name: "alice" })).toBe(false);
	});
});

describe("And", () => {
	it("true when all predicates match", () => {
		const p = new And([
			new SinglePredicate(new DictInput("a"), new ExactMatcher("1")),
			new SinglePredicate(new DictInput("b"), new ExactMatcher("2")),
		]);
		expect(p.evaluate({ a: "1", b: "2" })).toBe(true);
	});

	it("false when one predicate fails", () => {
		const p = new And([
			new SinglePredicate(new DictInput("a"), new ExactMatcher("1")),
			new SinglePredicate(new DictInput("b"), new ExactMatcher("wrong")),
		]);
		expect(p.evaluate({ a: "1", b: "2" })).toBe(false);
	});

	it("empty AND is vacuously true", () => {
		const p = new And<Record<string, string>>([]);
		expect(p.evaluate({})).toBe(true);
	});
});

describe("Or", () => {
	it("true when one predicate matches", () => {
		const p = new Or([
			new SinglePredicate(new DictInput("a"), new ExactMatcher("wrong")),
			new SinglePredicate(new DictInput("a"), new ExactMatcher("1")),
		]);
		expect(p.evaluate({ a: "1" })).toBe(true);
	});

	it("false when all predicates fail", () => {
		const p = new Or([
			new SinglePredicate(new DictInput("a"), new ExactMatcher("x")),
			new SinglePredicate(new DictInput("a"), new ExactMatcher("y")),
		]);
		expect(p.evaluate({ a: "1" })).toBe(false);
	});

	it("empty OR is vacuously false", () => {
		const p = new Or<Record<string, string>>([]);
		expect(p.evaluate({})).toBe(false);
	});
});

describe("Not", () => {
	it("inverts true to false", () => {
		const p = new Not(new SinglePredicate(new DictInput("a"), new ExactMatcher("1")));
		expect(p.evaluate({ a: "1" })).toBe(false);
	});

	it("inverts false to true", () => {
		const p = new Not(new SinglePredicate(new DictInput("a"), new ExactMatcher("wrong")));
		expect(p.evaluate({ a: "1" })).toBe(true);
	});
});

describe("predicateDepth", () => {
	it("single = 1", () => {
		const p = new SinglePredicate(new DictInput("a"), new ExactMatcher("1"));
		expect(predicateDepth(p)).toBe(1);
	});

	it("And wrapping single = 2", () => {
		const p = new And([new SinglePredicate(new DictInput("a"), new ExactMatcher("1"))]);
		expect(predicateDepth(p)).toBe(2);
	});

	it("Not(And(Single)) = 3", () => {
		const p = new Not(new And([new SinglePredicate(new DictInput("a"), new ExactMatcher("1"))]));
		expect(predicateDepth(p)).toBe(3);
	});
});
