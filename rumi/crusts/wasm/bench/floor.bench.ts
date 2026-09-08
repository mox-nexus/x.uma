/**
 * The boundary's floor: one no-op crossing per shape (D-066).
 *
 * Each row is the least a call of that shape can cost. A real path's number
 * is read against the floor of the shape it uses, not against zero, and a
 * change to the return shape is promoted only if its floor is distinguishable
 * from the current one under the noise gate.
 *
 * Run:
 *   cd rumi/crusts/wasm
 *   wasm-pack build --target web -- --features bench
 *   bun run bench/floor.bench.ts
 */

import { bench, run, summary } from "mitata";

import init, {
	boundaryFloorFlatMap,
	boundaryFloorOptionString,
	boundaryFloorU32,
} from "../pkg/xuma_crust.js";
await init();

// Six fields: the width of a hook context, so the flat-map floor is the map a
// hook evaluate would marshal.
const flat = {
	event: "PreToolUse",
	toolName: "Bash",
	sessionId: "s-1",
	cwd: "/work",
	gitBranch: "main",
	command: "ls",
};

summary(() => {
	bench("floor_u32", () => boundaryFloorU32());
	bench("floor_option_string", () => boundaryFloorOptionString());
	bench("floor_flat_map", () => boundaryFloorFlatMap(flat));
});

await run(process.env.BENCH_JSON ? { format: "json" } : {});
