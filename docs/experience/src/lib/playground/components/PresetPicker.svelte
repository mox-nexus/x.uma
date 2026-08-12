<script lang="ts">
  import type { Preset } from "../types.js";

  let {
    presets,
    active,
    onselect,
  }: {
    presets: Preset[];
    active: string;
    onselect: (preset: Preset) => void;
  } = $props();
</script>

<div class="preset-picker">
  <div class="label">Presets</div>
  <div class="pills">
    {#each presets as preset}
      <button
        class="pill"
        class:active={active === preset.id}
        onclick={() => onselect(preset)}
        title={preset.description}
      >
        {preset.name}
      </button>
    {/each}
  </div>
</div>

<style>
  .preset-picker {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .pills {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .pill {
    background: var(--dao-surface-2);
    color: var(--dao-muted);
    padding: 5px 12px;
    border-radius: 20px;
    font-size: 12px;
    font-weight: 500;
    transition:
      background 0.15s,
      color 0.15s;
  }

  .pill:hover {
    background: color-mix(in srgb, var(--ci-blue) 15%, var(--dao-surface-2));
    color: var(--dao-text);
  }

  .pill.active {
    background: color-mix(in srgb, var(--ci-blue) 20%, var(--dao-surface-2));
    color: var(--ci-blue);
    outline: 1px solid color-mix(in srgb, var(--ci-blue) 40%, transparent);
  }
</style>
