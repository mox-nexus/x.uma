import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		// GitHub Pages serves the site under /x.uma. Local dev serves at /.
		paths: { base: process.env.BASE_PATH || '' },
		adapter: adapter({
			pages: 'build',
			assets: 'build',
			fallback: undefined,
			precompress: false,
			strict: true
		}),
		prerender: {
			handleHttpError({ path, referrer, message }) {
				// Content is plain Markdown and cross-references other .md files,
				// which are not routes. Warn instead of failing the build.
				if (path.endsWith('.md')) {
					console.warn(`[prerender] ignoring .md link: ${path} (from ${referrer})`);
					return;
				}
				if (message.includes('does not begin with `base`')) {
					console.warn(`[prerender] ignoring out-of-base link: ${path} (from ${referrer})`);
					return;
				}
				throw new Error(message);
			},
			handleMissingId: 'warn'
		},
		alias: { $content: '../content' }
	}
};

export default config;
