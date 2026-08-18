/**
 * Config-path benchmarks for bumi.
 *
 * Measures the cost of JSON config -> Registry -> Matcher construction, and
 * compares config-loaded evaluation against compiler-built evaluation.
 *
 * The configs are canonical protojson (DECISIONS.md D-026) fed through
 * `parseProtojson`, the same reader `Registry.loadMatcher` consumes from in
 * production -- this measures the real config path, not a synthetic one.
 *
 * Run: cd bumi && bun run bench/config.bench.ts
 */

import { bench, run, summary } from "mitata";

import {
	Action,
	ExactMatcher,
	FieldMatcher,
	Matcher,
	parseProtojson,
	RegistryBuilder,
	SinglePredicate,
} from "../src/index.ts";
import { DictInput, register } from "../src/testing.ts";

// ── Shared protojson configs (identical shape across all implementations) ────

function mapInput(key: string) {
	return { "@type": "type.googleapis.com/xuma.kv.v1.MapInput", key };
}

function namedAction(name: string) {
	return { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", name };
}

function single(inputName: string, key: string, variant: string, value: string) {
	return {
		singlePredicate: {
			input: { name: inputName, typedConfig: mapInput(key) },
			valueMatch: { [variant]: value },
		},
	};
}

function actionMatch(name: string) {
	return { action: { name, typedConfig: namedAction(name) } };
}

const SIMPLE_CONFIG = JSON.stringify({
	matcherList: {
		matchers: [
			{ predicate: single("role", "role", "exact", "admin"), onMatch: actionMatch("matched") },
		],
	},
	onNoMatch: actionMatch("default"),
});

const COMPOUND_CONFIG = JSON.stringify({
	matcherList: {
		matchers: [
			{
				predicate: {
					andMatcher: {
						predicate: [
							single("role", "role", "exact", "admin"),
							single("org", "org", "prefix", "acme"),
						],
					},
				},
				onMatch: actionMatch("admin_acme"),
			},
		],
	},
});

const NESTED_CONFIG = JSON.stringify({
	matcherList: {
		matchers: [
			{
				predicate: single("tier", "tier", "exact", "premium"),
				onMatch: {
					matcher: {
						matcherList: {
							matchers: [
								{
									predicate: single("region", "region", "exact", "us"),
									onMatch: actionMatch("premium_us"),
								},
							],
						},
						onNoMatch: actionMatch("premium_other"),
					},
				},
			},
		],
	},
	onNoMatch: actionMatch("default"),
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
		const config = parseProtojson(JSON.parse(SIMPLE_CONFIG));
		registry.loadMatcher(config);
	});

	bench("config_load_compound", () => {
		const config = parseProtojson(JSON.parse(COMPOUND_CONFIG));
		registry.loadMatcher(config);
	});

	bench("config_load_nested", () => {
		const config = parseProtojson(JSON.parse(NESTED_CONFIG));
		registry.loadMatcher(config);
	});
});

// ── Evaluation parity ───────────────────────────────────────────────────────

summary(() => {
	const registry = buildRegistry();
	const configMatcher = registry.loadMatcher(
		parseProtojson(JSON.parse(SIMPLE_CONFIG)),
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
		const config = parseProtojson(JSON.parse(SIMPLE_CONFIG));
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
