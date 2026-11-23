#!/usr/bin/env node
/**
 * Run the README's literal routes.yaml through every shipped runtime and assert
 * they return the same answers.
 *
 * The README says "one config, all runtimes" and then shows three code blocks.
 * That is the project's central correctness claim, and nothing checked it — the
 * conformance suite has its own fixtures, but not this config, and a reader
 * copying from the README is not running the conformance suite.
 *
 * PLAN.md CI3.
 *
 * Extracts the first ```yaml block under the HTTP section, writes it to a temp
 * file, evaluates a fixed set of requests through the rumi CLI, the PyO3
 * binding and the wasm binding, and fails on any disagreement. Both bindings
 * have to be built first: `just crust-py-check` and `just crust-wasm-build`.
 */

import { readFileSync, writeFileSync, mkdtempSync, existsSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";

const ROOT = new URL("..", import.meta.url).pathname.replace(/\/$/, "");

/** The requests to evaluate. Chosen to hit match, partial-match and fallback. */
const CASES = [
	{ method: "GET", path: "/api/users", expect: "api_read" },
	{ method: "GET", path: "/other", expect: "not_found" },
	{ method: "POST", path: "/api/users", expect: "not_found" },
];

function extractConfig() {
	const readme = readFileSync(join(ROOT, "README.md"), "utf8");
	const m = /\*\*routes\.yaml\*\*[^\n]*\n```yaml\n([\s\S]*?)```/.exec(readme);
	if (!m) {
		console.error("check-readme-agreement: could not find the routes.yaml block in README.md");
		process.exit(1);
	}
	return m[1];
}

function run(label, cmd, args, opts = {}) {
	const res = spawnSync(cmd, args, { encoding: "utf8", cwd: ROOT, ...opts });
	if (res.status !== 0) {
		console.error(`check-readme-agreement: ${label} failed (exit ${res.status})`);
		console.error((res.stderr || res.stdout || "").slice(0, 1500));
		process.exit(1);
	}
	return (res.stdout || "").trim();
}

const WASM_PKG = join(ROOT, "rumi/crusts/wasm/pkg/xuma_crust.js");
if (!existsSync(WASM_PKG)) {
	console.error("check-readme-agreement: rumi/crusts/wasm/pkg is not built — run `just crust-wasm-build`");
	process.exit(1);
}

const dir = mkdtempSync(join(tmpdir(), "xuma-readme-"));
const configPath = join(dir, "routes.yaml");
writeFileSync(configPath, extractConfig());

const PY = `
import json, sys, yaml
from xuma_crust import HttpMatcher
with open(sys.argv[1]) as f:
    matcher = HttpMatcher.from_config(json.dumps(yaml.safe_load(f)))
for spec in sys.argv[2:]:
    method, path = spec.split(" ")
    r = matcher.evaluate(method, path)
    print(r if r is not None else "(none)")
`;

const TS = `
import init, { HttpMatcher } from "__WASM_PKG__";
await init();
const config = Bun.YAML.parse(await Bun.file(process.argv[2]).text());
const matcher = HttpMatcher.fromConfig(JSON.stringify(config));
for (const spec of process.argv.slice(3)) {
  const [method, path] = spec.split(" ");
  const r = matcher.evaluate({ method, path });
  console.log(r ?? "(none)");
}
`;

const specs = CASES.map((c) => `${c.method} ${c.path}`);

// Rust — via the shipped CLI, one invocation per case.
const rust = CASES.map((c) => {
	const out = run(
		"rust",
		"cargo",
		[
			"run", "-q", "--manifest-path", "rumi/Cargo.toml", "-p", "rumi-cli", "--",
			"run", "http", configPath, "--method", c.method, "--path", c.path,
		],
	);
	return out.split("\n").pop().trim() || "(none)";
});

// Python — the PyO3 binding, from the project that has it installed.
const pyFile = join(dir, "run.py");
writeFileSync(pyFile, PY);
const python = run("python", "uv", ["run", "--project", "rumi/crusts/python", "python", pyFile, configPath, ...specs])
	.split("\n")
	.map((s) => s.trim())
	.filter(Boolean);

// TypeScript — the wasm binding, from its build output.
const tsFile = join(dir, "run.ts");
writeFileSync(tsFile, TS.replace("__WASM_PKG__", WASM_PKG));
const typescript = run("typescript", "bun", [tsFile, configPath, ...specs])
	.split("\n")
	.map((s) => s.trim())
	.filter(Boolean);

let failed = false;
console.log("README routes.yaml, evaluated in every runtime:\n");
console.log("  case                     expected     rust         python       typescript");
CASES.forEach((c, i) => {
	const got = [rust[i], python[i], typescript[i]];
	const agree = got.every((g) => g === c.expect);
	if (!agree) failed = true;
	const label = `${c.method} ${c.path}`.padEnd(24);
	console.log(
		`  ${agree ? " " : "✗"} ${label} ${c.expect.padEnd(12)} ${got.map((g) => (g ?? "-").padEnd(12)).join(" ")}`,
	);
});

if (failed) {
	console.error("\ncheck-readme-agreement: the runtimes disagree, or disagree with the README");
	process.exit(1);
}
console.log("\ncheck-readme-agreement: all runtimes agree with the README");
