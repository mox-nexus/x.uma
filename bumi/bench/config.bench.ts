/**
 * Config-path benchmarks for bumi (Pure TypeScript).
 *
 * Measures the cost of JSON config → Registry → Matcher construction,
 * and compares config-loaded evaluation against compiler-built evaluation.
 *
 * Run: cd bumi && bun run bench/config.bench.ts
 */

import { bench, run, summary } from "mitata";

import {
	Action,
	ExactMatcher,
	FieldMatcher,
	Matcher,
	RegistryBuilder,
	SinglePredicate,
	parseMatcherConfig,
} from "../src/index.ts";
import type { MatchingData } from "../src/index.ts";
import { DictInput, register } from "../src/testing.ts";

// ── Shared JSON configs (identical across all implementations) ────────────────

const SIMPLE_CONFIG = JSON.stringify({
	matchers: [
		{
			predicate: {
				type: "single",
				input: {
					type_url: "xuma.test.v1.StringInput",
					config: { key: "role" },
				},
				value_match: { Exact: "admin" },
			},
			on_match: { type: "action", action: "matched" },
		},
	],
	on_no_match: { type: "action", action: "default" },
});

const COMPOUND_CONFIG = JSON.stringify({
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

const NESTED_CONFIG = JSON.stringify({
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
				type: "matcher",
				matcher: {
					matchers: [
						{
							predicate: {
								type: "single",
								input: {
									type_url: "xuma.test.v1.StringInput",
									config: { key: "region" },
								},
								value_match: { Exact: "us" },
							},
							on_match: { type: "action", action: "premium_us" },
						},
					],
					on_no_match: { type: "action", action: "premium_other" },
				},
			},
		},
	],
	on_no_match: { type: "action", action: "default" },
});

// ── Helpers ──────────────────────────────────────────────────────────────────

function buildRegistry() {
	return register(new RegistryBuilder()).build();
}

// ── Registry construction ───────────────────────────────────────────────────

summary(() => {
	bench("config_registry_build", () => buildRegistry());
});

// ── Config loading: JSON → parse → Registry → Matcher ───────────────────────

summary(() => {
	const registry = buildRegistry();

	bench("config_load_simple", () => {
		const config = parseMatcherConfig(JSON.parse(SIMPLE_CONFIG));
		registry.loadMatcher(config);
	});

	bench("config_load_compound", () => {
		const config = parseMatcherConfig(JSON.parse(COMPOUND_CONFIG));
		registry.loadMatcher(config);
	});

	bench("config_load_nested", () => {
		const config = parseMatcherConfig(JSON.parse(NESTED_CONFIG));
		registry.loadMatcher(config);
	});
});

// ── Evaluation parity ───────────────────────────────────────────────────────

summary(() => {
	const registry = buildRegistry();
	const configMatcher = registry.loadMatcher(
		parseMatcherConfig(JSON.parse(SIMPLE_CONFIG)),
	);

	const compilerMatcher = new Matcher<Record<string, string>, string>(
		[
			new FieldMatcher(
				new SinglePredicate(new DictInput("role"), new ExactMatcher("admin")),
				new Action("matched"),
			),
		],
		new Action("default"),
	);

	const ctx = { role: "admin" };

	bench("config_evaluate_simple", () => configMatcher.evaluate(ctx));
	bench("compiler_evaluate_simple", () => compilerMatcher.evaluate(ctx));
});

// ── Head-to-head: config load vs manual construction ────────────────────────
// NOTE: config_construct_simple duplicates config_load_simple intentionally —
// both appear in the same mitata summary group to compare config vs compiler
// construction side-by-side in benchmark output.

summary(() => {
	const registry = buildRegistry();

	bench("config_construct_simple", () => {
		const config = parseMatcherConfig(JSON.parse(SIMPLE_CONFIG));
		registry.loadMatcher(config);
	});

	bench("compiler_construct_simple", () =>
		new Matcher<Record<string, string>, string>(
			[
				new FieldMatcher(
					new SinglePredicate(
						new DictInput("role"),
						new ExactMatcher("admin"),
					),
					new Action("matched"),
				),
			],
			new Action("default"),
		),
	);
});

await run();
