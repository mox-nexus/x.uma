<script lang="ts">
	import { untrack } from 'svelte';
	import { RegistryBuilder, parseProtojson } from 'xuma';
	import { register as registerTest } from 'xuma/testing';

	/**
	 * A live matcher, embedded in prose.
	 *
	 * Authors write this in plain Markdown, `config` as canonical protojson:
	 *
	 *   <matcher config='{"matcherList":{"matchers":[...]}}' context='{"method":"GET"}' />
	 *
	 * rehype-raw parses the tag, and the renderer map in
	 * $lib/config/markdown-plugins.ts swaps it for this component. The Markdown
	 * stays Markdown: elsewhere the tag is inert, here it runs the real engine.
	 */
	let { config = '', context = '{}' }: { config?: string; context?: string } = $props();

	// Built once. The registry is immutable after build (INV-5).
	const registry = registerTest(new RegistryBuilder()).build();

	// Seeded once, deliberately. The props come from a static tag in Markdown and
	// never change; after mount the textareas own this state. untrack() says that
	// rather than leaving Svelte to warn about it.
	let configText = $state(untrack(() => config.trim()));
	let contextText = $state(untrack(() => context.trim()));

	type Outcome =
		| { kind: 'match'; action: string }
		| { kind: 'no-match' }
		| { kind: 'error'; message: string };

	const outcome = $derived.by((): Outcome => {
		try {
			const parsed = parseProtojson(JSON.parse(configText));
			const matcher = registry.loadMatcher(parsed);
			const ctx = JSON.parse(contextText) as Record<string, string>;
			const result = matcher.evaluate(ctx);
			return result !== null ? { kind: 'match', action: result } : { kind: 'no-match' };
		} catch (e) {
			return { kind: 'error', message: e instanceof Error ? e.message : String(e) };
		}
	});
</script>

<div class="matcher">
	<div class="panes">
		<label>
			<span>config</span>
			<textarea bind:value={configText} spellcheck="false" rows="10"></textarea>
		</label>
		<label>
			<span>context</span>
			<textarea bind:value={contextText} spellcheck="false" rows="10"></textarea>
		</label>
	</div>

	<div class="result" data-kind={outcome.kind}>
		{#if outcome.kind === 'match'}
			<span class="tag">match</span><code>{outcome.action}</code>
		{:else if outcome.kind === 'no-match'}
			<span class="tag">no match</span><code>on_no_match, or no decision</code>
		{:else}
			<span class="tag">error</span><code>{outcome.message}</code>
		{/if}
	</div>

	<p class="hint">Editable. Change either side and the result updates as you type.</p>
</div>

<style>
	.matcher {
		margin-block: var(--space-3);
		border: var(--border-width) solid var(--dao-border);
		background: var(--dao-surface);
	}

	.panes {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--border-width);
		background: var(--dao-border);
	}

	@media (max-width: 720px) {
		.panes {
			grid-template-columns: 1fr;
		}
	}

	label {
		display: flex;
		flex-direction: column;
		background: var(--dao-surface);
	}

	label span {
		padding: var(--space-1);
		font-size: var(--type-xs);
		text-transform: uppercase;
		letter-spacing: var(--tracking-wide);
		color: var(--dao-muted);
		border-block-end: var(--border-width) solid var(--dao-border-subtle);
	}

	textarea {
		flex: 1;
		padding: var(--space-1);
		font-family: var(--font-mono);
		font-size: var(--type-xs);
		line-height: var(--leading-relaxed);
		color: var(--dao-text);
		background: transparent;
		border: none;
		resize: vertical;
	}

	textarea:focus {
		outline: none;
		background: var(--glyph-blue);
	}

	.result {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-1);
		border-block-start: var(--border-width) solid var(--dao-border);
		font-size: var(--type-sm);
	}

	.tag {
		padding: 1px 6px;
		font-size: var(--type-xs);
		text-transform: uppercase;
		letter-spacing: var(--tracking-wide);
		border: var(--border-width) solid currentColor;
	}

	/* Trichrome by role: emergence is the decision reached, constraint the
	   boundary, spark the neutral no-match. Mark, do not fill. */
	.result[data-kind='match'] .tag {
		color: var(--ci-green);
	}
	.result[data-kind='error'] .tag {
		color: var(--ci-red);
	}
	.result[data-kind='no-match'] .tag {
		color: var(--dao-muted);
	}

	.result code {
		color: var(--dao-text-secondary);
	}

	.hint {
		margin: 0;
		padding: 0 var(--space-1) var(--space-1);
		font-size: var(--type-xs);
		color: var(--dao-muted);
	}
</style>
