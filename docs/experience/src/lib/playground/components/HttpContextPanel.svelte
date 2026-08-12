<script lang="ts">
  let {
    method = $bindable("GET"),
    path = $bindable("/"),
    headers = $bindable({}),
    onchange,
  }: {
    method: string;
    path: string;
    headers: Record<string, string>;
    onchange?: () => void;
  } = $props();

  let headerPairs = $state(toHeaderPairs(headers));

  function toHeaderPairs(
    h: Record<string, string>,
  ): Array<{ key: string; value: string }> {
    const entries = Object.entries(h);
    return entries.length > 0
      ? entries.map(([key, value]) => ({ key, value }))
      : [{ key: "", value: "" }];
  }

  function emitHeaders() {
    const result: Record<string, string> = {};
    for (const pair of headerPairs) {
      if (pair.key.trim()) {
        result[pair.key.trim().toLowerCase()] = pair.value;
      }
    }
    headers = result;
    onchange?.();
  }

  function addHeader() {
    headerPairs = [...headerPairs, { key: "", value: "" }];
  }

  function removeHeader(index: number) {
    headerPairs = headerPairs.filter((_, i) => i !== index);
    if (headerPairs.length === 0) headerPairs = [{ key: "", value: "" }];
    emitHeaders();
  }

  $effect(() => {
    const currentKeys = Object.keys(headers).sort().join(",");
    const pairKeys = headerPairs
      .filter((p) => p.key.trim())
      .map((p) => p.key.trim().toLowerCase())
      .sort()
      .join(",");
    if (currentKeys !== pairKeys) {
      headerPairs = toHeaderPairs(headers);
    }
  });
</script>

<div class="http-context">
  <div class="label">HTTP Request</div>

  <div class="method-path-row">
    <select
      class="input method-select"
      bind:value={method}
      onchange={() => onchange?.()}
    >
      <option>GET</option>
      <option>POST</option>
      <option>PUT</option>
      <option>DELETE</option>
      <option>PATCH</option>
      <option>HEAD</option>
      <option>OPTIONS</option>
    </select>
    <input
      type="text"
      class="input path-input"
      placeholder="/path"
      bind:value={path}
      oninput={() => onchange?.()}
    />
  </div>

  <div class="label" style="margin-top: 12px">Headers</div>
  {#each headerPairs as pair, i}
    <div class="pair-row">
      <input
        type="text"
        class="input key-input"
        placeholder="header"
        bind:value={pair.key}
        oninput={emitHeaders}
      />
      <span class="eq">:</span>
      <input
        type="text"
        class="input value-input"
        placeholder="value"
        bind:value={pair.value}
        oninput={emitHeaders}
      />
      <button class="remove-btn" onclick={() => removeHeader(i)}>x</button>
    </div>
  {/each}
  <button class="add-btn" onclick={addHeader}>+ Add header</button>
</div>

<style>
  .http-context {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .method-path-row {
    display: flex;
    gap: 8px;
  }

  .method-select {
    background: var(--dao-bg);
    border: 1px solid var(--dao-border);
    border-radius: var(--radius-sm);
    color: var(--ci-blue);
    font-family: var(--font-mono);
    font-size: 13px;
    font-weight: 600;
    padding: 6px 10px;
    width: 100px;
    flex-shrink: 0;
  }

  .path-input {
    flex: 1;
    min-width: 0;
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
    width: 140px;
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
