/**
 * Boundary cost of the WASM binding on the config path.
 *
 * What the numbers isolate:
 * 1. Config load — protojson text in, a compiled matcher out; the parse, the
 *    registry lookups and the factory calls all run in WASM
 * 2. Evaluate — a context object read across the boundary, one walk, a
 *    string back
 *
 * The configs are canonical protojson (DECISIONS.md D-026) fed through
 * `fromConfig`, the real entry point, not a synthetic one.
 *
 * Run:
 *   cd rumi/crusts/wasm
 *   wasm-pack build --target web
 *   bun run bench/config.bench.ts
 */

import { bench, run, summary } from "mitata";

import init, { HttpMatcher, KvMatcher } from "../pkg/xuma_crust.js";
await init();

// ── Configs ──────────────────────────────────────────────────────────────────

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

// ── Load: key-value domain ───────────────────────────────────────────────────

summary(() => {
	bench("crusty_config_load_simple", () => KvMatcher.fromConfig(SIMPLE_CONFIG));
});

summary(() => {
	bench("crusty_config_load_compound", () => KvMatcher.fromConfig(COMPOUND_CONFIG));
});

// ── Evaluate: key-value domain ───────────────────────────────────────────────

summary(() => {
	const crusty = KvMatcher.fromConfig(SIMPLE_CONFIG);
	const ctx = { role: "admin" };

	bench("crusty_config_evaluate_simple", () => crusty.evaluate(ctx));
});

// ── HTTP domain ──────────────────────────────────────────────────────────────

summary(() => {
	bench("crusty_http_config_load", () => HttpMatcher.fromConfig(HTTP_SIMPLE_CONFIG));
});

summary(() => {
	const httpMatcher = HttpMatcher.fromConfig(HTTP_SIMPLE_CONFIG);
	const ctx = { method: "GET", path: "/api/v1/users" };

	bench("crusty_http_config_evaluate", () => httpMatcher.evaluate(ctx));
});

await run();
