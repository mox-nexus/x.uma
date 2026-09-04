import { describe, expect, it } from "bun:test";
import { compileRouteMatches } from "../src/http/gateway.ts";
import { MatcherError } from "../src/matcher.ts";
import { loadHttpFixtures } from "./helpers/fixture-loader.ts";

const fixtures = loadHttpFixtures();

describe("http conformance", () => {
	for (const fixture of fixtures) {
		it(`${fixture.fixtureName}::${fixture.caseName}`, () => {
			if (fixture.unlisted === true) {
				// Must not work here. See the ledger note in fixture-loader.ts.
				expect(fixture.compile).toThrow();
				return;
			}
			if (fixture.errorContains !== undefined) {
				expect(fixture.compile).toThrow(fixture.errorContains);
				return;
			}
			if (fixture.matcher === null || fixture.request === null) {
				throw new Error("non-error fixture is missing its matcher or request");
			}
			const result = fixture.matcher.evaluate(fixture.request);
			expect(result).toBe(fixture.expect);
		});
	}

	it("the error path is not inert", () => {
		// A runner that never reached the error branch would pass everything above.
		expect(fixtures.some((f) => f.errorContains !== undefined)).toBe(true);
		expect(() => compileRouteMatches([], "hit")).toThrow(MatcherError);
	});
});
