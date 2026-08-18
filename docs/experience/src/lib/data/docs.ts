/**
 * The documentation manifest.
 *
 * Navigation derives from this list, so a page that is not declared here has no
 * route and no home. `kind` is required, which means a page cannot be added
 * without deciding which Diataxis quadrant it belongs to. That is deliberate:
 * the mox tokens define one colour per quadrant, and an undeclared page would
 * have nothing to render.
 */

/** The four Diataxis quadrants. Tokenized in ~/mox/brand as --quadrant-*. */
export type Quadrant = 'tutorial' | 'how-to' | 'reference' | 'explanation';

export interface DocsEntry {
	/** URL segment under /docs/ */
	slug: string;
	/** Path under docs/content/, without the .md extension */
	file: string;
	title: string;
	/** One line. Shown on cards and in the quadrant index. */
	description: string;
	kind: Quadrant;
	readMinutes?: number;
}

export const QUADRANTS: Record<Quadrant, { label: string; blurb: string; token: string }> = {
	tutorial: {
		label: 'Tutorials',
		blurb: 'Start here. Working code in a few minutes.',
		token: 'var(--quadrant-tutorials)'
	},
	'how-to': {
		label: 'How-to',
		blurb: 'Solve one specific problem.',
		token: 'var(--quadrant-how-to)'
	},
	reference: {
		label: 'Reference',
		blurb: 'Look something up.',
		token: 'var(--quadrant-reference)'
	},
	explanation: {
		label: 'Explanation',
		blurb: 'Understand why it works this way.',
		token: 'var(--quadrant-explanation)'
	}
};

export const DOCS: DocsEntry[] = [
	// ── Tutorials ────────────────────────────────────────────────────────────
	{
		slug: 'rust',
		file: 'getting-started/rust',
		title: 'Rust',
		description: 'Build an HTTP route matcher with rumi-core and rumi-http.',
		kind: 'tutorial',
		readMinutes: 6
	},
	{
		slug: 'python',
		file: 'getting-started/python',
		title: 'Python',
		description: 'The same matcher in pure Python, or Rust-backed via PyO3.',
		kind: 'tutorial',
		readMinutes: 6
	},
	{
		slug: 'typescript',
		file: 'getting-started/typescript',
		title: 'TypeScript',
		description: 'The same matcher in pure TypeScript, or WASM-backed.',
		kind: 'tutorial',
		readMinutes: 6
	},

	// ── How-to ───────────────────────────────────────────────────────────────
	{
		slug: 'route-by-header',
		file: 'how-to/route-by-header',
		title: 'Route on a header',
		description: 'Match a request by header value, with a fallback when nothing matches.',
		kind: 'how-to',
		readMinutes: 4
	},
	{
		slug: 'custom-input',
		file: 'how-to/custom-input',
		title: 'Add a custom input',
		description: 'Teach the engine to read a field it does not know about yet.',
		kind: 'how-to',
		readMinutes: 5
	},
	{
		slug: 'claude-hook',
		file: 'how-to/claude-hook',
		title: 'Gate a Claude Code tool call',
		description: 'From a rule to a hook Claude Code actually runs, including the exit-code contract.',
		kind: 'how-to',
		readMinutes: 5
	},
	{
		slug: 'debug-a-match',
		file: 'how-to/debug-a-match',
		title: 'Debug why something matched',
		description: 'Read the evaluation trace to see the path taken through the tree.',
		kind: 'how-to',
		readMinutes: 4
	},
	{
		slug: 'share-config',
		file: 'how-to/share-config',
		title: 'Share one config across languages',
		description: 'Author rules once and evaluate them identically in three runtimes.',
		kind: 'how-to',
		readMinutes: 4
	},

	// ── Reference ────────────────────────────────────────────────────────────
	{
		slug: 'config',
		file: 'reference/config',
		title: 'Config format',
		description: 'Every field of the matcher config, and what it means.',
		kind: 'reference'
	},
	{
		slug: 'cli',
		file: 'reference/cli',
		title: 'CLI',
		description: 'The rumi binary: evaluate, validate, and trace configs.',
		kind: 'reference'
	},
	{
		slug: 'api',
		file: 'reference/api',
		title: 'API',
		description: 'Generated API documentation per language.',
		kind: 'reference'
	},

	// ── Explanation ──────────────────────────────────────────────────────────
	{
		slug: 'pipeline',
		file: 'concepts/pipeline',
		title: 'The matching pipeline',
		description: 'How a context becomes a decision, one stage at a time.',
		kind: 'explanation',
		readMinutes: 7
	},
	{
		slug: 'type-erasure',
		file: 'concepts/type-erasure',
		title: 'Type erasure and ports',
		description: 'Why erasing at the data level keeps matchers shareable across domains.',
		kind: 'explanation',
		readMinutes: 7
	},
	{
		slug: 'architecture',
		file: 'explain/architecture',
		title: 'Architecture',
		description: 'Hexagonal core, domain adapters, and the extension seam.',
		kind: 'explanation',
		readMinutes: 8
	},
	{
		slug: 'security',
		file: 'performance/security',
		title: 'Security model',
		description: 'ReDoS protection, depth limits, and the resource bounds config load enforces.',
		kind: 'explanation',
		readMinutes: 5
	}
];

const BY_SLUG = new Map(DOCS.map((d) => [d.slug, d]));

export function getEntry(slug: string): DocsEntry | undefined {
	return BY_SLUG.get(slug);
}

export function byQuadrant(kind: Quadrant): DocsEntry[] {
	return DOCS.filter((d) => d.kind === kind);
}

/** Previous and next within the same quadrant, for end-of-page navigation. */
export function getNavigation(slug: string): { prev?: DocsEntry; next?: DocsEntry } {
	const entry = getEntry(slug);
	if (!entry) return {};
	const siblings = byQuadrant(entry.kind);
	const i = siblings.findIndex((d) => d.slug === slug);
	return {
		prev: i > 0 ? siblings[i - 1] : undefined,
		next: i < siblings.length - 1 ? siblings[i + 1] : undefined
	};
}
