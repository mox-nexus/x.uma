<script lang="ts">
	import { base } from '$app/paths';
	import Matcher from '$lib/components/Matcher.svelte';
	import { QUADRANTS, byQuadrant, type Quadrant } from '$lib/data/docs.js';

	const ORDER: Quadrant[] = ['tutorial', 'how-to', 'reference', 'explanation'];

	const DEMO_CONFIG = `{
  "matcherList": {
    "matchers": [
      {
        "predicate": {
          "singlePredicate": {
            "input": {
              "name": "method",
              "typedConfig": { "@type": "type.googleapis.com/xuma.core.v1.MapInput", "key": "method" }
            },
            "valueMatch": { "exact": "GET" }
          }
        },
        "onMatch": {
          "action": {
            "name": "read-handler",
            "typedConfig": { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", "name": "read-handler" }
          }
        }
      },
      {
        "predicate": {
          "singlePredicate": {
            "input": {
              "name": "method",
              "typedConfig": { "@type": "type.googleapis.com/xuma.core.v1.MapInput", "key": "method" }
            },
            "valueMatch": { "exact": "POST" }
          }
        },
        "onMatch": {
          "action": {
            "name": "write-handler",
            "typedConfig": { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", "name": "write-handler" }
          }
        }
      }
    ]
  },
  "onNoMatch": {
    "action": {
      "name": "405-not-allowed",
      "typedConfig": { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", "name": "405-not-allowed" }
    }
  }
}`;

	const DEMO_CONTEXT = `{ "method": "GET" }`;

	const RUNTIMES = [
		{
			lang: 'Rust',
			code: `let matcher = registry.load_typed_matcher(config, &actions)?;
matcher.evaluate(&ctx)   // Some("read-handler")`
		},
		{
			lang: 'Python',
			code: `matcher = KvMatcher.from_config(config)
matcher.evaluate(ctx)    # "read-handler"`
		},
		{
			lang: 'TypeScript',
			code: `const matcher = KvMatcher.fromConfig(config);
matcher.evaluate(ctx);   // "read-handler"`
		}
	];
</script>

<svelte:head>
	<title>x.uma — cross-platform matcher engine</title>
	<meta
		name="description"
		content="Match structured data against rule trees. Write the rules once, evaluate them in Rust, Python, or TypeScript, and get the same answer."
	/>
</svelte:head>

<section class="hero">
	<h1>Write the rules once.<br /><em>Get the same answer everywhere.</em></h1>
	<p>
		x.uma is a matcher engine implementing the xDS Unified Matcher API. One config, one Rust
		engine, three languages, one conformance suite proving they agree.
	</p>
</section>

<section class="demo">
	<h2>It runs here</h2>
	<p class="lede">
		This is the real engine: rumi, compiled to WebAssembly and running in your browser, loading
		the config beside it. Change the method to <code>POST</code>, or to something else entirely,
		and watch the decision change.
	</p>
	<Matcher config={DEMO_CONFIG} context={DEMO_CONTEXT} />
	<p class="aside">
		Rules are tried in order and the first match wins. When nothing matches,
		<code>on_no_match</code> decides. That is the whole evaluation model.
	</p>
</section>

<section class="runtimes">
	<h2>The same config, three runtimes</h2>
	<p class="lede">
		The config above is data, not code. Every runtime hands it to the same engine and gets the
		same answer.
	</p>
	<div class="grid">
		{#each RUNTIMES as r (r.lang)}
			<article>
				<h3>{r.lang}</h3>
				<pre><code>{r.code}</code></pre>
			</article>
		{/each}
	</div>
</section>

<section class="impls">
	<h2>One engine, three runtimes</h2>
	<p class="lede">
		Pick by runtime. Every package is the same Rust engine; the bindings add a host-language
		surface, not a second implementation.
	</p>
	<table>
		<thead>
			<tr><th>Package</th><th>Language</th><th>What it is</th></tr>
		</thead>
		<tbody>
			<tr><td><code>rumi-core</code></td><td>Rust</td><td>The engine. Reference implementation.</td></tr>
			<tr><td><code>xuma-crust</code></td><td>Python 3.12+</td><td>The engine, bound through PyO3.</td></tr>
			<tr><td><code>xuma-crust</code></td><td>TypeScript</td><td>The engine, compiled to WebAssembly.</td></tr>
		</tbody>
	</table>
	<p class="aside">
		All three run the same conformance fixtures from <code>spec/tests/</code>. A binding that
		disagrees with the engine fails its own build.
	</p>
</section>

<section class="pipeline">
	<h2>How a decision is made</h2>
	<pre><code>{`Context  →  DataInput  →  MatchingData  →  InputMatcher  →  bool
            domain-        erased           domain-
            specific                        agnostic`}</code></pre>
	<p class="lede">
		An <code>ExactMatcher</code> does not know whether it is matching an HTTP path, a Claude Code
		hook event, or your own domain. It matches <em>data</em>. Extracting that data from a context
		is a separate port, which is why one matcher works everywhere.
	</p>
</section>

<section class="quadrants">
	<h2>Where to go next</h2>
	<div class="grid">
		{#each ORDER as kind (kind)}
			{@const entries = byQuadrant(kind)}
			<article style="--quadrant: {QUADRANTS[kind].token}">
				<h3>{QUADRANTS[kind].label}</h3>
				<p>{QUADRANTS[kind].blurb}</p>
				<ul>
					{#each entries.slice(0, 4) as doc (doc.slug)}
						<li><a href="{base}/docs/{doc.slug}">{doc.title}</a></li>
					{/each}
				</ul>
			</article>
		{/each}
	</div>
</section>

<style>
	section {
		max-width: var(--content-wide);
		margin-inline: auto;
		padding: var(--space-5) var(--space-3) 0;
	}

	section:last-of-type {
		padding-block-end: var(--space-6);
	}

	.hero h1 {
		font-size: var(--type-3xl);
		line-height: var(--leading-tight);
		margin-block-end: var(--space-2);
	}

	/* The one vivid moment. Documentation body stays on the soft OKLCH layer;
	   the brand layer gets the entrance. See ~/mox/brand/system.md. */
	.hero em {
		font-style: normal;
		color: #4da6ff;
		text-shadow: 0 0 27px oklch(62% 0.15 240 / 0.45);
	}

	.hero p {
		max-width: var(--content-width);
		color: var(--dao-text-secondary);
		font-size: var(--type-lg);
		line-height: var(--leading-relaxed);
	}

	h2 {
		font-size: var(--type-xl);
	}

	.lede,
	.aside {
		max-width: var(--content-width);
		color: var(--dao-text-secondary);
	}

	.aside {
		font-size: var(--type-sm);
		color: var(--dao-muted);
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(270px, 1fr));
		gap: var(--space-2);
	}

	article {
		padding: var(--space-2);
		border: var(--border-width) solid var(--dao-border);
		background: var(--dao-surface);
	}

	article h3 {
		margin-block-start: 0;
		font-size: var(--type-base);
	}

	.runtimes pre {
		margin-block-end: 0;
		border: none;
		background: none;
		padding: 0;
		font-size: var(--type-xs);
	}

	.quadrants article {
		border-block-start: var(--border-accent) solid var(--quadrant);
	}

	.quadrants article p {
		color: var(--dao-muted);
		font-size: var(--type-sm);
	}

	.quadrants ul {
		list-style: none;
		margin: 0;
		padding: 0;
		font-size: var(--type-sm);
	}
</style>
