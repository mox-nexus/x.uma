/**
 * Compile benchmarks for bumi (Pure TypeScript).
 *
 * Measures matcher construction cost: string matcher creation,
 * predicate tree building, and matcher tree compilation.
 *
 * Run: cd bumi && bun run bench/compile.bench.ts
 */

import { bench, run, summary } from "mitata";

import {
	Action,
	ContainsMatcher,
	ExactMatcher,
	FieldMatcher,
	Matcher,
	PrefixMatcher,
	RegexMatcher,
	SinglePredicate,
	SuffixMatcher,
} from "../src/index.ts";
import type { DataInput, MatchingData } from "../src/index.ts";

// ── Fixtures ─────────────────────────────────────────────────────────────────

interface Ctx {
	readonly value: string;
}

const valueInput: DataInput<Ctx> = {
	get(ctx: Ctx): MatchingData {
		return ctx.value;
	},
};

// ── StringMatcher construction ───────────────────────────────────────────────

summary(() => {
	bench("compile_exact", () => new ExactMatcher("/api/v1/users"));
	bench("compile_prefix", () => new PrefixMatcher("/api/"));
	bench("compile_suffix", () => new SuffixMatcher(".json"));
	bench("compile_contains_ci", () => new ContainsMatcher("Content-Type", true));
});

summary(() => {
	bench("compile_regex_simple", () => new RegexMatcher(String.raw`^/api/v\d+/users$`));
	bench(
		"compile_regex_complex",
		() =>
			new RegexMatcher(
				String.raw`^/api/v[1-3]/(users|orders|products)/[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$`,
			),
	);
});

// ── Matcher tree construction at scale ───────────────────────────────────────

function buildNExactRules(n: number): Matcher<Ctx, string> {
	const rules: FieldMatcher<Ctx, string>[] = [];
	for (let i = 0; i < n; i++) {
		rules.push(
			new FieldMatcher(
				new SinglePredicate(valueInput, new ExactMatcher(`/route/${i}`)),
				new Action(`action_${i}`),
			),
		);
	}
	return new Matcher(rules, null);
}

function buildNRegexRules(n: number): Matcher<Ctx, string> {
	const rules: FieldMatcher<Ctx, string>[] = [];
	for (let i = 0; i < n; i++) {
		rules.push(
			new FieldMatcher(
				new SinglePredicate(valueInput, new RegexMatcher(`^/route/${i}/\\d+$`)),
				new Action(`action_${i}`),
			),
		);
	}
	return new Matcher(rules, null);
}

summary(() => {
	for (const n of [10, 50, 100, 200]) {
		bench(`compile_${n}_exact_rules`, () => buildNExactRules(n));
	}
});

summary(() => {
	for (const n of [10, 50, 100]) {
		bench(`compile_${n}_regex_rules`, () => buildNRegexRules(n));
	}
});

await run();
