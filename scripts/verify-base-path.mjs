#!/usr/bin/env node
/**
 * Fail the build when a static site's asset URLs do not match the base path it
 * was built for.
 *
 * SvelteKit emits absolute asset URLs. Built with the wrong `paths.base`, the
 * site loads locally and serves a blank page with /_app/* 404s once deployed
 * under a prefix. Nothing about the artifact looks wrong, so the failure is
 * only visible in production.
 *
 * This turns that into a loud build failure instead of a silent deploy.
 *
 * Usage: node scripts/verify-base-path.mjs <build-dir>
 *        BASE_PATH is read from the environment, matching svelte.config.js.
 */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

const buildDir = process.argv[2];
if (!buildDir) {
	console.error('usage: verify-base-path.mjs <build-dir>');
	process.exit(2);
}

const base = process.env.BASE_PATH ?? '';
const expected = `${base}/_app/`;

/** Entry documents are the only place SvelteKit writes absolute asset URLs. */
function entryDocuments(dir) {
	const found = [];
	for (const name of readdirSync(dir)) {
		const path = join(dir, name);
		if (statSync(path).isDirectory()) {
			found.push(...entryDocuments(path));
		} else if (name.endsWith('.html')) {
			found.push(path);
		}
	}
	return found;
}

const docs = entryDocuments(buildDir);
if (docs.length === 0) {
	console.error(`✗ base-path check: no .html files under ${buildDir}`);
	process.exit(1);
}

/**
 * Two shapes are legitimate, depending on how the app was adapted:
 *
 *   ./_app/...   relative. Fully prerendered pages use these, and they resolve
 *                against the page's own URL, so the base path cannot break them.
 *   /_app/...    absolute. An SPA fallback document has no stable directory to
 *                resolve against, so SvelteKit emits absolute URLs. These are
 *                the ones that 404 when the base is wrong.
 *
 * Only absolute references are checked. Finding neither is itself a failure:
 * a check that quietly passes on an unrecognised output shape is worse than no
 * check, because it reads as a guarantee.
 */
const ASSET_REF = /["'](\.{0,2}\/[^"']*?_app\/[^"']*)["']/g;
const offenders = [];
let absolute = 0;
let relative = 0;

for (const doc of docs) {
	const html = readFileSync(doc, 'utf8');
	for (const [, url] of html.matchAll(ASSET_REF)) {
		if (url.startsWith('./') || url.startsWith('../')) {
			relative++;
		} else {
			absolute++;
			if (!url.startsWith(expected)) offenders.push({ doc, url });
		}
	}
}

const checked = absolute + relative;

if (checked === 0) {
	console.error(`✗ base-path check: found no _app/ asset references in ${docs.length} documents.`);
	console.error('  Either the build is empty or the output shape changed. Not passing silently.');
	process.exit(1);
}

if (offenders.length > 0) {
	const label = base === '' ? '(unset)' : base;
	console.error(`✗ base-path check FAILED for ${buildDir}`);
	console.error(`  BASE_PATH is ${label}, so every asset URL must start with "${expected}".`);
	console.error(`  ${offenders.length} of ${absolute} absolute references do not:`);
	for (const { doc, url } of offenders.slice(0, 5)) {
		console.error(`    ${doc}: ${url}`);
	}
	console.error('\n  Deployed under a prefix this renders a blank page with 404s.');
	console.error('  Set BASE_PATH to the path the site is served from, then rebuild.');
	process.exit(1);
}

const detail =
	absolute > 0
		? `${absolute} absolute refs match "${expected}"` +
			(relative > 0 ? `, ${relative} relative refs are base-independent` : '')
		: `${relative} relative refs, all base-independent`;

console.log(`✓ base-path: ${detail} across ${docs.length} documents`);
