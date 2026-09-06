/**
 * Head-to-head: bumi (pure TypeScript) vs xuma-crust (WASM Rust bindings).
 *
 * Compares identical workloads through both implementations to isolate:
 * 1. WASM overhead — boundary crossing cost (JS ↔ WASM)
 * 2. Regex engine — JS RegExp (backtracking) vs Rust regex (linear time)
 * 3. Compile cost — TS class construction vs Rust struct compilation via WASM
 *
 * Run:
 *   cd rumi/crusts/wasm
 *   wasm-pack build --target web
 *   bun run bench/crusty.bench.ts
 */

import { bench, run, summary } from "mitata";

// xuma-crust (WASM Rust bindings)
import init, { HookMatcher, StringMatch } from "../pkg/xuma_crust.js";
await init();

// bumi (pure TypeScript) — build equivalent matchers manually
import {
	Action,
	And,
	ContainsMatcher,
	ExactMatcher,
	FieldMatcher,
	Matcher,
	PrefixMatcher,
	RegexMatcher,
	SinglePredicate,
} from "../../../../bumi/src/index.ts";
import type { DataInput, MatchingData } from "../../../../bumi/src/index.ts";

// ── Pure TypeScript fixtures (mirror the hook matcher domain) ────────────────

interface HookCtx {
	readonly event: string;
	readonly toolName?: string;
	readonly command?: string;
}

const eventInput: DataInput<HookCtx> = {
	get(ctx: HookCtx): MatchingData {
		return ctx.event;
	},
};

const toolNameInput: DataInput<HookCtx> = {
	get(ctx: HookCtx): MatchingData {
		return ctx.toolName ?? null;
	},
};

const commandInput: DataInput<HookCtx> = {
	get(ctx: HookCtx): MatchingData {
		return ctx.command ?? null;
	},
};

// ── Compile benchmarks ───────────────────────────────────────────────────────

summary(() => {
	bench("crusty_compile_simple", () =>
		HookMatcher.compile([{ event: "PreToolUse" }], "matched"),
	);

	bench("bumi_compile_simple", () =>
		new Matcher(
			[
				new FieldMatcher(
					new SinglePredicate(eventInput, new ExactMatcher("PreToolUse")),
					new Action("matched"),
				),
			],
			null,
		),
	);
});

summary(() => {
	bench("crusty_compile_complex", () =>
		HookMatcher.compile(
			[
				{
					event: "PreToolUse",
					toolName: StringMatch.regex(String.raw`^(Write|Edit|Bash)$`),
					arguments: [["command", StringMatch.contains("rm")]],
				},
			],
			"blocked",
			"allowed",
		),
	);

	bench("bumi_compile_complex", () =>
		new Matcher(
			[
				new FieldMatcher(
					new And([
						new SinglePredicate(eventInput, new ExactMatcher("PreToolUse")),
						new SinglePredicate(
							toolNameInput,
							new RegexMatcher(String.raw`^(Write|Edit|Bash)$`),
						),
						new SinglePredicate(commandInput, new ContainsMatcher("rm")),
					]),
					new Action("blocked"),
				),
			],
			new Action("allowed"),
		),
	);
});

// ── Evaluate benchmarks (exact match) ────────────────────────────────────────

summary(() => {
	const crusty = HookMatcher.compile([{ event: "PreToolUse" }], "matched");
	const bumi = new Matcher(
		[
			new FieldMatcher(
				new SinglePredicate(eventInput, new ExactMatcher("PreToolUse")),
				new Action("matched"),
			),
		],
		null,
	);

	// Both sides construct their context inside the measured region. Until
	// 2026-09-05 the bumi side reused a context hoisted out of the loop while the
	// crusty side built a fresh object every iteration and marshalled it across
	// the WASM boundary, so the pair timed two different amounts of work and every
	// ratio drawn from it was unearned. Allocating on both sides is the workload a
	// caller actually has: a request arrives, a context is built, a match is run.
	bench("crusty_exact_hit", () => crusty.evaluate({ event: "PreToolUse" }));
	bench("bumi_exact_hit", () => bumi.evaluate({ event: "PreToolUse" }));
	bench("crusty_exact_miss", () => crusty.evaluate({ event: "PostToolUse" }));
	bench("bumi_exact_miss", () => bumi.evaluate({ event: "PostToolUse" }));
});

// ── Evaluate benchmarks (regex) ──────────────────────────────────────────────

summary(() => {
	const crusty = HookMatcher.compile(
		[{ toolName: StringMatch.regex(String.raw`^mcp__\w+__\w+$`) }],
		"matched",
	);
	const bumi = new Matcher(
		[
			new FieldMatcher(
				new SinglePredicate(
					toolNameInput,
					new RegexMatcher(String.raw`^mcp__\w+__\w+$`),
				),
				new Action("matched"),
			),
		],
		null,
	);

	// Symmetric context construction — see the note in the exact-match group.
	bench("crusty_regex_hit", () =>
		crusty.evaluate({ event: "PreToolUse", toolName: "mcp__db__query" }),
	);
	bench("bumi_regex_hit", () =>
		bumi.evaluate({ event: "PreToolUse", toolName: "mcp__db__query" }),
	);
	bench("crusty_regex_miss", () =>
		crusty.evaluate({ event: "PreToolUse", toolName: "Write" }),
	);
	bench("bumi_regex_miss", () =>
		bumi.evaluate({ event: "PreToolUse", toolName: "Write" }),
	);
});

// ── Evaluate benchmarks (complex multi-field) ────────────────────────────────

summary(() => {
	const crusty = HookMatcher.compile(
		[
			{
				event: "PreToolUse",
				toolName: StringMatch.prefix("mcp__"),
				arguments: [["command", StringMatch.contains("drop")]],
			},
		],
		"blocked",
		"allowed",
	);

	const bumi = new Matcher(
		[
			new FieldMatcher(
				new And([
					new SinglePredicate(eventInput, new ExactMatcher("PreToolUse")),
					new SinglePredicate(toolNameInput, new PrefixMatcher("mcp__")),
					new SinglePredicate(commandInput, new ContainsMatcher("drop")),
				]),
				new Action("blocked"),
			),
		],
		new Action("allowed"),
	);

	// Symmetric context construction — see the note in the exact-match group. The
	// two sides still differ in shape (`arguments: {command}` vs a flat `command`)
	// because the two APIs differ, so the crusty side allocates one more object
	// than bumi does. That asymmetry is inherent to the interfaces rather than an
	// artefact of the harness, and it is small next to the boundary crossing.
	bench("crusty_complex_hit", () =>
		crusty.evaluate({
			event: "PreToolUse",
			toolName: "mcp__db__exec",
			arguments: { command: "DROP TABLE users" },
		}),
	);
	bench("bumi_complex_hit", () =>
		bumi.evaluate({
			event: "PreToolUse",
			toolName: "mcp__db__exec",
			command: "DROP TABLE users",
		}),
	);
	bench("crusty_complex_miss", () =>
		crusty.evaluate({
			event: "PostToolUse",
			toolName: "mcp__db__exec",
			arguments: { command: "DROP TABLE users" },
		}),
	);
	bench("bumi_complex_miss", () =>
		bumi.evaluate({
			event: "PostToolUse",
			toolName: "mcp__db__exec",
			command: "DROP TABLE users",
		}),
	);
});

// ── Trace overhead (crusty only — bumi doesn't have trace) ───────────────────

summary(() => {
	const crusty = HookMatcher.compile(
		[
			{
				event: "PreToolUse",
				toolName: StringMatch.prefix("mcp__"),
				arguments: [["command", StringMatch.contains("drop")]],
			},
		],
		"blocked",
		"allowed",
	);

	bench("crusty_trace", () =>
		crusty.trace({
			event: "PreToolUse",
			toolName: "mcp__db__exec",
			arguments: { command: "DROP TABLE users" },
		}),
	);

	bench("crusty_evaluate", () =>
		crusty.evaluate({
			event: "PreToolUse",
			toolName: "mcp__db__exec",
			arguments: { command: "DROP TABLE users" },
		}),
	);
});

await run();
