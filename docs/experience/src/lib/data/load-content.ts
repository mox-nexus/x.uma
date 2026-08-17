import { error } from '@sveltejs/kit';
import { base } from '$app/paths';
import { DOCS, getEntry, getNavigation } from './docs.js';

/**
 * Load a docs article's raw Markdown.
 *
 * import.meta.glob lets Vite resolve every content file at build time while the
 * slug picks one at runtime. Content stays plain Markdown on disk, with no
 * preprocessor and no framework coupling.
 */
const MODULES = import.meta.glob('../../../../content/**/*.md', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

export function loadDocsContent(slug: string) {
	const entry = getEntry(slug);
	if (!entry) throw error(404, `Unknown doc: ${slug}`);

	const path = `../../../../content/${entry.file}.md`;
	const content = MODULES[path];
	if (!content) {
		throw error(500, `Declared in the manifest but missing on disk: content/${entry.file}.md`);
	}

	return { content: rewriteInternalLinks(content, entry), entry, ...getNavigation(slug) };
}

/**
 * Rewrite `../category/page.md` links to the routes the site actually serves.
 *
 * `content/` is plain Markdown on purpose, so its cross-references are written
 * the way Markdown references work — relative paths with a `.md` suffix, which
 * resolve correctly in a plain viewer and on GitHub. The site flattens those
 * into `/docs/<slug>`, so before 2026-08-17 every one of them 404'd. Rewriting
 * here keeps the Markdown portable and the site correct, instead of trading one
 * for the other.
 *
 * A link naming a file that is not in the manifest is left untouched, and
 * `scripts/check-doc-links.mjs` fails the build for it — silently emitting a
 * dead link is the behaviour being fixed.
 */
function rewriteInternalLinks(markdown: string, entry: { file: string }): string {
	const slugByFile = new Map(DOCS.map((e) => [e.file, e.slug]));

	return markdown.replace(
		/\]\((?!https?:)((?:\.{1,2}\/)?[A-Za-z0-9._/-]+?)\.md(#[^)]*)?\)/g,
		(whole, rawTarget: string, hash: string | undefined) => {
			// Resolve against the page's own directory: links may be written
			// bare (`config.md`), same-dir (`./config.md`) or up (`../x/y.md`).
			const dir = entry.file.includes('/') ? entry.file.slice(0, entry.file.lastIndexOf('/')) : '';
			const joined = rawTarget.startsWith('.') ? `${dir}/${rawTarget}` : `${dir}/${rawTarget}`;
			const target = normalizePath(joined);
			const slug = slugByFile.get(target);
			if (!slug) return whole;
			return `](${base}/docs/${slug}${hash ?? ''})`;
		}
	);
}

/** Collapse `a/b/../c` and `a/./b` without pulling in node:path. */
function normalizePath(path: string): string {
	const out: string[] = [];
	for (const part of path.split('/')) {
		if (part === '' || part === '.') continue;
		if (part === '..') out.pop();
		else out.push(part);
	}
	return out.join('/');
}
