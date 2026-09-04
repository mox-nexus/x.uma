/**
 * The playground diagram must describe the config it was given.
 *
 * `docs/experience` had no tests at all until 2026-09-01 — 2,400 lines of
 * playground behind `svelte-check`, which checks types and not truth. A diagram
 * that drew the wrong thing type-checked perfectly, and the playground is the
 * one surface a visitor actually touches.
 *
 * The bug that prompted these: `walkOnMatch` dropped its `isFallback` flag when
 * an `onNoMatch` held a nested matcher rather than a bare action, so the whole
 * fallback branch rendered identically to the match branch — same node type, no
 * `no-match` edge. For a tool whose entire job is answering "why did this
 * match?", that is the explanation being wrong.
 */

import { describe, expect, test } from "bun:test";
import { configToGraph } from "../src/lib/playground/graph/config-to-graph.js";

const mapInput = (key: string) => ({
	name: key,
	typedConfig: { "@type": "type.googleapis.com/xuma.kv.v1.MapInput", key },
});

const action = (name: string) => ({
	action: {
		name,
		typedConfig: { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", name },
	},
});

const rule = (key: string, value: string, act: string) => ({
	predicate: { singlePredicate: { input: mapInput(key), valueMatch: { exact: value } } },
	onMatch: action(act),
});

const graph = (cfg: unknown) => configToGraph(JSON.stringify(cfg), "config");
const actions = (g: ReturnType<typeof graph>) =>
	(g.nodes ?? [])
		.filter((n) => n.type === "action" || n.type === "fallback")
		.map((n) => [n.type, (n.data as { action?: string }).action]);

describe("fallback branches are drawn as fallbacks", () => {
	test("a bare action under onNoMatch", () => {
		const g = graph({
			matcherList: { matchers: [rule("k", "hit", "PRIMARY")] },
			onNoMatch: action("FALLBACK"),
		});
		expect(actions(g)).toEqual([
			["action", "PRIMARY"],
			["fallback", "FALLBACK"],
		]);
		expect((g.edges ?? []).filter((e) => e.kind === "no-match")).toHaveLength(1);
	});

	test("a nested matcher under onNoMatch — the whole branch, however deep", () => {
		// The regression. Everything reachable from onNoMatch is fallback,
		// including actions two levels down inside a nested matcher.
		const g = graph({
			matcherList: { matchers: [rule("k", "hit", "PRIMARY")] },
			onNoMatch: {
				matcher: { matcherList: { matchers: [rule("k", "other", "INSIDE_FALLBACK")] } },
			},
		});
		expect(actions(g)).toEqual([
			["action", "PRIMARY"],
			["fallback", "INSIDE_FALLBACK"],
		]);
		// The edge into the nested matcher is a no-match edge too, so the
		// fallback path reads as one route rather than starting mid-air.
		expect((g.edges ?? []).filter((e) => e.kind === "no-match").length).toBeGreaterThanOrEqual(2);
	});

	test("a nested matcher under a normal onMatch stays a match", () => {
		// The control. A fix that marked every nested matcher as a fallback
		// would pass both tests above and be just as wrong.
		const g = graph({
			matcherList: {
				matchers: [
					{
						predicate: { singlePredicate: { input: mapInput("k"), valueMatch: { exact: "hit" } } },
						onMatch: { matcher: { matcherList: { matchers: [rule("j", "deep", "NESTED")] } } },
					},
				],
			},
		});
		expect(actions(g)).toEqual([["action", "NESTED"]]);
		expect((g.edges ?? []).filter((e) => e.kind === "no-match")).toHaveLength(0);
	});
});

describe("the diagram counts what the config holds", () => {
	test("a list reports its rule count", () => {
		const g = graph({
			matcherList: { matchers: [rule("k", "a", "A"), rule("k", "b", "B")] },
		});
		const m = (g.nodes ?? []).find((n) => n.type === "matcher");
		expect((m?.data as { count?: number }).count).toBe(2);
	});

	test("a tree reports its entry count, not zero", () => {
		// A tree has entries, not field matchers. Counting the wrong field drew
		// a populated config as "Matcher (0)" with no rule nodes.
		const g = graph({
			matcherTree: {
				input: mapInput("k"),
				exactMatchMap: { map: { alpha: action("A"), beta: action("B") } },
			},
		});
		const m = (g.nodes ?? []).find((n) => n.type === "matcher");
		expect((m?.data as { count?: number }).count).toBe(2);
		expect((m?.data as { label?: string }).label).toContain("tree");
	});
});
