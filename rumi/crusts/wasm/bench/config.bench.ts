/**
 * Head-to-head config benchmarks: bumi vs xuma-crust (WASM).
 *
 * Compares the config loading path across both implementations to isolate:
 * 1. Config parsing overhead — JSON → config types
 * 2. Registry loading — type URL lookup + factory invocation
 * 3. Evaluation parity — config-loaded matcher evaluation speed
 *
 * The configs are canonical protojson (DECISIONS.md D-026), fed through
 * `parseProtojson` on the bumi side and `fromConfig` on the crust side —
 * both are the real production entry points, not synthetic ones.
 *
 * Run:
 *   cd rumi/crusts/wasm
 *   wasm-pack build --target web
 *   bun run bench/config.bench.ts
 */

import { bench, run, summary } from "mitata";

// xuma-crust (WASM Rust bindings)
import init, { HttpMatcher, TestMatcher } from "../pkg/xuma_crust.js";
await init();

// bumi
import { parseProtojson, RegistryBuilder } from "../../../../bumi/src/index.ts";
import { register } from "../../../../bumi/src/testing.ts";

// ── Shared protojson configs ─────────────────────────────────────────────────

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

const HTTP_SIMPLE_CONFIG = JSON.stringify({
	matcherList: {
		matchers: [
			{
				predicate: {
					singlePredicate: {
						input: { name: "path", typedConfig: { "@type": "type.googleapis.com/xuma.http.v1.PathInput" } },
						valueMatch: { exact: "/api/v1/users" },
					},
				},
				onMatch: actionMatch("users_api"),
			},
		],
	},
	onNoMatch: actionMatch("not_found"),
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
		const config = parseProtojson(JSON.parse(SIMPLE_CONFIG));
		registry.loadMatcher(config);
	});
});

summary(() => {
	const registry = bumiRegistry();

	bench("crusty_config_load_compound", () =>
		TestMatcher.fromConfig(COMPOUND_CONFIG),
	);

	bench("bumi_config_load_compound", () => {
		const config = parseProtojson(JSON.parse(COMPOUND_CONFIG));
		registry.loadMatcher(config);
	});
});

// ── Config evaluate: test domain ────────────────────────────────────────────

summary(() => {
	const crusty = TestMatcher.fromConfig(SIMPLE_CONFIG);
	const registry = bumiRegistry();
	const bumi = registry.loadMatcher(
		parseProtojson(JSON.parse(SIMPLE_CONFIG)),
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
