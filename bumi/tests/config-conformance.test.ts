/**
 * Config conformance tests for bumi.
 *
 * Loads YAML fixtures from spec/tests/06_config/ and runs them through
 * bumi's registry config loading path -- the same fixtures that rumi,
 * puma, and both crusty bindings must also pass.
 *
 * Run with: cd bumi && bun test tests/config-conformance.test.ts
 */

import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { loadAll } from "js-yaml";

import { ConfigParseError, parseMatcherConfig } from "../src/config.ts";
import { MatcherError } from "../src/matcher.ts";
import { RegistryBuilder } from "../src/registry.ts";
import { register } from "../src/testing.ts";

const SPEC_DIR = resolve(import.meta.dir, "..", "..", "spec", "tests");
const CONFIG_DIR = join(SPEC_DIR, "06_config");

function makeRegistry() {
	const builder = new RegistryBuilder<Record<string, string>>();
	register(builder);
	return builder.build();
}

interface ConfigFixture {
	name: string;
	config: unknown;
	cases:
		| Array<{
				name: string;
				context: Record<string, unknown>;
				expect: string | null;
		  }>
		| undefined;
	expect_error: boolean | undefined;
	_source: string;
}

function loadConfigFixtures(): ConfigFixture[] {
	const fixtures: ConfigFixture[] = [];
	let files: string[];

	try {
		files = readdirSync(CONFIG_DIR)
			.filter((f) => f.endsWith(".yaml"))
			.sort();
	} catch {
		return fixtures;
	}

	for (const file of files) {
		const content = readFileSync(join(CONFIG_DIR, file), "utf-8");
		// biome-ignore lint/suspicious/noExplicitAny: YAML document parsing
		const docs = loadAll(content) as any[];
		for (const doc of docs) {
			if (doc == null || typeof doc !== "object") continue;
			fixtures.push({
				name: doc.name as string,
				config: doc.config,
				cases: doc.cases as ConfigFixture["cases"],
				expect_error: doc.expect_error as boolean | undefined,
				_source: file,
			});
		}
	}
	return fixtures;
}

const allFixtures = loadConfigFixtures();
const positiveFixtures = allFixtures.filter((f) => !f.expect_error);
const errorFixtures = allFixtures.filter((f) => f.expect_error);

describe("config conformance: positive", () => {
	for (const fixture of positiveFixtures) {
		const id = `${fixture._source}::${fixture.name}`;

		test(id, () => {
			const registry = makeRegistry();
			const config = parseMatcherConfig(fixture.config);
			const matcher = registry.loadMatcher(config);

			for (const c of fixture.cases ?? []) {
				const ctx: Record<string, string> = {};
				for (const [k, v] of Object.entries(c.context)) {
					ctx[String(k)] = String(v);
				}
				const actual = matcher.evaluate(ctx);
				const expected = c.expect ?? null;
				expect(actual).toBe(expected);
			}
		});
	}
});

describe("config conformance: errors", () => {
	for (const fixture of errorFixtures) {
		const id = `${fixture._source}::${fixture.name}`;

		test(id, () => {
			const registry = makeRegistry();

			let config: ReturnType<typeof parseMatcherConfig>;
			try {
				config = parseMatcherConfig(fixture.config);
			} catch (e) {
				// Parse error -- expected for error fixtures
				expect(e instanceof ConfigParseError || e instanceof Error).toBe(true);
				return;
			}

			// Parse succeeded, loading must fail
			expect(() => registry.loadMatcher(config)).toThrow(MatcherError);
		});
	}
});
