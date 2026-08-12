<script lang="ts">
  import type { EvalResult } from "../types.js";

  let { result }: { result: EvalResult | null } = $props();
</script>

<div class="result-area">
  <div class="label">Result</div>
  {#if result === null}
    <div class="badge idle">Press Evaluate or edit config</div>
  {:else if result.kind === "match"}
    <div class="badge match">
      <span class="icon">&#10003;</span>
      MATCH
      <code class="action">{result.action}</code>
    </div>
  {:else if result.kind === "no-match"}
    <div class="badge no-match">
      <span class="icon">&mdash;</span>
      NO MATCH
    </div>
  {:else if result.kind === "error"}
    <div class="badge error">
      <span class="icon">!</span>
      {result.message}
    </div>
    {#if result.detail}
      <pre class="error-detail">{result.detail}</pre>
    {/if}
  {/if}
</div>

<style>
  .result-area {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 13px;
    font-weight: 600;
  }

  .icon {
    font-size: 15px;
    flex-shrink: 0;
  }

  .idle {
    background: var(--dao-surface-2);
    color: var(--dao-muted);
  }

  .match {
    background: color-mix(in srgb, var(--ci-green) 15%, transparent);
    color: var(--ci-green);
    border: 1px solid color-mix(in srgb, var(--ci-green) 30%, transparent);
  }

  .no-match {
    background: color-mix(in srgb, var(--ev-weak) 12%, transparent);
    color: var(--ev-weak);
    border: 1px solid color-mix(in srgb, var(--ev-weak) 25%, transparent);
  }

  .error {
    background: color-mix(in srgb, var(--ci-red) 12%, transparent);
    color: var(--ci-red);
    border: 1px solid color-mix(in srgb, var(--ci-red) 30%, transparent);
  }

  .action {
    background: color-mix(in srgb, var(--ci-green) 10%, transparent);
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: 400;
  }

  .error-detail {
    background: var(--dao-bg);
    border: 1px solid var(--dao-border);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--dao-muted);
    white-space: pre-wrap;
    word-break: break-word;
    overflow-x: auto;
  }
</style>
