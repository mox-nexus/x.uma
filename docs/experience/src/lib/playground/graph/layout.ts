/**
 * elkjs layout wrapper for auto-positioning graph nodes.
 *
 * Computes a top-to-bottom hierarchical layout using the ELK layered algorithm.
 *
 * Node dimensions come from `measure.ts` and are carried through to the
 * renderer unchanged. Earlier versions passed fixed constants to ELK while the
 * DOM sized nodes by content, so the layout was computed for boxes that were
 * never drawn and neighbours overlapped. Measurement is now the single source.
 */
import ELK from "elkjs/lib/elk.bundled.js";
import { measureNode } from "./measure.js";
import type { GraphData, PlacedGraph, PlacedNode } from "./types.js";

const elk = new ELK();

/** Spacing on the mox 9-grid: --space-4 within a layer, --space-5 between. */
const SPACING_NODE = "36";
const SPACING_LAYER = "54";

/** Breathing room around the drawn graph, --space-2. */
const MARGIN = 18;

export async function layoutGraph(graph: GraphData): Promise<PlacedGraph> {
  if (graph.nodes.length === 0) {
    return { nodes: [], edges: [], width: 0, height: 0 };
  }

  const measured = new Map(graph.nodes.map((n) => [n.id, measureNode(n)]));

  const laid = await elk.layout({
    id: "root",
    layoutOptions: {
      "elk.algorithm": "layered",
      "elk.direction": "DOWN",
      "elk.spacing.nodeNode": SPACING_NODE,
      "elk.layered.spacing.nodeNodeBetweenLayers": SPACING_LAYER,
      "elk.layered.nodePlacement.strategy": "BRANDES_KOEPF",
    },
    children: graph.nodes.map((n) => {
      const m = measured.get(n.id)!;
      return { id: n.id, width: m.width, height: m.height };
    }),
    edges: graph.edges.map((e) => ({
      id: e.id,
      sources: [e.source],
      targets: [e.target],
    })),
  });

  const placed: PlacedNode[] = graph.nodes.map((node) => {
    const m = measured.get(node.id)!;
    const pos = laid.children?.find((c) => c.id === node.id);
    return {
      ...node,
      x: (pos?.x ?? 0) + MARGIN,
      y: (pos?.y ?? 0) + MARGIN,
      width: m.width,
      height: m.height,
      lines: m.lines,
    };
  });

  return {
    nodes: placed,
    edges: graph.edges,
    width: Math.max(...placed.map((n) => n.x + n.width)) + MARGIN,
    height: Math.max(...placed.map((n) => n.y + n.height)) + MARGIN,
  };
}
