<script lang="ts">
  import type { ModeKind } from "../types.js";

  let {
    mode = $bindable("config"),
    onchange,
  }: {
    mode: ModeKind;
    onchange?: (mode: ModeKind) => void;
  } = $props();

  function select(m: ModeKind) {
    mode = m;
    onchange?.(m);
  }
</script>

<div class="mode-tabs">
  <button
    class="tab"
    class:active={mode === "config"}
    onclick={() => select("config")}
  >
    Config
  </button>
  <button
    class="tab"
    class:active={mode === "http"}
    onclick={() => select("http")}
  >
    HTTP
  </button>
</div>

<style>
  .mode-tabs {
    display: flex;
    gap: 2px;
    background: var(--dao-surface-2);
    border-radius: var(--radius-sm);
    padding: 2px;
  }

  .tab {
    padding: 6px 16px;
    border-radius: 4px;
    font-size: 13px;
    font-weight: 500;
    color: var(--dao-muted);
    background: transparent;
    transition:
      background 0.15s,
      color 0.15s;
  }

  .tab:hover {
    color: var(--dao-text);
  }

  .tab.active {
    background: var(--dao-surface);
    color: var(--dao-text);
  }
</style>
