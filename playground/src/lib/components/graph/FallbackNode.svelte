<script lang="ts">
  import { Handle, Position } from "@xyflow/svelte";

  let { data }: { data: { action: string } } = $props();
</script>

<div class="fallback-node">
  <Handle type="target" position={Position.Top} />
  <div class="label">fallback</div>
  <div class="action">{data.action}</div>
</div>

<style>
  .fallback-node {
    background: color-mix(in srgb, var(--red, #f38ba8) 10%, var(--bg-surface, #282840));
    border: 2px dashed var(--red, #f38ba8);
    border-radius: 8px;
    padding: 6px 14px;
    min-width: 80px;
    text-align: center;
    opacity: 0.8;
  }

  .label {
    font-size: 10px;
    color: var(--text-muted, #a6adc8);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .action {
    font-weight: 600;
    font-size: 12px;
    color: var(--red, #f38ba8);
  }
</style>
