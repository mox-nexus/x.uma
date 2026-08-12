import { error } from '@sveltejs/kit';
import { getEntry, getNavigation } from './docs.js';

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

	return { content, entry, ...getNavigation(slug) };
}
