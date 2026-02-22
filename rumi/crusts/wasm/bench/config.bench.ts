/**
 * Head-to-head config benchmarks: bumi (pure TS) vs bumi-crusty (WASM).
 *
 * Compares the config loading path across both implementations to isolate:
 * 1. Config parsing overhead — JSON → config types
 * 2. Registry loading — type URL lookup + factory invocation
 * 3. Evaluation parity — config-loaded matcher evaluation speed
 *
 * Run:
 *   cd rumi/crusts/wasm
 *   wasm-pack build --target nodejs
 *   bun run bench/config.bench.ts
 */

import { bench, run, summary } from "mitata";

// bumi-crusty (WASM Rust bindings)
import { HttpMatcher, TestMatcher } from "../pkg/bumi_crusty.js";

// bumi (pure TypeScript)
import {
	Action,
	ExactMatcher,
	FieldMatcher,
	Matcher,
	RegistryBuilder,
	SinglePredicate,
	parseMatcherConfig,
} from "../../../../bumi/src/index.ts";
import { DictInput, register } from "../../../../bumi/src/testing.ts";

// ── Shared JSON configs ──────────────────────────────────────────────────────

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

const HTTP_SIMPLE_CONFIG = JSON.stringify({
	matchers: [
		{
			predicate: {
				type: "single",
				input: {
					type_url: "xuma.http.v1.PathInput",
					config: {},
				},
				value_match: { Exact: "/api/v1/users" },
			},
			on_match: { type: "action", action: "users_api" },
		},
	],
	on_no_match: { type: "action", action: "not_found" },
});

// ── Helpers ──────────────────────────────────────────────────────────────────

function bumiRegistry() {
	return register(new RegistryBuilder()).build();
}

// ── Config load: test domain ────────────────────────────────────────────────

summary(() => {
	const registry = bumiRegistry();

	bench("crusty_config_load_simple", () =>
		TestMatcher.fromConfig(SIMPLE_CONFIG),
	);

	bench("bumi_config_load_simple", () => {
		const config = parseMatcherConfig(JSON.parse(SIMPLE_CONFIG));
		registry.loadMatcher(config);
	});
});

summary(() => {
	const registry = bumiRegistry();

	bench("crusty_config_load_compound", () =>
		TestMatcher.fromConfig(COMPOUND_CONFIG),
	);

	bench("bumi_config_load_compound", () => {
		const config = parseMatcherConfig(JSON.parse(COMPOUND_CONFIG));
		registry.loadMatcher(config);
	});
});

// ── Config evaluate: test domain ────────────────────────────────────────────

summary(() => {
	const crusty = TestMatcher.fromConfig(SIMPLE_CONFIG);
	const registry = bumiRegistry();
	const bumi = registry.loadMatcher(
		parseMatcherConfig(JSON.parse(SIMPLE_CONFIG)),
	);

	const ctx = { role: "admin" };

	bench("crusty_config_evaluate_simple", () => crusty.evaluate(ctx));
	bench("bumi_config_evaluate_simple", () => bumi.evaluate(ctx));
});

// ── HTTP domain (crusty only — no pure bumi HTTP registry yet) ──────────────

summary(() => {
	bench("crusty_http_config_load", () =>
		HttpMatcher.fromConfig(HTTP_SIMPLE_CONFIG),
	);
});

summary(() => {
	const httpMatcher = HttpMatcher.fromConfig(HTTP_SIMPLE_CONFIG);
	const ctx = { method: "GET", path: "/api/v1/users" };

	bench("crusty_http_config_evaluate", () => httpMatcher.evaluate(ctx));
});

await run();
