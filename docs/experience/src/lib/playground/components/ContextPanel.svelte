<script lang="ts">
  let {
    context = $bindable({}),
    onchange,
  }: {
    context: Record<string, string>;
    onchange?: (ctx: Record<string, string>) => void;
  } = $props();

  let pairs = $state(toPairs(context));

  function toPairs(
    ctx: Record<string, string>,
  ): Array<{ key: string; value: string }> {
    const entries = Object.entries(ctx);
    return entries.length > 0
      ? entries.map(([key, value]) => ({ key, value }))
      : [{ key: "", value: "" }];
  }

  function emitChange() {
    const result: Record<string, string> = {};
    for (const pair of pairs) {
      if (pair.key.trim()) {
        result[pair.key.trim()] = pair.value;
      }
    }
    context = result;
    onchange?.(result);
  }

  function addPair() {
    pairs = [...pairs, { key: "", value: "" }];
  }

  function removePair(index: number) {
    pairs = pairs.filter((_, i) => i !== index);
    if (pairs.length === 0) pairs = [{ key: "", value: "" }];
    emitChange();
  }

  // Sync external context changes
  $effect(() => {
    const currentKeys = Object.keys(context).sort().join(",");
    const pairKeys = pairs
      .filter((p) => p.key.trim())
      .map((p) => p.key.trim())
      .sort()
      .join(",");
    if (currentKeys !== pairKeys) {
      pairs = toPairs(context);
    }
  });
</script>

<div class="context-panel">
  <div class="label">Context (key = value)</div>
  {#each pairs as pair, i}
    <div class="pair-row">
      <input
        type="text"
        class="input key-input"
        placeholder="key"
        bind:value={pair.key}
        oninput={emitChange}
      />
      <span class="eq">=</span>
      <input
        type="text"
        class="input value-input"
        placeholder="value"
        bind:value={pair.value}
        oninput={emitChange}
      />
      <button class="remove-btn" onclick={() => removePair(i)}>x</button>
    </div>
  {/each}
  <button class="add-btn" onclick={addPair}>+ Add field</button>
</div>

<style>
  .context-panel {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .pair-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .eq {
    color: var(--dao-muted);
    font-family: var(--font-mono);
    font-size: 13px;
    flex-shrink: 0;
  }

  .input {
    background: var(--dao-bg);
    border: 1px solid var(--dao-border);
    border-radius: var(--radius-sm);
    color: var(--dao-text);
    font-family: var(--font-mono);
    font-size: 13px;
    padding: 6px 10px;
  }

  .input:focus {
    outline: 1px solid var(--ci-blue);
  }

  .key-input {
    width: 120px;
    flex-shrink: 0;
  }

  .value-input {
    flex: 1;
    min-width: 0;
  }

  .remove-btn {
    background: none;
    color: var(--dao-muted);
    font-family: var(--font-mono);
    font-size: 14px;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }

  .remove-btn:hover {
    color: var(--ci-red);
    background: var(--dao-surface-2);
  }

  .add-btn {
    background: none;
    color: var(--dao-muted);
    font-size: 12px;
    padding: 4px 0;
    text-align: left;
  }

  .add-btn:hover {
    color: var(--ci-blue);
  }
</style>
