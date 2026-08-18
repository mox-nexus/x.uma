/**
 * Conformance over the protojson fixtures.
 *
 * `spec/tests/07_protojson/` holds the format x.uma actually implements. The
 * four older dialects are transitional.
 *
 * Each fixture names the implementations expected to run it, and this runner
 * holds that ledger in **both** directions: if `typescript` is listed the
 * fixture must run, and if it is not listed the fixture must *fail* to run. A
 * skip that quietly starts working means the ledger is reporting on work
 * somebody already finished, and a suite that lies about its own coverage is
 * worse than one that is red.
 */

import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { loadAll } from "js-yaml";

import { RegistryBuilder, parseProtojson } from "../src/index.ts";
import { register } from "../src/testing.ts";

const ME = "typescript";
const PROTO_DIR = join(import.meta.dir, "..", "..", "spec", "tests", "07_protojson");

interface Fixture {
	name: string;
	proto_matcher: unknown;
	implementations?: string[];
	expect_error?: boolean;
	error_contains?: string;
	cases?: { name: string; context?: Record<string, string>; expect: string | null }[];
}

function load(): Fixture[] {
	const out: Fixture[] = [];
	for (const file of readdirSync(PROTO_DIR).sort()) {
		if (!file.endsWith(".yaml") && !file.endsWith(".yml")) continue;
		for (const doc of loadAll(readFileSync(join(PROTO_DIR, file), "utf8"))) {
			const d = doc as Fixture | null;
			if (d && "proto_matcher" in d) out.push(d);
		}
	}
	return out;
}

const FIXTURES = load();

function build(fixture: Fixture) {
	const registry = register(new RegistryBuilder()).build();
	return registry.loadMatcher(parseProtojson(fixture.proto_matcher));
}

describe("protojson conformance", () => {
	test("the corpus is not empty", () => {
		// A runner over zero fixtures passes and proves nothing.
		expect(FIXTURES.length).toBeGreaterThan(0);
	});

	for (const fixture of FIXTURES) {
		test(fixture.name, () => {
			const expected = fixture.implementations ?? ["rust", "python", "typescript"];

			if (!expected.includes(ME)) {
				// Not listed, so it must not work. See the file docstring.
				expect(() => build(fixture)).toThrow();
				return;
			}

			if (fixture.expect_error) {
				expect(() => build(fixture)).toThrow();
				if (fixture.error_contains) {
					let message = "";
					try {
						build(fixture);
					} catch (e) {
						message = e instanceof Error ? e.message : String(e);
					}
					expect(message).toContain(fixture.error_contains);
				}
				return;
			}

			const matcher = build(fixture);
			for (const c of fixture.cases ?? []) {
				expect(matcher.evaluate(c.context ?? {})).toBe(c.expect);
			}
		});
	}
});
