/**
 * Tests for bumi registry (bumi/src/registry.ts).
 *
 * Validates the builder -> frozen registry -> loadMatcher pipeline.
 */

import { describe, expect, test } from "bun:test";
import { parseMatcherConfig } from "../src/config.ts";
import {
	InvalidConfigError,
	MAX_FIELD_MATCHERS,
	MAX_PATTERN_LENGTH,
	MAX_PREDICATES_PER_COMPOUND,
	MAX_REGEX_PATTERN_LENGTH,
	PatternTooLongError,
	RegistryBuilder,
	TooManyFieldMatchersError,
	TooManyPredicatesError,
	UnknownTypeUrlError,
} from "../src/registry.ts";
import { DictInput, register } from "../src/testing.ts";

describe("RegistryBuilder", () => {
	test("registers and freezes", () => {
		const builder = new RegistryBuilder<Record<string, string>>();
		builder.input("test.DictInput", (cfg) => new DictInput(cfg.key as string));
		const registry = builder.build();

		expect(registry.inputCount).toBe(1);
		expect(registry.containsInput("test.DictInput")).toBe(true);
		expect(registry.containsInput("test.Unknown")).toBe(false);
	});

	test("register helper", () => {
		const builder = new RegistryBuilder<Record<string, string>>();
		register(builder);
		const registry = builder.build();

		expect(registry.containsInput("xuma.test.v1.StringInput")).toBe(true);
	});

	test("introspection type URLs", () => {
		const builder = new RegistryBuilder<Record<string, string>>();
		builder.input("b.Input", (cfg) => new DictInput(cfg.key as string));
		builder.input("a.Input", (cfg) => new DictInput(cfg.key as string));
		const registry = builder.build();

		// Sorted alphabetically
		expect(registry.inputTypeUrls()).toEqual(["a.Input", "b.Input"]);
	});
});

describe("loadMatcher", () => {
	function makeRegistry() {
		const builder = new RegistryBuilder<Record<string, string>>();
		register(builder);
		return builder.build();
	}

	test("simple exact match", () => {
		const registry = makeRegistry();
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "single",
						input: {
							type_url: "xuma.test.v1.StringInput",
							config: { key: "name" },
						},
						value_match: { Exact: "alice" },
					},
					on_match: { type: "action", action: "matched" },
				},
			],
			on_no_match: { type: "action", action: "default" },
		});
		const matcher = registry.loadMatcher(config);

		expect(matcher.evaluate({ name: "alice" })).toBe("matched");
		expect(matcher.evaluate({ name: "bob" })).toBe("default");
	});

	test("and predicate", () => {
		const registry = makeRegistry();
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "and",
						predicates: [
							{
								type: "single",
								input: {
									type_url: "xuma.test.v1.StringInput",
									config: { key: "role" },
								},
								value_match: { Exact: "admin" },
							},
							{
								type: "single",
								input: {
									type_url: "xuma.test.v1.StringInput",
									config: { key: "org" },
								},
								value_match: { Prefix: "acme" },
							},
						],
					},
					on_match: { type: "action", action: "admin_acme" },
				},
			],
		});
		const matcher = registry.loadMatcher(config);

		expect(matcher.evaluate({ role: "admin", org: "acme-corp" })).toBe("admin_acme");
		expect(matcher.evaluate({ role: "admin", org: "other" })).toBeNull();
	});

	test("nested matcher", () => {
		const registry = makeRegistry();
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "single",
						input: {
							type_url: "xuma.test.v1.StringInput",
							config: { key: "tier" },
						},
						value_match: { Prefix: "" },
					},
					on_match: {
						type: "matcher",
						matcher: {
							matchers: [
								{
									predicate: {
										type: "single",
										input: {
											type_url: "xuma.test.v1.StringInput",
											config: { key: "tier" },
										},
										value_match: { Exact: "premium" },
									},
									on_match: {
										type: "action",
										action: "premium_route",
									},
								},
							],
						},
					},
				},
			],
			on_no_match: { type: "action", action: "fallback" },
		});
		const matcher = registry.loadMatcher(config);

		expect(matcher.evaluate({ tier: "premium" })).toBe("premium_route");
		expect(matcher.evaluate({ tier: "basic" })).toBe("fallback");
	});

	test("all string match types", () => {
		const registry = makeRegistry();
		const cases: [string, string, Record<string, string>, boolean][] = [
			["Exact", "hello", { key: "hello" }, true],
			["Prefix", "hel", { key: "hello" }, true],
			["Suffix", "llo", { key: "hello" }, true],
			["Contains", "ell", { key: "hello" }, true],
			["Regex", "^h.*o$", { key: "hello" }, true],
			["Exact", "hello", { key: "world" }, false],
		];

		for (const [variant, pattern, ctx, shouldMatch] of cases) {
			const config = parseMatcherConfig({
				matchers: [
					{
						predicate: {
							type: "single",
							input: {
								type_url: "xuma.test.v1.StringInput",
								config: { key: "key" },
							},
							value_match: { [variant]: pattern },
						},
						on_match: { type: "action", action: "hit" },
					},
				],
			});
			const matcher = registry.loadMatcher(config);
			const result = matcher.evaluate(ctx);
			const expected = shouldMatch ? "hit" : null;
			expect(result).toBe(expected);
		}
	});
});

describe("registry errors", () => {
	test("unknown input type_url", () => {
		const registry = new RegistryBuilder<Record<string, string>>().build();
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "single",
						input: { type_url: "unknown.Input", config: {} },
						value_match: { Exact: "x" },
					},
					on_match: { type: "action", action: "x" },
				},
			],
		});

		expect(() => registry.loadMatcher(config)).toThrow(UnknownTypeUrlError);
		try {
			registry.loadMatcher(config);
		} catch (e) {
			if (e instanceof UnknownTypeUrlError) {
				expect(e.typeUrl).toBe("unknown.Input");
				expect(e.registry).toBe("input");
			}
		}
	});

	test("unknown input lists available", () => {
		const builder = new RegistryBuilder<Record<string, string>>();
		register(builder);
		const registry = builder.build();

		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "single",
						input: { type_url: "unknown.Input", config: {} },
						value_match: { Exact: "x" },
					},
					on_match: { type: "action", action: "x" },
				},
			],
		});

		try {
			registry.loadMatcher(config);
			expect.unreachable("should have thrown");
		} catch (e) {
			expect(e).toBeInstanceOf(UnknownTypeUrlError);
			if (e instanceof UnknownTypeUrlError) {
				expect(e.available).toContain("xuma.test.v1.StringInput");
				expect(e.message).toContain("xuma.test.v1.StringInput");
			}
		}
	});

	test("unknown matcher type_url", () => {
		const builder = new RegistryBuilder<Record<string, string>>();
		register(builder);
		const registry = builder.build();

		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "single",
						input: {
							type_url: "xuma.test.v1.StringInput",
							config: { key: "x" },
						},
						custom_match: { type_url: "unknown.Matcher", config: {} },
					},
					on_match: { type: "action", action: "x" },
				},
			],
		});

		expect(() => registry.loadMatcher(config)).toThrow(UnknownTypeUrlError);
		try {
			registry.loadMatcher(config);
		} catch (e) {
			if (e instanceof UnknownTypeUrlError) {
				expect(e.typeUrl).toBe("unknown.Matcher");
				expect(e.registry).toBe("matcher");
			}
		}
	});

	test("invalid config", () => {
		const builder = new RegistryBuilder<Record<string, string>>();
		register(builder);
		const registry = builder.build();

		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "single",
						input: {
							type_url: "xuma.test.v1.StringInput",
							config: { wrong_field: 42 },
						},
						value_match: { Exact: "x" },
					},
					on_match: { type: "action", action: "x" },
				},
			],
		});

		expect(() => registry.loadMatcher(config)).toThrow(InvalidConfigError);
	});
});

describe("width limits", () => {
	function makeRegistry() {
		const builder = new RegistryBuilder<Record<string, string>>();
		register(builder);
		return builder.build();
	}

	test("too many field matchers", () => {
		const registry = makeRegistry();
		const fm = {
			predicate: {
				type: "single",
				input: {
					type_url: "xuma.test.v1.StringInput",
					config: { key: "x" },
				},
				value_match: { Exact: "x" },
			},
			on_match: { type: "action", action: "x" },
		};
		const config = parseMatcherConfig({
			matchers: Array(MAX_FIELD_MATCHERS + 1).fill(fm),
		});

		expect(() => registry.loadMatcher(config)).toThrow(TooManyFieldMatchersError);
		try {
			registry.loadMatcher(config);
		} catch (e) {
			if (e instanceof TooManyFieldMatchersError) {
				expect(e.count).toBe(MAX_FIELD_MATCHERS + 1);
				expect(e.max).toBe(MAX_FIELD_MATCHERS);
			}
		}
	});

	test("too many predicates and", () => {
		const registry = makeRegistry();
		const single = {
			type: "single",
			input: {
				type_url: "xuma.test.v1.StringInput",
				config: { key: "x" },
			},
			value_match: { Exact: "x" },
		};
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "and",
						predicates: Array(MAX_PREDICATES_PER_COMPOUND + 1).fill(single),
					},
					on_match: { type: "action", action: "x" },
				},
			],
		});

		expect(() => registry.loadMatcher(config)).toThrow(TooManyPredicatesError);
	});

	test("too many predicates or", () => {
		const registry = makeRegistry();
		const single = {
			type: "single",
			input: {
				type_url: "xuma.test.v1.StringInput",
				config: { key: "x" },
			},
			value_match: { Exact: "x" },
		};
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "or",
						predicates: Array(MAX_PREDICATES_PER_COMPOUND + 1).fill(single),
					},
					on_match: { type: "action", action: "x" },
				},
			],
		});

		expect(() => registry.loadMatcher(config)).toThrow(TooManyPredicatesError);
	});

	test("pattern too long exact", () => {
		const registry = makeRegistry();
		const longPattern = "x".repeat(MAX_PATTERN_LENGTH + 1);
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "single",
						input: {
							type_url: "xuma.test.v1.StringInput",
							config: { key: "x" },
						},
						value_match: { Exact: longPattern },
					},
					on_match: { type: "action", action: "x" },
				},
			],
		});

		expect(() => registry.loadMatcher(config)).toThrow(PatternTooLongError);
		try {
			registry.loadMatcher(config);
		} catch (e) {
			if (e instanceof PatternTooLongError) {
				expect(e.length).toBe(MAX_PATTERN_LENGTH + 1);
				expect(e.max).toBe(MAX_PATTERN_LENGTH);
			}
		}
	});

	test("regex pattern too long", () => {
		const registry = makeRegistry();
		const longRegex = "a".repeat(MAX_REGEX_PATTERN_LENGTH + 1);
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "single",
						input: {
							type_url: "xuma.test.v1.StringInput",
							config: { key: "x" },
						},
						value_match: { Regex: longRegex },
					},
					on_match: { type: "action", action: "x" },
				},
			],
		});

		expect(() => registry.loadMatcher(config)).toThrow(PatternTooLongError);
	});

	test("pattern at limit succeeds", () => {
		const registry = makeRegistry();
		const pattern = "x".repeat(MAX_PATTERN_LENGTH);
		const config = parseMatcherConfig({
			matchers: [
				{
					predicate: {
						type: "single",
						input: {
							type_url: "xuma.test.v1.StringInput",
							config: { key: "x" },
						},
						value_match: { Exact: pattern },
					},
					on_match: { type: "action", action: "x" },
				},
			],
		});

		// Should not throw
		registry.loadMatcher(config);
	});

	test("field matchers at limit succeeds", () => {
		const registry = makeRegistry();
		const fm = {
			predicate: {
				type: "single",
				input: {
					type_url: "xuma.test.v1.StringInput",
					config: { key: "x" },
				},
				value_match: { Exact: "x" },
			},
			on_match: { type: "action", action: "x" },
		};
		const config = parseMatcherConfig({
			matchers: Array(MAX_FIELD_MATCHERS).fill(fm),
		});

		// Should not throw
		registry.loadMatcher(config);
	});
});
