import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	// wasm-pack's glue locates its .wasm relative to import.meta.url, which
	// pre-bundling rewrites. The engine module passes the URL explicitly, and
	// leaving the package out of the pre-bundle keeps the fallback honest too.
	optimizeDeps: { exclude: ['xuma-crust'] },
	server: { port: 6200 }
});
