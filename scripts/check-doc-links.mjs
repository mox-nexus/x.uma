#!/usr/bin/env node
/**
 * Assert every internal Markdown link in docs/content points at a page the site
 * actually serves.
 *
 * The site flattens `content/<category>/<page>.md` to `/docs/<slug>`. Links are
 * written the portable Markdown way — `../category/page.md` — and rewritten at
 * render time by `load-content.ts`. That rewrite is a lookup in the docs
 * manifest, and a link whose target is not in the manifest is silently left as
 * a dead `.md` href.
 *
 * Before 2026-08-17 every internal link in the docs was dead: they used a
 * `<category>/<page>` structure the site does not have. The docs build reported
 * exactly one of them, as a prefetch 404, and nothing failed.
 *
 * PLAN.md A4 / CI4.
 *
 * Two link syntaxes, both checked. Only the `.md` one was, until 2026-08-31,
 * while the summary line read "all internal links resolve to routable pages" —
 * an absolute `](/anything)` was not merely unresolved, it was not counted.
 * Proved by injecting `](/reference/nope)` and watching the count stay at 25.
 * A gate that silently declines to look at a whole syntax is worse than no
 * gate, because the summary reads like coverage.
 */

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative, posix } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname.replace(/\/$/, "");
const CONTENT = join(ROOT, "docs/content");
const MANIFEST = join(ROOT, "docs/experience/src/lib/data/docs.ts");

/** Every `file:` declared in the docs manifest — the set of routable pages. */
function manifestFiles() {
	const src = readFileSync(MANIFEST, "utf8");
	const files = new Set();
	for (const m of src.matchAll(/file:\s*'([^']+)'/g)) files.add(m[1]);
	if (files.size === 0) {
		console.error("check-doc-links: parsed no `file:` entries — the manifest format changed");
		process.exit(1);
	}
	return files;
}

function markdownFiles(dir) {
	const out = [];
	for (const entry of readdirSync(dir)) {
		const abs = join(dir, entry);
		if (statSync(abs).isDirectory()) out.push(...markdownFiles(abs));
		else if (entry.endsWith(".md")) out.push(abs);
	}
	return out;
}

/** Every `slug:` in the manifest — the set of `/docs/<slug>` URLs. */
function manifestSlugs() {
	const src = readFileSync(MANIFEST, "utf8");
	const slugs = new Set();
	for (const m of src.matchAll(/slug:\s*'([^']+)'/g)) slugs.add(`/docs/${m[1]}`);
	if (slugs.size === 0) {
		console.error("check-doc-links: parsed no `slug:` entries — the manifest format changed");
		process.exit(1);
	}
	return slugs;
}

/**
 * Absolute paths the site serves that are not manifest pages.
 *
 * `/api/rust` is rustdoc output, copied into the build by `docs.yml`. It has no
 * SvelteKit route and never appears in the manifest, so it has to be declared —
 * and declaring it here is the point: an absolute link to anything *not* on this
 * list or in the manifest is now a failure rather than a silent pass.
 */
const STATIC_ROOTS = new Set(["/", "/docs", "/playground", "/api/rust"]);

const files = manifestFiles();
const slugs = manifestSlugs();
const problems = [];
let checked = 0;

for (const file of markdownFiles(CONTENT)) {
	const text = readFileSync(file, "utf8");
	const from = relative(CONTENT, file).replace(/\.md$/, "");
	const dir = posix.dirname(from);

	for (const m of text.matchAll(/\]\((?!https?:)((?:\.{1,2}\/)?[A-Za-z0-9._/-]+?\.md)(#[^)]*)?\)/g)) {
		checked += 1;
		const target = posix.normalize(posix.join(dir, m[1])).replace(/\.md$/, "");
		if (!files.has(target)) {
			const line = text.slice(0, m.index).split("\n").length;
			problems.push(
				`${relative(ROOT, file)}:${line}: '${m[1]}' resolves to '${target}', which is not in the docs manifest`,
			);
		}
	}

	// Absolute site links: `](/docs/some-slug)`, `](/api/rust)`.
	for (const m of text.matchAll(/\]\((\/[A-Za-z0-9._/-]*)(#[^)]*)?\)/g)) {
		checked += 1;
		const href = m[1].replace(/\/$/, "") || "/";
		if (!slugs.has(href) && !STATIC_ROOTS.has(href)) {
			const line = text.slice(0, m.index).split("\n").length;
			problems.push(
				`${relative(ROOT, file)}:${line}: '${m[1]}' is not a manifest slug or a declared static root`,
			);
		}
	}
}

if (checked === 0) {
	console.error("check-doc-links: found no internal links at all — the extractor is broken");
	process.exit(1);
}

if (problems.length > 0) {
	console.error(`check-doc-links: ${problems.length} dead link(s) of ${checked}\n`);
	for (const p of problems) console.error(`  ${p}`);
	console.error("\nEither fix the path or add the page to docs/experience/src/lib/data/docs.ts");
	process.exit(1);
}

console.log(`check-doc-links: ${checked} internal links, all resolve to routable pages`);
