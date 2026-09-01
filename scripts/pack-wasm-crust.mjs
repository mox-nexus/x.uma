#!/usr/bin/env node
/**
 * Prepare `rumi/crusts/wasm/pkg` for npm, then assert the tarball is right.
 *
 * # Why this exists
 *
 * Two defects, both invisible until you look inside the tarball.
 *
 * `wasm-pack` writes its own `package.json` with a `files` array listing three
 * build outputs. npm always adds `README.md` and `package.json` on top of that
 * list — but not `LICENSE-MIT` / `LICENSE-APACHE`, because npm's implicit rule
 * matches `LICENSE`, not a suffixed variant. wasm-pack copies both into `pkg/`,
 * so they are present and still absent from what ships: a package declaring
 * `MIT OR Apache-2.0` with neither licence in it. Verified with `npm pack
 * --dry-run` on 2026-09-01: 5 files, no licence.
 *
 * And nothing published this package at all. Two documents told readers to
 * `bun add xuma-crust` while `release-crust.yml` built only the PyO3 wheel
 * (PLAN.md E8).
 *
 * # Why it verifies rather than trusting
 *
 * The patch is three lines; the check is the rest of the file. A publish is
 * irreversible — a version, once on npm, cannot be reused — so the interesting
 * question is not "did the patch run" but "is the artifact right". The manifest
 * below is the contract, and an unexpected file fails just as loudly as a
 * missing one: a stray `.tgz` or a `snippets/` directory shipping silently is
 * how a package grows things nobody meant to publish.
 */

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname.replace(/\/$/, "");
const PKG = join(ROOT, "rumi/crusts/wasm/pkg");

/** Exactly what the tarball must contain. Not a minimum — the whole set. */
const EXPECTED = new Set([
	"package.json",
	"README.md",
	"LICENSE-MIT",
	"LICENSE-APACHE",
	"xuma_crust.js",
	"xuma_crust.d.ts",
	"xuma_crust_bg.wasm",
]);

if (!existsSync(join(PKG, "package.json"))) {
	console.error(`pack-wasm-crust: ${PKG} has no package.json — run \`wasm-pack build --target web\` first`);
	process.exit(1);
}

const manifestPath = join(PKG, "package.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));

// npm's implicit includes cover README and package.json but not a suffixed
// LICENSE, so the licences are named explicitly.
const licences = ["LICENSE-MIT", "LICENSE-APACHE"];
for (const f of licences) {
	if (!existsSync(join(PKG, f))) {
		console.error(`pack-wasm-crust: ${f} is not in pkg/ — wasm-pack copies it from the crate root; is it missing there?`);
		process.exit(1);
	}
	if (!manifest.files.includes(f)) manifest.files.push(f);
}
writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

// The version is the workspace version, carried through by wasm-pack. Checked
// because publishing the wrong one cannot be undone.
const cargo = readFileSync(join(ROOT, "rumi/Cargo.toml"), "utf8");
const wanted = /^version\s*=\s*"([^"]+)"/m.exec(cargo)?.[1];
if (manifest.version !== wanted) {
	console.error(`pack-wasm-crust: pkg version ${manifest.version} does not match the workspace's ${wanted}`);
	process.exit(1);
}

/** What npm would actually ship, straight from `npm pack --dry-run`. */
const out = execFileSync("npm", ["pack", "--dry-run", "--json"], {
	cwd: PKG,
	encoding: "utf8",
	maxBuffer: 32 * 1024 * 1024,
});
const shipped = new Set(JSON.parse(out)[0].files.map((f) => f.path));

const missing = [...EXPECTED].filter((f) => !shipped.has(f));
const extra = [...shipped].filter((f) => !EXPECTED.has(f));

if (missing.length || extra.length) {
	for (const f of missing) console.error(`  MISSING  ${f}`);
	for (const f of extra) console.error(`  UNEXPECTED  ${f}`);
	console.error(
		"\npack-wasm-crust: the tarball is not what this package promises.\n" +
			"A published version can never be re-uploaded, so this is a hard failure.",
	);
	process.exit(1);
}

console.log(`pack-wasm-crust: ${manifest.name}@${manifest.version}, ${shipped.size} files, licences included`);
