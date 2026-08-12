<script lang="ts">
	import { base } from '$app/paths';
	import { QUADRANTS, byQuadrant, type Quadrant } from '$lib/data/docs.js';

	const ORDER: Quadrant[] = ['tutorial', 'how-to', 'reference', 'explanation'];
</script>

<svelte:head>
	<title>Docs — x.uma</title>
</svelte:head>

<div class="index">
	<h1>Documentation</h1>
	<p class="lede">
		Organised by what you came to do. Start with a tutorial if x.uma is new to you, reach for a
		how-to when you have a specific problem, and read the explanations when you want to know why it
		works the way it does.
	</p>

	{#each ORDER as kind (kind)}
		{@const entries = byQuadrant(kind)}
		<section style="--quadrant: {QUADRANTS[kind].token}">
			<h2>{QUADRANTS[kind].label}</h2>
			<p class="blurb">{QUADRANTS[kind].blurb}</p>
			<ul>
				{#each entries as doc (doc.slug)}
					<li>
						<a href="{base}/docs/{doc.slug}">{doc.title}</a>
						<span>{doc.description}</span>
					</li>
				{/each}
			</ul>
		</section>
	{/each}
</div>

<style>
	.index {
		max-width: var(--content-wide);
		margin-inline: auto;
		padding: var(--space-4) var(--space-3) var(--space-6);
	}

	.lede {
		max-width: var(--content-width);
		color: var(--dao-text-secondary);
	}

	section {
		margin-block-start: var(--space-4);
		padding-inline-start: var(--space-2);
		border-inline-start: var(--border-accent) solid var(--quadrant);
	}

	section h2 {
		margin-block: 0 2px;
		font-size: var(--type-lg);
	}

	.blurb {
		margin-block-end: var(--space-2);
		font-size: var(--type-sm);
		color: var(--dao-muted);
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	li {
		display: flex;
		flex-direction: column;
		gap: 1px;
		margin-block-end: var(--space-2);
	}

	li span {
		font-size: var(--type-sm);
		color: var(--dao-muted);
	}
</style>
