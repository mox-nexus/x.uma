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
