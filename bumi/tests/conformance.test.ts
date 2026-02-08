import { describe, expect, it } from "bun:test";
import { loadCoreFixtures } from "./helpers/fixture-loader.ts";

const fixtures = loadCoreFixtures();

describe("core conformance", () => {
	for (const fixture of fixtures) {
		it(`${fixture.fixtureName}::${fixture.caseName}`, () => {
			const result = fixture.matcher.evaluate(fixture.context);
			expect(result).toBe(fixture.expect);
		});
	}
});
