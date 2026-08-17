#!/usr/bin/env node
/**
 * Assert that every `rumi ...` command appearing in the docs is one the binary
 * actually has.
 *
 * This exists because four how-to pages taught `rumi eval`, `rumi validate` and
 * a `--config` flag, none of which ever existed. Prose cannot fail a build, so
 * nothing caught it. PLAN.md A4 / CI4.
 *
 * Scope is deliberately narrow: it checks the *shape* of a command — subcommand
 * and flags — against `rumi --help`, which is the `cli` class in PLAN.md §0.9's
 * taxonomy. It does not execute them; a command that needs a config file on
 * disk is not something a docs check should be inventing.
 *
 * Usage: node scripts/check-doc-commands.mjs [--help-text <file>]
 */

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { spawnSync } from "node:child_process";

const ROOT = new URL("..", import.meta.url).pathname.replace(/\/$/, "");
const TARGETS = ["docs/content", "README.md"];

/** Collect .md files under a path (file or directory). */
function markdownFiles(path) {
	const abs = join(ROOT, path);
	if (statSync(abs).isFile()) return abs.endsWith(".md") ? [abs] : [];
	const out = [];
	for (const entry of readdirSync(abs)) {
		out.push(...markdownFiles(join(path, entry)));
	}
	return out;
}

/** Extract `rumi ...` invocations from fenced bash/shell blocks. */
function rumiCommands(file) {
	const text = readFileSync(file, "utf8");
	const found = [];
	const fence = /^```(bash|sh|shell|console)\n([\s\S]*?)^```/gm;
	let m;
	while ((m = fence.exec(text)) !== null) {
		const blockStart = text.slice(0, m.index).split("\n").length;
		m[2].split("\n").forEach((raw, i) => {
			const line = raw.replace(/^\$\s*/, "").trim();
			if (!line.startsWith("rumi ")) return;
			found.push({ file, line: blockStart + 1 + i, cmd: line });
		});
	}
	return found;
}

/** Parse `rumi --help` into the set of subcommands and flags it advertises. */
function parseHelp(help) {
	const subcommands = new Set(["help"]);
	const flags = new Set();
	for (const line of help.split("\n")) {
		const sub = /^\s{2}([a-z][a-z-]*)\s/.exec(line);
		if (sub) subcommands.add(sub[1]);
		for (const f of line.matchAll(/(--[a-z][a-z-]*)/g)) flags.add(f[1]);
	}
	return { subcommands, flags };
}

const helpIdx = process.argv.indexOf("--help-text");

/** `rumi --help` writes to stderr, so both streams are captured and joined. */
function readHelp() {
	if (helpIdx !== -1) return readFileSync(process.argv[helpIdx + 1], "utf8");
	const res = spawnSync(
		"cargo",
		["run", "-q", "--manifest-path", "rumi/Cargo.toml", "-p", "rumi-cli", "--", "--help"],
		{ cwd: ROOT, encoding: "utf8" },
	);
	const text = `${res.stdout ?? ""}\n${res.stderr ?? ""}`;
	if (!text.includes("Commands:")) {
		console.error("check-doc-commands: could not read `rumi --help`; got:\n" + text.slice(0, 500));
		process.exit(1);
	}
	return text;
}

const help = readHelp();

const { subcommands, flags } = parseHelp(help);
const DOMAINS = new Set(["http", "claude"]);

const problems = [];
let checked = 0;

for (const target of TARGETS) {
	for (const file of markdownFiles(target)) {
		for (const { line, cmd } of rumiCommands(file)) {
			checked += 1;
			const tokens = cmd.split(/\s+/).slice(1);
			const where = `${relative(ROOT, file)}:${line}`;

			const head = tokens[0];
			if (head === undefined) continue;
			if (head.startsWith("-")) {
				if (!flags.has(head) && head !== "-h") {
					problems.push(`${where}: unknown flag '${head}'\n    ${cmd}`);
				}
			} else if (!subcommands.has(head)) {
				problems.push(
					`${where}: '${head}' is not a rumi subcommand (have: ${[...subcommands].sort().join(", ")})\n    ${cmd}`,
				);
			}

			for (const tok of tokens.slice(1)) {
				if (!tok.startsWith("--")) continue;
				const name = tok.split("=")[0];
				if (!flags.has(name)) {
					problems.push(`${where}: unknown flag '${name}'\n    ${cmd}`);
				}
			}

			// A domain word must be one the CLI knows, not e.g. `rumi run gprc ...`
			const second = tokens[1];
			if (second && !second.startsWith("-") && !DOMAINS.has(second) && !second.includes(".")) {
				problems.push(`${where}: '${second}' is neither a domain nor a config path\n    ${cmd}`);
			}
		}
	}
}

if (checked === 0) {
	console.error("check-doc-commands: found no rumi commands at all — the extractor is broken");
	process.exit(1);
}

if (problems.length > 0) {
	console.error(`check-doc-commands: ${problems.length} problem(s) in ${checked} commands\n`);
	for (const p of problems) console.error(`  ${p}\n`);
	process.exit(1);
}

console.log(`check-doc-commands: ${checked} commands, all shapes match \`rumi --help\``);
