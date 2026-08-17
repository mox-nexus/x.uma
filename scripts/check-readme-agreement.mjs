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
 * file, evaluates a fixed set of requests in Rust, Python and TypeScript, and
 * fails on any disagreement.
 */

import { readFileSync, writeFileSync, mkdtempSync } from "node:fs";
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

const dir = mkdtempSync(join(tmpdir(), "xuma-readme-"));
const configPath = join(dir, "routes.yaml");
writeFileSync(configPath, extractConfig());

const PY = `
import sys, yaml
from xuma import RegistryBuilder, parse_matcher_config
from xuma.http import HttpRequest, register
cfg = parse_matcher_config(yaml.safe_load(open(sys.argv[1])))
matcher = register(RegistryBuilder()).build().load_matcher(cfg)
for method, path in [(a, b) for a, b in (x.split(" ") for x in sys.argv[2:])]:
    r = matcher.evaluate(HttpRequest(method=method, raw_path=path))
    print(r if r is not None else "(none)")
`;

const TS = `
import { parse } from "yaml";
import { readFileSync } from "node:fs";
import { RegistryBuilder, parseMatcherConfig } from "__ROOT__/bumi/src/index.ts";
import { HttpRequest, register } from "__ROOT__/bumi/src/http/index.ts";
const cfg = parseMatcherConfig(parse(readFileSync(process.argv[2], "utf8")));
const matcher = register(new RegistryBuilder()).build().loadMatcher(cfg);
for (const spec of process.argv.slice(3)) {
  const [method, path] = spec.split(" ");
  const r = matcher.evaluate(new HttpRequest(method, path));
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

const pyFile = join(dir, "run.py");
writeFileSync(pyFile, PY);
const python = run("python", "uv", ["run", "--project", "puma", "python", pyFile, configPath, ...specs])
	.split("\n")
	.map((s) => s.trim())
	.filter(Boolean);

const tsFile = join(dir, "run.ts");
writeFileSync(tsFile, TS.replaceAll("__ROOT__", ROOT));
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
