/**
 * ReDoS safety demonstration for bumi (Pure TypeScript).
 *
 * JavaScript's RegExp engine uses backtracking — vulnerable to
 * catastrophic backtracking on pathological patterns like `(a+)+$`.
 *
 * Pattern: `(a+)+$` against `"a" * N + "X"`
 * - JS RegExp: O(2^N) — hangs at N=25+
 * - Rust `regex`: O(N) — microseconds even at N=100
 *
 * SAFETY: Benchmarks capped at N=20. Do NOT increase without understanding
 * that N=25 can take seconds and N=30 may hang indefinitely.
 *
 * Run: cd bumi && bun run bench/redos.bench.ts
 */

import { bench, run, summary } from "mitata";

import { Action, FieldMatcher, Matcher, RegexMatcher, SinglePredicate } from "../src/index.ts";
import type { DataInput, MatchingData } from "../src/index.ts";

// ── Fixtures ─────────────────────────────────────────────────────────────────

const REDOS_PATTERN = String.raw`(a+)+$`;
const SAFE_PATTERN = String.raw`^a+X$`;

interface Ctx {
	readonly value: string;
}

const valueInput: DataInput<Ctx> = {
	get(ctx: Ctx): MatchingData {
		return ctx.value;
	},
};

function pathologicalInput(n: number): string {
	return "a".repeat(n) + "X";
}

// ── Raw regex matcher (ReDoS pattern) ────────────────────────────────────────

summary(() => {
	for (const n of [5, 10, 15, 20]) {
		const matcher = new RegexMatcher(REDOS_PATTERN);
		const value = pathologicalInput(n);
		bench(`redos_regex_n${n}`, () => matcher.matches(value));
	}
});

// ── Full pipeline (ReDoS pattern through Matcher) ────────────────────────────

summary(() => {
	for (const n of [10, 20]) {
		const m = new Matcher(
			[
				new FieldMatcher(
					new SinglePredicate(valueInput, new RegexMatcher(REDOS_PATTERN)),
					new Action("blocked"),
				),
			],
			new Action("allowed"),
		);
		const ctx: Ctx = { value: pathologicalInput(n) };
		bench(`redos_pipeline_n${n}`, () => m.evaluate(ctx));
	}
});

// ── Safe regex for comparison ────────────────────────────────────────────────

summary(() => {
	for (const n of [10, 20]) {
		const matcher = new RegexMatcher(SAFE_PATTERN);
		const value = pathologicalInput(n);
		bench(`safe_regex_n${n}`, () => matcher.matches(value));
	}
});

await run();
