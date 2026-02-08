import { describe, expect, it } from "bun:test";
import { loadHttpFixtures } from "./helpers/fixture-loader.ts";

const fixtures = loadHttpFixtures();

describe("http conformance", () => {
	for (const fixture of fixtures) {
		it(`${fixture.fixtureName}::${fixture.caseName}`, () => {
			const result = fixture.matcher.evaluate(fixture.request);
			expect(result).toBe(fixture.expect);
		});
	}
});
