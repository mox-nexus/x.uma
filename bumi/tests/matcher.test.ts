import { describe, expect, it } from "bun:test";
import {
	Action,
	FieldMatcher,
	MAX_DEPTH,
	Matcher,
	MatcherError,
	NestedMatcher,
} from "../src/matcher.ts";
import { SinglePredicate } from "../src/predicate.ts";
import { ExactMatcher } from "../src/string-matchers.ts";
import { DictInput } from "../src/testing.ts";

describe("Matcher", () => {
	it("first match wins", () => {
		const m = new Matcher([
			new FieldMatcher(
				new SinglePredicate(new DictInput("x"), new ExactMatcher("a")),
				new Action("first"),
			),
			new FieldMatcher(
				new SinglePredicate(new DictInput("x"), new ExactMatcher("a")),
				new Action("second"),
			),
		]);
		expect(m.evaluate({ x: "a" })).toBe("first");
	});

	it("no match returns null", () => {
		const m = new Matcher([
			new FieldMatcher(
				new SinglePredicate(new DictInput("x"), new ExactMatcher("a")),
				new Action("hit"),
			),
		]);
		expect(m.evaluate({ x: "b" })).toBeNull();
	});

	it("on_no_match fallback", () => {
		const m = new Matcher(
			[
				new FieldMatcher(
					new SinglePredicate(new DictInput("x"), new ExactMatcher("a")),
					new Action("hit"),
				),
			],
			new Action("default"),
		);
		expect(m.evaluate({ x: "b" })).toBe("default");
	});

	it("empty matcher returns null", () => {
		const m = new Matcher<Record<string, string>, string>([]);
		expect(m.evaluate({ x: "a" })).toBeNull();
	});

	it("empty matcher with fallback", () => {
		const m = new Matcher<Record<string, string>, string>([], new Action("default"));
		expect(m.evaluate({})).toBe("default");
	});
});

describe("NestedMatcher", () => {
	it("nested match succeeds", () => {
		const inner = new Matcher([
			new FieldMatcher(
				new SinglePredicate(new DictInput("y"), new ExactMatcher("b")),
				new Action("nested_hit"),
			),
		]);
		const outer = new Matcher([
			new FieldMatcher(
				new SinglePredicate(new DictInput("x"), new ExactMatcher("a")),
				new NestedMatcher(inner),
			),
		]);
		expect(outer.evaluate({ x: "a", y: "b" })).toBe("nested_hit");
	});

	it("nested failure propagates to next rule", () => {
		const inner = new Matcher([
			new FieldMatcher(
				new SinglePredicate(new DictInput("y"), new ExactMatcher("b")),
				new Action("nested_hit"),
			),
		]);
		const outer = new Matcher([
			new FieldMatcher(
				new SinglePredicate(new DictInput("x"), new ExactMatcher("a")),
				new NestedMatcher(inner),
			),
			new FieldMatcher(
				new SinglePredicate(new DictInput("x"), new ExactMatcher("a")),
				new Action("fallthrough"),
			),
		]);
		// x=a matches, but nested fails (y != b), continues to second rule
		expect(outer.evaluate({ x: "a", y: "nope" })).toBe("fallthrough");
	});
});

describe("depth validation", () => {
	it("shallow passes", () => {
		// Should not throw
		new Matcher([
			new FieldMatcher(
				new SinglePredicate(new DictInput("x"), new ExactMatcher("a")),
				new Action("hit"),
			),
		]);
	});

	it("at MAX_DEPTH passes", () => {
		let current: Matcher<Record<string, string>, string> = new Matcher([
			new FieldMatcher(
				new SinglePredicate(new DictInput("x"), new ExactMatcher("a")),
				new Action("deep"),
			),
		]);
		while (current.depth() < MAX_DEPTH) {
			current = new Matcher([
				new FieldMatcher(
					new SinglePredicate(new DictInput("x"), new ExactMatcher("a")),
					new NestedMatcher(current),
				),
			]);
		}
		expect(current.depth()).toBe(MAX_DEPTH);
	});

	it("exceeds MAX_DEPTH throws at construction", () => {
		let current: Matcher<Record<string, string>, string> = new Matcher([
			new FieldMatcher(
				new SinglePredicate(new DictInput("x"), new ExactMatcher("a")),
				new Action("deep"),
			),
		]);
		while (current.depth() < MAX_DEPTH) {
			current = new Matcher([
				new FieldMatcher(
					new SinglePredicate(new DictInput("x"), new ExactMatcher("a")),
					new NestedMatcher(current),
				),
			]);
		}
		expect(current.depth()).toBe(MAX_DEPTH);
		// One more level throws
		expect(
			() =>
				new Matcher([
					new FieldMatcher(
						new SinglePredicate(new DictInput("x"), new ExactMatcher("a")),
						new NestedMatcher(current),
					),
				]),
		).toThrow(MatcherError);
	});
});
