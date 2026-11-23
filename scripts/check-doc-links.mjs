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
 * The docs build reports at most a prefetch 404 for a dead link, and nothing
 * fails, so this check does.
 *
 * Two link syntaxes, both checked: the `.md` one and the absolute `](/anything)`
 * one. A gate that silently declines to look at a whole syntax is worse than no
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
 * Deliberately does **not** include `/api/rust`. Rustdoc output is assembled
 * into the site after the SvelteKit build, so a bare `](/api/rust)` fails
 * prerender — and it would be wrong in production anyway: the site is served
 * under a base path (`/x.uma` on Pages), which a hand-written absolute link
 * ignores. Link the full URL instead. Leaving the root undeclared is
 * what keeps both failure modes caught.
 */
const STATIC_ROOTS = new Set(["/", "/docs", "/playground"]);

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
