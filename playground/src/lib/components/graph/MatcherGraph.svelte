<script lang="ts">
  import {
    SvelteFlow,
    Controls,
    Background,
    type NodeTypes,
  } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";
  import type { Node, Edge } from "@xyflow/svelte";
  import type { ModeKind } from "$lib/types.js";
  import { configToGraph } from "$lib/graph/config-to-graph.js";
  import { layoutGraph } from "$lib/graph/layout.js";
  import MatcherNode from "./MatcherNode.svelte";
  import PredicateNode from "./PredicateNode.svelte";
  import ActionNode from "./ActionNode.svelte";
  import FallbackNode from "./FallbackNode.svelte";

  let { configJson, mode }: { configJson: string; mode: ModeKind } = $props();

  let nodes: Node[] = $state.raw([]);
  let edges: Edge[] = $state.raw([]);

  const nodeTypes: NodeTypes = {
    matcher: MatcherNode as any,
    predicate: PredicateNode as any,
    action: ActionNode as any,
    fallback: FallbackNode as any,
  };

  let layoutTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    // Track both configJson and mode
    const _json = configJson;
    const _mode = mode;

    clearTimeout(layoutTimer);
    layoutTimer = setTimeout(async () => {
      const raw = configToGraph(_json, _mode);
      if (raw.nodes.length === 0) {
        nodes = [];
        edges = [];
        return;
      }
      const laid = await layoutGraph(raw.nodes, raw.edges);
      nodes = laid.nodes;
      edges = laid.edges;
    }, 200);
  });
</script>

<div class="graph-wrapper" style="height: {Math.min(600, Math.max(280, nodes.length * 50))}px">
  {#if nodes.length > 0}
    <SvelteFlow
      {nodes}
      {edges}
      {nodeTypes}
      fitView
      nodesDraggable={false}
      nodesConnectable={false}
      elementsSelectable={false}
      panOnDrag={true}
      zoomOnScroll={true}
      minZoom={0.3}
      maxZoom={2}
      colorMode="dark"
      proOptions={{ hideAttribution: true }}
    >
      <Controls showInteractive={false} showLock={false} />
      <Background variant="dots" gap={16} size={1} />
    </SvelteFlow>
  {:else}
    <div class="empty">
      <span>Edit config to see the matcher tree</span>
    </div>
  {/if}
</div>

<style>
  .graph-wrapper {
    width: 100%;
    height: 300px; /* overridden by inline style */
    border: 1px solid var(--border, #45475a);
    border-radius: var(--radius, 6px);
    overflow: hidden;
    position: relative;
  }

  .graph-wrapper :global(.svelte-flow) {
    background: var(--bg, #1e1e2e) !important;
  }

  .graph-wrapper :global(.svelte-flow__background) {
    opacity: 0.3;
  }

  .graph-wrapper :global(.svelte-flow__edge-path) {
    stroke: var(--text-muted, #a6adc8) !important;
    stroke-width: 1.5;
  }

  .graph-wrapper :global(.svelte-flow__controls) {
    background: var(--bg-surface, #282840);
    border: 1px solid var(--border, #45475a);
    border-radius: var(--radius, 6px);
  }

  .graph-wrapper :global(.svelte-flow__controls-button) {
    background: var(--bg-surface, #282840);
    border-color: var(--border, #45475a);
    fill: var(--text-muted, #a6adc8);
  }

  .graph-wrapper :global(.svelte-flow__controls-button:hover) {
    background: var(--bg-elevated, #313150);
  }

  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted, #a6adc8);
    font-size: 13px;
  }
</style>
