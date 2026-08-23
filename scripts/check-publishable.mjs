#!/usr/bin/env node
/**
 * Assert no publishable crate depends on a `publish = false` crate.
 *
 * `cargo publish` resolves optional dependencies too, so feature-gating does
 * not help: a single such edge makes the crate unpublishable, and you find out
 * when the release workflow is already half done and crates.io versions can
 * never be re-uploaded.
 *
 * This is the part of M6 that IS verifiable before publishing. The rest is not:
 * `cargo publish --dry-run` runs a verification build that resolves from
 * crates.io, so crate N+1 cannot be dry-run until crate N is actually live.
 * Checking the edges, the metadata and the ordering is what can be done ahead
 * of time; the chain itself is only provable at publish time.
 *
 * PLAN.md E1, E2, E3, F9, F10.
 */

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname.replace(/\/$/, "");

const meta = JSON.parse(
	execFileSync(
		"cargo",
		["metadata", "--no-deps", "--manifest-path", "rumi/Cargo.toml", "--format-version", "1"],
		{ cwd: ROOT, encoding: "utf8", maxBuffer: 32 * 1024 * 1024 },
	),
);

const unpublishable = new Set(
	meta.packages.filter((p) => Array.isArray(p.publish) && p.publish.length === 0).map((p) => p.name),
);
const publishable = meta.packages.filter((p) => !unpublishable.has(p.name));

const problems = [];
const missingMetadata = [];

for (const pkg of publishable) {
	for (const dep of pkg.dependencies) {
		// kind null = a normal dependency; dev and build deps are not published.
		if (dep.kind === null && unpublishable.has(dep.name)) {
			problems.push(`${pkg.name} depends on ${dep.name}, which is publish = false`);
		}
		if (dep.kind === null && dep.path && !dep.req.match(/\d/)) {
			problems.push(`${pkg.name}'s dependency ${dep.name} is a path with no version requirement`);
		}
	}
	for (const field of ["description", "readme", "keywords", "categories", "license", "repository"]) {
		const v = pkg[field];
		if (v === null || v === undefined || (Array.isArray(v) && v.length === 0)) {
			missingMetadata.push(`${pkg.name} has no ${field}`);
		}
	}
}

console.log(`publishable: ${publishable.map((p) => p.name).sort().join(", ")}`);
console.log(`publish = false: ${[...unpublishable].sort().join(", ")}\n`);

if (problems.length || missingMetadata.length) {
	for (const p of problems) console.error(`  BLOCKER  ${p}`);
	for (const m of missingMetadata) console.error(`  metadata ${m}`);
	console.error("\ncheck-publishable: failed");
	process.exit(1);
}

console.log("check-publishable: no unpublishable edges, all metadata present");

// ═══════════════════════════════════════════════════════════════════════════
// Does the release workflow publish all of them, in an order that can succeed?
// ═══════════════════════════════════════════════════════════════════════════
//
// The set above is derived from Cargo. The workflow is a hand-maintained list.
// Nothing linked the two, and they had drifted: `release.yml` published
// rumi-core, rumi-http and rumi-cli, while rumi-proto and rumi-kv — both
// publishable, both dependencies of rumi-cli — were absent. The workflow would
// have published two crates, then failed on the third against a crates.io that
// had never heard of `rumi-kv`, with `rumi-core` and `rumi-http` already
// uploaded at a version that can never be re-used.
//
// Order matters as much as membership: `cargo publish` runs a verification
// build resolving from crates.io, so a crate cannot go up before the crates it
// depends on. A topological violation fails exactly the same way as an omission.

const WORKFLOW = ".github/workflows/release.yml";
const wf = readFileSync(join(ROOT, WORKFLOW), "utf8");

const byManifest = new Map(
	meta.packages.map((p) => [p.manifest_path.slice(ROOT.length + 1), p.name]),
);

// The workflow keeps the order in one place: a `CRATES:` env list on the rust
// job, consumed by both the dry-run loop and the publish loop. Reading that one
// line is what links Cargo's answer to the workflow's.
const listed = wf.match(/^\s*CRATES:\s*(.+)$/m);
if (listed === null) {
	console.error(`  BLOCKER  ${WORKFLOW} has no CRATES: list — this check cannot see what it publishes`);
	console.error("\ncheck-publishable: failed");
	process.exit(1);
}

const order = [];
for (const dir of listed[1].trim().split(/\s+/)) {
	const name = byManifest.get(`${dir}/Cargo.toml`);
	if (name === undefined) {
		problems.push(`${WORKFLOW} lists ${dir}, which is not a workspace member`);
		continue;
	}
	if (unpublishable.has(name)) {
		problems.push(`${WORKFLOW} publishes ${name}, which is publish = false`);
		continue;
	}
	order.push(name);
}

const wanted = publishable.map((p) => p.name);
for (const name of wanted) {
	if (!order.includes(name)) {
		problems.push(`${name} is publishable but ${WORKFLOW} never publishes it`);
	}
}

// Topological order, checked against the same metadata the set came from.
const byName = new Set(wanted);
const internal = new Map(
	publishable.map((p) => [
		p.name,
		p.dependencies.filter((d) => d.kind === null && byName.has(d.name)).map((d) => d.name),
	]),
);
order.forEach((name, i) => {
	for (const dep of internal.get(name) ?? []) {
		const at = order.indexOf(dep);
		if (at === -1) continue; // already reported as missing
		if (at > i) {
			problems.push(
				`${WORKFLOW} publishes ${name} (step ${i + 1}) before its dependency ${dep} (step ${at + 1})`,
			);
		}
	}
});

console.log(`release.yml order: ${order.join(" -> ")}`);

if (problems.length) {
	for (const p of problems) console.error(`  BLOCKER  ${p}`);
	console.error("\ncheck-publishable: failed");
	process.exit(1);
}

console.log("check-publishable: release.yml covers every publishable crate, in dependency order");
