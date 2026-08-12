<script lang="ts">
	import Markdown from 'svelte-exmarkdown';
	import { base } from '$app/paths';
	import { plugins } from '$lib/config/markdown-plugins.js';
	import { QUADRANTS } from '$lib/data/docs.js';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const quadrant = $derived(QUADRANTS[data.entry.kind]);
</script>

<svelte:head>
	<title>{data.entry.title} — x.uma</title>
	<meta name="description" content={data.entry.description} />
</svelte:head>

<article style="--quadrant: {quadrant.token}">
	<header>
		<span class="kind">{quadrant.label}</span>
		{#if data.entry.readMinutes}
			<span class="meta">{data.entry.readMinutes} min</span>
		{/if}
	</header>

	<div class="prose" data-pagefind-body>
		<Markdown md={data.content} {plugins} />
	</div>

	{#if data.prev || data.next}
		<nav>
			{#if data.prev}
				<a class="prev" href="{base}/docs/{data.prev.slug}">
					<span>Previous</span>{data.prev.title}
				</a>
			{:else}
				<span></span>
			{/if}
			{#if data.next}
				<a class="next" href="{base}/docs/{data.next.slug}">
					<span>Next</span>{data.next.title}
				</a>
			{/if}
		</nav>
	{/if}
</article>

<style>
	article {
		max-width: var(--content-wide);
		margin-inline: auto;
		padding: var(--space-3) var(--space-3) var(--space-6);
	}

	header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-block-end: var(--space-2);
		font-size: var(--type-xs);
		text-transform: uppercase;
		letter-spacing: var(--tracking-wider);
	}

	.kind {
		padding-inline-start: var(--space-1);
		border-inline-start: var(--border-accent) solid var(--quadrant);
		color: var(--dao-text-secondary);
	}

	.meta {
		color: var(--dao-muted);
	}

	nav {
		display: flex;
		justify-content: space-between;
		gap: var(--space-2);
		margin-block-start: var(--space-5);
		padding-block-start: var(--space-2);
		border-block-start: var(--border-width) solid var(--dao-border);
	}

	nav a {
		display: flex;
		flex-direction: column;
		gap: 2px;
		font-size: var(--type-sm);
		border: none;
	}

	nav .next {
		text-align: end;
	}

	nav a span {
		font-size: var(--type-xs);
		text-transform: uppercase;
		letter-spacing: var(--tracking-wide);
		color: var(--dao-muted);
	}
</style>
