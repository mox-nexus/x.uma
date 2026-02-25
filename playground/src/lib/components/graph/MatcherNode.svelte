<script lang="ts">
  import { Handle, Position } from "@xyflow/svelte";

  let { data }: { data: { label: string; count: number } } = $props();
</script>

<div class="matcher-node">
  <Handle type="target" position={Position.Top} />
  <div class="label">{data.label}</div>
  <div class="count">{data.count} rule{data.count !== 1 ? "s" : ""}</div>
  <Handle type="source" position={Position.Bottom} />
</div>

<style>
  .matcher-node {
    background: var(--bg-surface, #282840);
    border: 2px solid var(--accent, #89b4fa);
    border-radius: 8px;
    padding: 8px 16px;
    min-width: 120px;
    text-align: center;
  }

  .label {
    font-weight: 700;
    font-size: 13px;
    color: var(--accent, #89b4fa);
  }

  .count {
    font-size: 11px;
    color: var(--text-muted, #a6adc8);
    margin-top: 2px;
  }
</style>
