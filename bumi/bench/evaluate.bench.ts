/**
 * Evaluate benchmarks for bumi (Pure TypeScript).
 *
 * Measures the hot path: predicate evaluation, first-match-wins scanning,
 * miss-heavy workloads, and scaling.
 *
 * Run: cd bumi && bun run bench/evaluate.bench.ts
 */

import { bench, run, summary } from "mitata";

import {
	Action,
	And,
	ContainsMatcher,
	ExactMatcher,
	FieldMatcher,
	Matcher,
	Or,
	PrefixMatcher,
	RegexMatcher,
	SinglePredicate,
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

function fieldMatcher(expected: string, action: string): FieldMatcher<Ctx, string> {
	return new FieldMatcher(
		new SinglePredicate(valueInput, new ExactMatcher(expected)),
		new Action(action),
	);
}

function prefixFieldMatcher(prefix: string, action: string): FieldMatcher<Ctx, string> {
	return new FieldMatcher(
		new SinglePredicate(valueInput, new PrefixMatcher(prefix)),
		new Action(action),
	);
}

function regexFieldMatcher(pattern: string, action: string): FieldMatcher<Ctx, string> {
	return new FieldMatcher(
		new SinglePredicate(valueInput, new RegexMatcher(pattern)),
		new Action(action),
	);
}

// ── Core scenarios ───────────────────────────────────────────────────────────

summary(() => {
	const hitMatcher = new Matcher([fieldMatcher("/api", "api_backend")], new Action("default"));
	const hitCtx: Ctx = { value: "/api" };

	const missMatcher = new Matcher([fieldMatcher("/api", "api_backend")], new Action("default"));
	const missCtx: Ctx = { value: "/other" };

	bench("exact_match_hit", () => hitMatcher.evaluate(hitCtx));
	bench("exact_match_miss", () => missMatcher.evaluate(missCtx));
});

summary(() => {
	const m = new Matcher([prefixFieldMatcher("/api/", "api")], new Action("default"));
	const ctx: Ctx = { value: "/api/v2/users/123" };
	bench("prefix_match_hit", () => m.evaluate(ctx));
});

summary(() => {
	const m = new Matcher(
		[regexFieldMatcher(String.raw`^/api/v\d+/users/\d+$`, "user_route")],
		new Action("default"),
	);
	const hitCtx: Ctx = { value: "/api/v2/users/12345" };
	const missCtx: Ctx = { value: "/other/path" };

	bench("regex_match_hit", () => m.evaluate(hitCtx));
	bench("regex_match_miss", () => m.evaluate(missCtx));
});

// ── Predicate composition ────────────────────────────────────────────────────

summary(() => {
	const andMatcher = new Matcher(
		[
			new FieldMatcher(
				new And([
					new SinglePredicate(valueInput, new ContainsMatcher("hello")),
					new SinglePredicate(valueInput, new ContainsMatcher("world")),
				]),
				new Action("matched"),
			),
		],
		null,
	);
	const andCtx: Ctx = { value: "hello world" };

	const orMatcher = new Matcher(
		[
			new FieldMatcher(
				new Or([
					new SinglePredicate(valueInput, new ExactMatcher("hello")),
					new SinglePredicate(valueInput, new ExactMatcher("world")),
				]),
				new Action("matched"),
			),
		],
		null,
	);
	const orCtx: Ctx = { value: "hello" };

	bench("predicate_and_all_match", () => andMatcher.evaluate(andCtx));
	bench("predicate_or_first_matches", () => orMatcher.evaluate(orCtx));
});

// ── Scaling: rule count ──────────────────────────────────────────────────────

function makeNRuleMatcher(n: number, includeTarget: boolean): Matcher<Ctx, string> {
	const count = includeTarget ? n - 1 : n;
	const rules: FieldMatcher<Ctx, string>[] = [];
	for (let i = 0; i < count; i++) {
		rules.push(fieldMatcher(`rule_${i}`, `action_${i}`));
	}
	if (includeTarget) {
		rules.push(fieldMatcher("target", "found"));
	}
	return new Matcher(rules, new Action("fallback"));
}

summary(() => {
	const targetCtx: Ctx = { value: "target" };
	for (const n of [10, 50, 100, 200]) {
		const m = makeNRuleMatcher(n, true);
		bench(`rule_count_${n}_last_match`, () => m.evaluate(targetCtx));
	}
});

summary(() => {
	const missCtx: Ctx = { value: "no_match" };
	for (const n of [10, 100]) {
		const m = makeNRuleMatcher(n, false);
		bench(`rule_count_${n}_miss`, () => m.evaluate(missCtx));
	}
});

// ── Miss-heavy workload ──────────────────────────────────────────────────────

summary(() => {
	const rules: FieldMatcher<Ctx, string>[] = [];
	for (let i = 0; i < 10; i++) {
		rules.push(fieldMatcher(`/blocked/${i}`, `block_${i}`));
	}
	const m = new Matcher(rules, new Action("allow"));
	const ctx: Ctx = { value: "/api/v1/users" };

	bench("miss_heavy_10_rules", () => m.evaluate(ctx));
});

await run();
