import { loadDocsContent } from '$lib/data/load-content.js';
import { DOCS } from '$lib/data/docs.js';
import type { EntryGenerator, PageLoad } from './$types';

export const prerender = true;

/** Every route comes from the manifest, so nothing renders that is undeclared. */
export const entries: EntryGenerator = () => DOCS.map((d) => ({ slug: d.slug }));

export const load: PageLoad = ({ params }) => loadDocsContent(params.slug);
