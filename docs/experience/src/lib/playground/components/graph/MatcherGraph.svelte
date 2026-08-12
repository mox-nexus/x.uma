<script lang="ts">
  import type { ModeKind } from "$lib/playground/types.js";
  import { configToGraph } from "$lib/playground/graph/config-to-graph.js";
  import { layoutGraph } from "$lib/playground/graph/layout.js";
  import { draw } from "$lib/playground/graph/draw.js";
  import { toExcalidraw } from "$lib/playground/graph/excalidraw.js";
  import type { PlacedGraph } from "$lib/playground/graph/types.js";

  let { configJson, mode }: { configJson: string; mode: ModeKind } = $props();

  let svgEl = $state<SVGSVGElement | undefined>(undefined);
  let placed = $state.raw<PlacedGraph | null>(null);

  /** Current viewBox. Panning and zooming move this, never the geometry. */
  let view = $state({ x: 0, y: 0, w: 0, h: 0 });

  const MIN_HEIGHT = 280;
  const MAX_HEIGHT = 600;
  const ZOOM_MIN = 0.4;
  const ZOOM_MAX = 4;

  let layoutTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    const json = configJson;
    const kind = mode;

    clearTimeout(layoutTimer);
    layoutTimer = setTimeout(async () => {
      const graph = await layoutGraph(configToGraph(json, kind));
      placed = graph.nodes.length > 0 ? graph : null;
      if (placed) resetView();
    }, 200);

    return () => clearTimeout(layoutTimer);
  });

  $effect(() => {
    if (svgEl && placed) draw(svgEl, placed);
  });

  function resetView() {
    if (!placed) return;
    view = { x: 0, y: 0, w: placed.width, h: placed.height };
  }

  const height = $derived(
    placed ? Math.min(MAX_HEIGHT, Math.max(MIN_HEIGHT, placed.height)) : MIN_HEIGHT,
  );

  function onWheel(event: WheelEvent) {
    if (!placed || !svgEl) return;
    event.preventDefault();

    const rect = svgEl.getBoundingClientRect();
    // Pointer position in graph coordinates, held fixed across the zoom.
    const px = view.x + ((event.clientX - rect.left) / rect.width) * view.w;
    const py = view.y + ((event.clientY - rect.top) / rect.height) * view.h;

    const factor = event.deltaY > 0 ? 1.1 : 1 / 1.1;
    const scale = placed.width / (view.w * factor);
    if (scale < ZOOM_MIN || scale > ZOOM_MAX) return;

    const w = view.w * factor;
    const h = view.h * factor;
    view = {
      w,
      h,
      x: px - ((px - view.x) / view.w) * w,
      y: py - ((py - view.y) / view.h) * h,
    };
  }

  let dragging = $state(false);
  let last = { x: 0, y: 0 };

  function onPointerDown(event: PointerEvent) {
    if (!placed) return;
    dragging = true;
    last = { x: event.clientX, y: event.clientY };
    (event.currentTarget as SVGSVGElement).setPointerCapture(event.pointerId);
  }

  function onPointerMove(event: PointerEvent) {
    if (!dragging || !svgEl) return;
    const rect = svgEl.getBoundingClientRect();
    view = {
      ...view,
      x: view.x - ((event.clientX - last.x) / rect.width) * view.w,
      y: view.y - ((event.clientY - last.y) / rect.height) * view.h,
    };
    last = { x: event.clientX, y: event.clientY };
  }

  function onPointerUp(event: PointerEvent) {
    dragging = false;
    (event.currentTarget as SVGSVGElement).releasePointerCapture(event.pointerId);
  }

  function exportExcalidraw() {
    if (!placed) return;
    const blob = new Blob([toExcalidraw(placed)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "matcher-tree.excalidraw";
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="graph-wrapper" style="height: {height}px">
  {#if placed}
    <div class="toolbar">
      <button type="button" onclick={resetView} title="Reset view">reset</button>
      <button type="button" onclick={exportExcalidraw} title="Download as .excalidraw">
        export
      </button>
    </div>
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <svg
      bind:this={svgEl}
      role="img"
      aria-label="Matcher tree diagram"
      viewBox="{view.x} {view.y} {view.w} {view.h}"
      class:dragging
      onwheel={onWheel}
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerUp}
    ></svg>
  {:else}
    <div class="empty">
      <span>Edit config to see the matcher tree</span>
    </div>
  {/if}
</div>

<style>
  .graph-wrapper {
    position: relative;
    width: 100%;
    overflow: hidden;
    border: var(--border-width) solid var(--dao-border);
    background: var(--dao-bg);
  }

  svg {
    display: block;
    width: 100%;
    height: 100%;
    cursor: grab;
    touch-action: none;
  }

  svg.dragging {
    cursor: grabbing;
  }

  .toolbar {
    position: absolute;
    top: var(--space-1);
    right: var(--space-1);
    z-index: var(--z-raised);
    display: flex;
    gap: var(--space-1);
  }

  .toolbar button {
    padding: 2px var(--space-1);
    font-family: var(--font-mono);
    font-size: var(--type-xs);
    color: var(--dao-muted);
    background: var(--dao-surface);
    border: var(--border-width) solid var(--dao-border);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: var(--transition-colors);
  }

  .toolbar button:hover {
    color: var(--color-link);
    border-color: var(--ci-blue);
  }

  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    font-family: var(--font-mono);
    font-size: var(--type-sm);
    color: var(--dao-muted);
  }
</style>
