<script lang="ts">
	import '../app.css';
	import { base } from '$app/paths';
	import { page } from '$app/state';
	import { DOCS, QUADRANTS, type Quadrant } from '$lib/data/docs.js';

	let { children } = $props();

	const ORDER: Quadrant[] = ['tutorial', 'how-to', 'reference', 'explanation'];
	const onDocs = $derived(page.url.pathname.includes('/docs'));
</script>

<div class="shell">
	<header>
		<a class="wordmark" href="{base}/">
			x<span>.</span>uma
		</a>
		<nav>
			<a href="{base}/docs">Docs</a>
			<a href="{base}/playground">Playground</a>
			<a href="https://github.com/mox-nexus/x.uma">GitHub</a>
		</nav>
	</header>

	<div class="body" class:with-sidebar={onDocs}>
		{#if onDocs}
			<aside>
				{#each ORDER as kind (kind)}
					<section>
						<h2 style="--quadrant: {QUADRANTS[kind].token}">{QUADRANTS[kind].label}</h2>
						<ul>
							{#each DOCS.filter((d) => d.kind === kind) as doc (doc.slug)}
								<li>
									<a
										href="{base}/docs/{doc.slug}"
										aria-current={page.url.pathname.endsWith(`/docs/${doc.slug}`)
											? 'page'
											: undefined}>{doc.title}</a
									>
								</li>
							{/each}
						</ul>
					</section>
				{/each}
			</aside>
		{/if}

		<main>
			{@render children()}
		</main>
	</div>

	<footer>
		<span>MIT OR Apache-2.0</span>
		<span>Alpha 0.0.2 — not yet published to registries</span>
	</footer>
</div>

<style>
	.shell {
		display: flex;
		flex-direction: column;
		min-height: 100vh;
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		height: var(--header-height);
		padding-inline: var(--space-3);
		border-block-end: var(--border-width) solid var(--dao-border);
		position: sticky;
		top: 0;
		z-index: var(--z-sticky);
		background: var(--dao-bg);
	}

	.wordmark {
		font-family: var(--font-sans);
		font-size: var(--type-lg);
		font-weight: var(--weight-semibold);
		letter-spacing: var(--tracking-tight);
		color: var(--dao-text);
		border: none;
	}

	.wordmark span {
		color: var(--ci-blue);
	}

	header nav {
		display: flex;
		gap: var(--space-2);
		font-size: var(--type-sm);
	}

	.body {
		flex: 1;
		display: block;
	}

	.body.with-sidebar {
		display: grid;
		grid-template-columns: var(--sidebar-width) minmax(0, 1fr);
	}

	@media (max-width: 860px) {
		.body.with-sidebar {
			grid-template-columns: 1fr;
		}
		aside {
			display: none;
		}
	}

	aside {
		padding: var(--space-3) var(--space-2);
		border-inline-end: var(--border-width) solid var(--dao-border);
	}

	aside section {
		margin-block-end: var(--space-3);
	}

	aside h2 {
		margin: 0 0 var(--space-1);
		font-family: var(--font-mono);
		font-size: var(--type-xs);
		font-weight: var(--weight-medium);
		text-transform: uppercase;
		letter-spacing: var(--tracking-wider);
		color: var(--dao-muted);
		padding-inline-start: var(--space-1);
		border-inline-start: var(--border-accent) solid var(--quadrant);
	}

	aside ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	aside li {
		margin: 0;
	}

	aside a {
		display: block;
		padding: 3px var(--space-1);
		font-size: var(--type-sm);
		color: var(--dao-text-secondary);
		border: none;
	}

	aside a:hover,
	aside a[aria-current='page'] {
		color: var(--dao-text);
		background: var(--glyph-blue);
	}

	main {
		min-width: 0;
	}

	footer {
		display: flex;
		justify-content: space-between;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border-block-start: var(--border-width) solid var(--dao-border);
		font-size: var(--type-xs);
		color: var(--dao-muted);
	}
</style>
