/**
 * Tests for bumi registry (bumi/src/registry.ts).
 *
 * Validates the builder -> frozen registry -> loadMatcher pipeline.
 *
 * These used to build configs through `parseMatcherConfig`, the terse
 * dialect's reader. That dialect is retired (DECISIONS.md D-026); the IR
 * types it produced (`MatcherConfig` and friends) are still exactly what the
 * registry consumes, so the tests construct them directly instead. The
 * `single`/`field` helpers below are the only new things -- small builders
 * that keep test bodies close to their previous shape, not a second config
 * format.
 */

import { describe, expect, test } from "bun:test";
import {
	ActionConfig,
	AndPredicateConfig,
	BuiltInMatch,
	CustomMatch,
	FieldMatcherConfig,
	MatcherConfig,
	MatcherOnMatchConfig,
	OrPredicateConfig,
	SinglePredicateConfig,
	TypedConfig,
} from "../src/config.ts";
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

/** A `xuma.kv.v1.MapInput` reference, config-shaped as the registry expects. */
function mapInput(key: string): TypedConfig {
	return new TypedConfig("xuma.kv.v1.MapInput", { key });
}

/** A single predicate: read `key`, compare with `variant` (Exact/Prefix/...). */
function single(key: string, variant: string, value: string): SinglePredicateConfig {
	return new SinglePredicateConfig(mapInput(key), new BuiltInMatch(variant, value));
}

/** A field matcher with a plain action on_match. */
function field(
	predicate: SinglePredicateConfig | AndPredicateConfig | OrPredicateConfig,
	action: string,
): FieldMatcherConfig<string> {
	return new FieldMatcherConfig(predicate, new ActionConfig(action));
}

describe("RegistryBuilder", () => {
	test("registers and freezes", () => {
		const builder = new RegistryBuilder<Record<string, string>>();
		builder.input("test.DictInput", (cfg) => new DictInput(cfg.key as string));
		const registry = builder.build();

		expect(registry.inputCount).toBe(1);
		expect(registry.containsInput("test.DictInput")).toBe(true);
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
		const config = new MatcherConfig(
			[field(single("name", "Exact", "alice"), "matched")],
			new ActionConfig("default"),
		);
		const matcher = registry.loadMatcher(config);

		expect(matcher.evaluate({ name: "alice" })).toBe("matched");
		expect(matcher.evaluate({ name: "bob" })).toBe("default");
	});

	test("and predicate", () => {
		const registry = makeRegistry();
		const config = new MatcherConfig([
			field(
				new AndPredicateConfig([single("role", "Exact", "admin"), single("org", "Prefix", "acme")]),
				"admin_acme",
			),
		]);
		const matcher = registry.loadMatcher(config);

		expect(matcher.evaluate({ role: "admin", org: "acme-corp" })).toBe("admin_acme");
		expect(matcher.evaluate({ role: "admin", org: "other" })).toBeNull();
	});

	test("nested matcher", () => {
		const registry = makeRegistry();
		const inner = new MatcherConfig([field(single("tier", "Exact", "premium"), "premium_route")]);
		const config = new MatcherConfig(
			[new FieldMatcherConfig(single("tier", "Prefix", ""), new MatcherOnMatchConfig(inner))],
			new ActionConfig("fallback"),
		);
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
			const config = new MatcherConfig([field(single("key", variant, pattern), "hit")]);
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
		const config = new MatcherConfig([
			field(
				new SinglePredicateConfig(new TypedConfig("unknown.Input"), new BuiltInMatch("Exact", "x")),
				"x",
			),
		]);

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

		const config = new MatcherConfig([
			field(
				new SinglePredicateConfig(new TypedConfig("unknown.Input"), new BuiltInMatch("Exact", "x")),
				"x",
			),
		]);

		try {
			registry.loadMatcher(config);
			expect.unreachable("should have thrown");
		} catch (e) {
			expect(e).toBeInstanceOf(UnknownTypeUrlError);
			if (e instanceof UnknownTypeUrlError) {
				expect(e.available).toContain("xuma.kv.v1.MapInput");
				expect(e.message).toContain("xuma.kv.v1.MapInput");
			}
		}
	});

	test("unknown matcher type_url", () => {
		const builder = new RegistryBuilder<Record<string, string>>();
		register(builder);
		const registry = builder.build();

		const config = new MatcherConfig([
			field(
				new SinglePredicateConfig(
					mapInput("x"),
					new CustomMatch(new TypedConfig("unknown.Matcher")),
				),
				"x",
			),
		]);

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

		const config = new MatcherConfig([
			field(
				new SinglePredicateConfig(
					new TypedConfig("xuma.kv.v1.MapInput", { wrong_field: 42 }),
					new BuiltInMatch("Exact", "x"),
				),
				"x",
			),
		]);

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
		const fm = field(single("x", "Exact", "x"), "x");
		const config = new MatcherConfig(
			Array(MAX_FIELD_MATCHERS + 1).fill(fm) as FieldMatcherConfig<string>[],
		);

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
		const one = single("x", "Exact", "x");
		const config = new MatcherConfig([
			field(new AndPredicateConfig(Array(MAX_PREDICATES_PER_COMPOUND + 1).fill(one)), "x"),
		]);

		expect(() => registry.loadMatcher(config)).toThrow(TooManyPredicatesError);
	});

	test("too many predicates or", () => {
		const registry = makeRegistry();
		const one = single("x", "Exact", "x");
		const config = new MatcherConfig([
			field(new OrPredicateConfig(Array(MAX_PREDICATES_PER_COMPOUND + 1).fill(one)), "x"),
		]);

		expect(() => registry.loadMatcher(config)).toThrow(TooManyPredicatesError);
	});

	test("pattern too long exact", () => {
		const registry = makeRegistry();
		const longPattern = "x".repeat(MAX_PATTERN_LENGTH + 1);
		const config = new MatcherConfig([field(single("x", "Exact", longPattern), "x")]);

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
		const config = new MatcherConfig([field(single("x", "Regex", longRegex), "x")]);

		expect(() => registry.loadMatcher(config)).toThrow(PatternTooLongError);
	});

	test("pattern at limit succeeds", () => {
		const registry = makeRegistry();
		const pattern = "x".repeat(MAX_PATTERN_LENGTH);
		const config = new MatcherConfig([field(single("x", "Exact", pattern), "x")]);

		// Should not throw
		registry.loadMatcher(config);
	});

	test("field matchers at limit succeeds", () => {
		const registry = makeRegistry();
		const fm = field(single("x", "Exact", "x"), "x");
		const config = new MatcherConfig(
			Array(MAX_FIELD_MATCHERS).fill(fm) as FieldMatcherConfig<string>[],
		);

		// Should not throw
		registry.loadMatcher(config);
	});
});
