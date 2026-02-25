<script lang="ts">
  import { Handle, Position } from "@xyflow/svelte";

  let { data }: { data: { action: string } } = $props();
</script>

<div class="action-node">
  <Handle type="target" position={Position.Top} />
  <div class="action">{data.action}</div>
</div>

<style>
  .action-node {
    background: color-mix(in srgb, var(--green, #a6e3a1) 15%, var(--bg-surface, #282840));
    border: 2px solid var(--green, #a6e3a1);
    border-radius: 8px;
    padding: 6px 14px;
    min-width: 80px;
    text-align: center;
  }

  .action {
    font-weight: 600;
    font-size: 12px;
    color: var(--green, #a6e3a1);
  }
</style>
