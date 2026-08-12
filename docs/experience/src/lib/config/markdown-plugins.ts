import type { Plugin } from 'svelte-exmarkdown';
import { gfmPlugin } from 'svelte-exmarkdown/gfm';
import rehypeSlug from 'rehype-slug';
import rehypeRaw from 'rehype-raw';
import rehypeShikiFromHighlighter from '@shikijs/rehype/core';
import { createHighlighter } from 'shiki';
import Matcher from '$lib/components/Matcher.svelte';

/**
 * Syntax highlighting.
 *
 * Uses the `shiki` bundle deliberately. Switching to `shiki/core` with static
 * theme and grammar imports was tried and measured worse: the bundle
 * lazy-loads grammars via dynamic import, so pinning them statically made them
 * eager and grew a docs page from 260 kB to 306 kB gzipped.
 *
 * The real cost is architectural, not configuration. svelte-exmarkdown renders
 * markdown at runtime, so the highlighter reaches the browser at all, even
 * though every page is prerendered and the code is already highlighted in the
 * HTML. Fixing it properly means rendering markdown to HTML at build time and
 * hydrating only the <matcher> islands. Worth doing, but not by guessing at
 * bundler flags.
 */
const highlighter = await createHighlighter({
	themes: ['github-dark'],
	langs: ['rust', 'python', 'typescript', 'javascript', 'json', 'yaml', 'bash', 'toml', 'protobuf']
});

export const plugins: Plugin[] = [
	gfmPlugin(),
	// rehype-raw parses literal HTML in Markdown, which is what lets an author
	// write <matcher .../> in a .md file and have it become a Svelte component.
	{ rehypePlugin: [rehypeRaw] },
	{ rehypePlugin: [rehypeSlug] },
	{ rehypePlugin: [rehypeShikiFromHighlighter, highlighter, { theme: 'github-dark' }] },
	// The seam: a custom tag renders as a live, editable matcher.
	{ renderer: { matcher: Matcher } }
];
