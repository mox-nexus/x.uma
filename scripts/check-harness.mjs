#!/usr/bin/env node
/**
 * Keep the Claude Code harness (.claude/ and CLAUDE.md) true to the tree it describes.
 *
 * Until 2026-11-23 the project skills described five implementations after two had been
 * deleted, named eight `just` recipes that did not exist, and pinned claims to file:line
 * anchors that had drifted. Prose cannot fail a build, so this does, on four things:
 *
 *   1. a retired name (puma, bumi, r-umi) in a skill, agent, command or script;
 *   2. a file:line anchor in a skill, which goes stale with the next edit to that file;
 *   3. a `just <recipe>` that the justfile does not define;
 *   4. a repository path in backticks that does not exist.
 *
 * `--self-test` runs each check against text built to fail it, so a check that silently
 * matches nothing is caught too.
 */

import { readFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { join, relative } from "node:path";
import { spawnSync } from "node:child_process";

const ROOT = new URL("..", import.meta.url).pathname.replace(/\/$/, "");

const RETIRED = /\b(puma|bumi|r-umi)\b/i;
const ANCHOR = /\b[\w./-]+\.(rs|toml|py|ts|mjs|js|ya?ml|proto|json|sh):\d+/;
const JUST = /`(?:set -o pipefail; )?just ([a-z][a-z0-9-]*)/g;
const PATH = /`((?:rumi|proto|spec|docs|scripts|bench|reference|\.claude)\/[^`\s<>*{}]*)`/g;

function files(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    return statSync(p).isDirectory() ? files(p) : [p];
  });
}

// Read from the justfile itself, so CI needs no `just` binary. `--compare` checks the parse
// against `just --summary` where just is installed.
function recipes() {
  const names = new Set();
  for (const line of readFileSync(join(ROOT, "justfile"), "utf8").split("\n")) {
    const recipe = line.match(/^@?([a-z][a-z0-9_-]*)(?:\s[^:]*)?:(?!=)/);
    const alias = line.match(/^alias\s+([a-z][a-z0-9_-]*)\s*:=/);
    if (recipe) names.add(recipe[1]);
    if (alias) names.add(alias[1]);
  }
  return names;
}

if (process.argv.includes("--compare")) {
  const out = spawnSync("just", ["--summary"], { cwd: ROOT, encoding: "utf8" });
  const fromJust = new Set(out.stdout.trim().split(/\s+/));
  const parsed = recipes();
  const missing = [...fromJust].filter((r) => !parsed.has(r));
  const extra = [...parsed].filter((r) => !fromJust.has(r));
  console.log(`just: ${fromJust.size}, parsed: ${parsed.size}, missing: [${missing}], extra: [${extra}]`);
  process.exit(missing.length || extra.length ? 1 : 0);
}

function check(name, text, { isSkill, retiredApplies }, known, pathExists) {
  const problems = [];
  text.split("\n").forEach((line, i) => {
    const at = `${name}:${i + 1}`;
    if (retiredApplies && RETIRED.test(line)) problems.push(`${at}: retired name: ${line.trim()}`);
    if (isSkill && ANCHOR.test(line)) problems.push(`${at}: file:line anchor: ${line.match(ANCHOR)[0]}`);
    for (const m of line.matchAll(JUST)) {
      if (!known.has(m[1])) problems.push(`${at}: no such recipe: just ${m[1]}`);
    }
    for (const m of line.matchAll(PATH)) {
      if (!pathExists(m[1])) problems.push(`${at}: no such path: ${m[1]}`);
    }
  });
  return problems;
}

function selfTest() {
  const known = new Set(["ci", "gen"]);
  const exists = (p) => p === "rumi/core/";
  const cases = [
    ["a retired name", "use the puma engine", { isSkill: true, retiredApplies: true }],
    ["an anchor", "see `matcher.rs:64`", { isSkill: true, retiredApplies: true }],
    ["a missing recipe", "run `just check-no-std`", { isSkill: false, retiredApplies: false }],
    ["a missing path", "lives in `rumi/src/adapters/`", { isSkill: false, retiredApplies: false }],
  ];
  let failed = false;
  for (const [what, text, opts] of cases) {
    if (check("t", text, opts, known, exists).length === 0) {
      console.error(`self-test: ${what} was not caught`);
      failed = true;
    }
  }
  const clean = "run `just ci`, then `just gen`; code is in `rumi/core/`";
  if (check("t", clean, { isSkill: true, retiredApplies: true }, known, exists).length !== 0) {
    console.error("self-test: clean text was flagged");
    failed = true;
  }
  console.log(failed ? "self-test FAILED" : "self-test passed: every check fires, clean text passes");
  process.exit(failed ? 1 : 0);
}

if (process.argv.includes("--self-test")) selfTest();

const known = recipes();
const pathExists = (p) => existsSync(join(ROOT, p));
const targets = [
  ...files(join(ROOT, ".claude/skills")).map((p) => [p, { isSkill: true, retiredApplies: true }]),
  ...["agents", "commands", "scripts"].flatMap((d) =>
    files(join(ROOT, ".claude", d)).map((p) => [p, { isSkill: false, retiredApplies: true }]),
  ),
  [join(ROOT, "CLAUDE.md"), { isSkill: false, retiredApplies: false }],
].filter(([p]) => /\.(md|sh)$/.test(p));

const problems = targets.flatMap(([p, opts]) =>
  check(relative(ROOT, p), readFileSync(p, "utf8"), opts, known, pathExists),
);

if (problems.length) {
  console.error(problems.join("\n"));
  console.error(`\n${problems.length} problem(s) in the harness.`);
  process.exit(1);
}
console.log(`harness: ${targets.length} files checked against ${known.size} recipes`);
