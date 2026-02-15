/**
 * Tests for bumi config parsing (bumi/src/config.ts).
 *
 * Validates parseMatcherConfig() edge cases and structural correctness.
 */

import { describe, expect, test } from "bun:test";
import {
	ActionConfig,
	AndPredicateConfig,
	BuiltInMatch,
	ConfigParseError,
	CustomMatch,
	FieldMatcherConfig,
	MatcherConfig,
	MatcherOnMatchConfig,
	NotPredicateConfig,
	OrPredicateConfig,
	SinglePredicateConfig,
	TypedConfig,
	parseMatcherConfig,
} from "../src/config.ts";

describe("parseMatcherConfig", () => {
	test("simple exact match", () => {
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "single",
						input: { type_url: "test.Input", config: { key: "x" } },
						value_match: { Exact: "hello" },
					},
					on_match: { type: "action", action: "hit" },
				},
			],
		});

		expect(config).toBeInstanceOf(MatcherConfig);
		expect(config.matchers).toHaveLength(1);
		expect(config.matchers[0]).toBeInstanceOf(FieldMatcherConfig);

		const pred = config.matchers[0]!.predicate;
		expect(pred).toBeInstanceOf(SinglePredicateConfig);
		if (pred instanceof SinglePredicateConfig) {
			expect(pred.input).toBeInstanceOf(TypedConfig);
			expect(pred.input.typeUrl).toBe("test.Input");
			expect(pred.matcher).toBeInstanceOf(BuiltInMatch);
			if (pred.matcher instanceof BuiltInMatch) {
				expect(pred.matcher.variant).toBe("Exact");
				expect(pred.matcher.value).toBe("hello");
			}
		}
	});

	test("and predicate", () => {
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "and",
						predicates: [
							{
								type: "single",
								input: { type_url: "a", config: {} },
								value_match: { Exact: "x" },
							},
							{
								type: "single",
								input: { type_url: "b", config: {} },
								value_match: { Prefix: "y" },
							},
						],
					},
					on_match: { type: "action", action: "ok" },
				},
			],
		});

		const pred = config.matchers[0]!.predicate;
		expect(pred).toBeInstanceOf(AndPredicateConfig);
		if (pred instanceof AndPredicateConfig) {
			expect(pred.predicates).toHaveLength(2);
		}
	});

	test("or predicate", () => {
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "or",
						predicates: [
							{
								type: "single",
								input: { type_url: "a", config: {} },
								value_match: { Exact: "x" },
							},
						],
					},
					on_match: { type: "action", action: "ok" },
				},
			],
		});

		const pred = config.matchers[0]!.predicate;
		expect(pred).toBeInstanceOf(OrPredicateConfig);
	});

	test("not predicate", () => {
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "not",
						predicate: {
							type: "single",
							input: { type_url: "a", config: {} },
							value_match: { Exact: "x" },
						},
					},
					on_match: { type: "action", action: "ok" },
				},
			],
		});

		const pred = config.matchers[0]!.predicate;
		expect(pred).toBeInstanceOf(NotPredicateConfig);
	});

	test("nested matcher on_match", () => {
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "single",
						input: { type_url: "a", config: {} },
						value_match: { Prefix: "" },
					},
					on_match: {
						type: "matcher",
						matcher: {
							matchers: [
								{
									predicate: {
										type: "single",
										input: { type_url: "b", config: {} },
										value_match: { Exact: "x" },
									},
									on_match: { type: "action", action: "inner" },
								},
							],
						},
					},
				},
			],
		});

		const onMatch = config.matchers[0]!.onMatch;
		expect(onMatch).toBeInstanceOf(MatcherOnMatchConfig);
	});

	test("custom_match", () => {
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "single",
						input: { type_url: "a", config: {} },
						custom_match: {
							type_url: "custom.Matcher",
							config: { threshold: 0.5 },
						},
					},
					on_match: { type: "action", action: "ok" },
				},
			],
		});

		const pred = config.matchers[0]!.predicate;
		expect(pred).toBeInstanceOf(SinglePredicateConfig);
		if (pred instanceof SinglePredicateConfig) {
			expect(pred.matcher).toBeInstanceOf(CustomMatch);
			if (pred.matcher instanceof CustomMatch) {
				expect(pred.matcher.typedConfig.typeUrl).toBe("custom.Matcher");
			}
		}
	});

	test("on_no_match", () => {
		const config = parseMatcherConfig({
			matchers: [],
			on_no_match: { type: "action", action: "fallback" },
		});

		expect(config.onNoMatch).toBeInstanceOf(ActionConfig);
		if (config.onNoMatch instanceof ActionConfig) {
			expect(config.onNoMatch.action).toBe("fallback");
		}
	});

	test("all string match variants", () => {
		for (const variant of ["Exact", "Prefix", "Suffix", "Contains", "Regex"]) {
			const config = parseMatcherConfig({
				matchers: [
					{
						predicate: {
							type: "single",
							input: { type_url: "a", config: {} },
							value_match: { [variant]: "test" },
						},
						on_match: { type: "action", action: "ok" },
					},
				],
			});

			const pred = config.matchers[0]!.predicate;
			expect(pred).toBeInstanceOf(SinglePredicateConfig);
			if (pred instanceof SinglePredicateConfig) {
				expect(pred.matcher).toBeInstanceOf(BuiltInMatch);
				if (pred.matcher instanceof BuiltInMatch) {
					expect(pred.matcher.variant).toBe(variant);
				}
			}
		}
	});

	test("default config is empty object", () => {
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "single",
						input: { type_url: "a" },
						value_match: { Exact: "x" },
					},
					on_match: { type: "action", action: "ok" },
				},
			],
		});

		const pred = config.matchers[0]!.predicate;
		if (pred instanceof SinglePredicateConfig) {
			expect(pred.input.config).toEqual({});
		}
	});
});

describe("parse errors", () => {
	test("non-object input", () => {
		expect(() => parseMatcherConfig("not an object")).toThrow(ConfigParseError);
	});

	test("null input", () => {
		expect(() => parseMatcherConfig(null)).toThrow(ConfigParseError);
	});

	test("array input", () => {
		expect(() => parseMatcherConfig([])).toThrow(ConfigParseError);
	});

	test("missing matchers", () => {
		expect(() => parseMatcherConfig({})).toThrow(ConfigParseError);
	});

	test("matchers not array", () => {
		expect(() => parseMatcherConfig({ matchers: "bad" })).toThrow(ConfigParseError);
	});

	test("missing predicate", () => {
		expect(() =>
			parseMatcherConfig({
				matchers: [{ on_match: { type: "action", action: "x" } }],
			}),
		).toThrow(ConfigParseError);
	});

	test("missing on_match", () => {
		expect(() =>
			parseMatcherConfig({
				matchers: [
					{
						predicate: {
							type: "single",
							input: { type_url: "a", config: {} },
							value_match: { Exact: "x" },
						},
					},
				],
			}),
		).toThrow(ConfigParseError);
	});

	test("both value_match and custom_match", () => {
		expect(() =>
			parseMatcherConfig({
				matchers: [
					{
						predicate: {
							type: "single",
							input: { type_url: "a", config: {} },
							value_match: { Exact: "x" },
							custom_match: { type_url: "b", config: {} },
						},
						on_match: { type: "action", action: "x" },
					},
				],
			}),
		).toThrow(ConfigParseError);
	});

	test("neither value_match nor custom_match", () => {
		expect(() =>
			parseMatcherConfig({
				matchers: [
					{
						predicate: {
							type: "single",
							input: { type_url: "a", config: {} },
						},
						on_match: { type: "action", action: "x" },
					},
				],
			}),
		).toThrow(ConfigParseError);
	});

	test("unknown predicate type", () => {
		expect(() =>
			parseMatcherConfig({
				matchers: [
					{
						predicate: { type: "xor", predicates: [] },
						on_match: { type: "action", action: "x" },
					},
				],
			}),
		).toThrow(ConfigParseError);
	});

	test("unknown on_match type", () => {
		expect(() =>
			parseMatcherConfig({
				matchers: [
					{
						predicate: {
							type: "single",
							input: { type_url: "a", config: {} },
							value_match: { Exact: "x" },
						},
						on_match: { type: "invalid" },
					},
				],
			}),
		).toThrow(ConfigParseError);
	});
});
