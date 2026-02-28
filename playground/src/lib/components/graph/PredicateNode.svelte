<script lang="ts">
  import { Handle, Position } from "@xyflow/svelte";

  let {
    data,
  }: {
    data: {
      kind: string;
      key?: string;
      match?: string;
      label?: string;
      headers?: number;
      count?: number;
    };
  } = $props();
</script>

<div class="predicate-node" class:compound={data.kind === "AND" || data.kind === "OR" || data.kind === "NOT"}>
  <Handle type="target" position={Position.Top} />

  {#if data.kind === "single"}
    <div class="key">{data.key}</div>
    <div class="match">{data.match}</div>
  {:else if data.kind === "route"}
    <div class="key">{data.label}</div>
    {#if data.headers && data.headers > 0}
      <div class="match">+{data.headers} header{data.headers !== 1 ? "s" : ""}</div>
    {/if}
  {:else}
    <div class="kind">{data.kind}</div>
    {#if data.count}
      <div class="match">{data.count} predicates</div>
    {/if}
  {/if}

  <Handle type="source" position={Position.Bottom} />
</div>

<style>
  .predicate-node {
    background: var(--bg-surface, #282840);
    border: 2px solid var(--yellow, #f9e2af);
    border-radius: 20px;
    padding: 6px 14px;
    min-width: 100px;
    text-align: center;
  }

  .compound {
    border-color: var(--peach, #fab387);
  }

  .key {
    font-weight: 600;
    font-size: 12px;
    color: var(--yellow, #f9e2af);
  }

  .kind {
    font-weight: 700;
    font-size: 13px;
    color: var(--peach, #fab387);
  }

  .match {
    font-size: 10px;
    color: var(--text-muted, #a6adc8);
    margin-top: 1px;
    max-width: 160px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
