/**
 * elkjs layout wrapper for auto-positioning graph nodes.
 *
 * Computes a top-to-bottom hierarchical layout using the ELK layered algorithm.
 */
import ELK from "elkjs/lib/elk.bundled.js";
import type { Node, Edge } from "@xyflow/svelte";

const elk = new ELK();

const NODE_WIDTH = 180;
const NODE_HEIGHT = 50;

export async function layoutGraph(
  nodes: Node[],
  edges: Edge[],
): Promise<{ nodes: Node[]; edges: Edge[] }> {
  if (nodes.length === 0) return { nodes: [], edges: [] };

  const graph = {
    id: "root",
    layoutOptions: {
      "elk.algorithm": "layered",
      "elk.direction": "DOWN",
      "elk.spacing.nodeNode": "40",
      "elk.layered.spacing.nodeNodeBetweenLayers": "60",
      "elk.layered.nodePlacement.strategy": "BRANDES_KOEPF",
    },
    children: nodes.map((n) => ({
      id: n.id,
      width: NODE_WIDTH,
      height: NODE_HEIGHT,
    })),
    edges: edges.map((e) => ({
      id: e.id,
      sources: [e.source],
      targets: [e.target],
    })),
  };

  const laid = await elk.layout(graph);

  const positioned = nodes.map((node) => {
    const elkNode = laid.children?.find((c) => c.id === node.id);
    return {
      ...node,
      position: {
        x: elkNode?.x ?? 0,
        y: elkNode?.y ?? 0,
      },
    };
  });

  return { nodes: positioned, edges };
}
