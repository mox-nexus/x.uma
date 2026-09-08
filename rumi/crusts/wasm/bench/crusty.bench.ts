/**
 * Boundary cost of the WASM binding on the Claude hook path.
 *
 * Every call here crosses from JS into WASM and back. What the numbers
 * isolate:
 * 1. Compile cost — plain objects in, a compiled matcher out
 * 2. Evaluate cost — a context object read field by field across the
 *    boundary, one matcher walk, an owned action string back
 * 3. Regex — Rust `regex`, linear time, behind the same boundary
 * 4. Trace — the opt-in debugging path, against plain evaluate
 *
 * Run:
 *   cd rumi/crusts/wasm
 *   wasm-pack build --target web
 *   bun run bench/crusty.bench.ts
 */

import { bench, run, summary } from "mitata";

import init, { HookMatcher, StringMatch } from "../pkg/xuma_crust.js";
await init();

// ── Compile ──────────────────────────────────────────────────────────────────

summary(() => {
	bench("crusty_compile_simple", () =>
		HookMatcher.compile([{ event: "PreToolUse" }], "matched"),
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
});

// ── Evaluate: exact match ────────────────────────────────────────────────────

// The context is constructed inside the measured region. That is the workload
// a caller actually has: a request arrives, a context is built, a match is run.
summary(() => {
	const crusty = HookMatcher.compile([{ event: "PreToolUse" }], "matched");

	bench("crusty_exact_hit", () => crusty.evaluate({ event: "PreToolUse" }));
	bench("crusty_exact_miss", () => crusty.evaluate({ event: "PostToolUse" }));
});

// ── Evaluate: regex ──────────────────────────────────────────────────────────

summary(() => {
	const crusty = HookMatcher.compile(
		[{ toolName: StringMatch.regex(String.raw`^mcp__\w+__\w+$`) }],
		"matched",
	);

	bench("crusty_regex_hit", () =>
		crusty.evaluate({ event: "PreToolUse", toolName: "mcp__db__query" }),
	);
	bench("crusty_regex_miss", () =>
		crusty.evaluate({ event: "PreToolUse", toolName: "Write" }),
	);
});

// ── Evaluate: complex multi-field ────────────────────────────────────────────

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

	bench("crusty_complex_hit", () =>
		crusty.evaluate({
			event: "PreToolUse",
			toolName: "mcp__db__exec",
			arguments: { command: "DROP TABLE users" },
		}),
	);
	// The event fails, so the AND short-circuits on its first predicate.
	bench("crusty_complex_miss", () =>
		crusty.evaluate({
			event: "PostToolUse",
			toolName: "mcp__db__exec",
			arguments: { command: "DROP TABLE users" },
		}),
	);
});

// ── Trace ────────────────────────────────────────────────────────────────────

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
