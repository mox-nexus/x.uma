/**
 * Graph model types.
 *
 * Deliberately independent of any rendering library. `config-to-graph.ts`
 * produces this shape; renderers consume it. Swapping the renderer must not
 * touch the model.
 */

/** What a node represents in the matcher tree. Drives colour role and shape. */
export type NodeKind = "matcher" | "predicate" | "action" | "fallback";

/** A node before layout: no position, no size. */
export interface GraphNode {
  id: string;
  type: NodeKind;
  data: Record<string, unknown>;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  /**
   * Semantic role, not styling. `no-match` is the `on_no_match` fallback path.
   * The renderer decides how to distinguish it.
   */
  kind?: "no-match";
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

/**
 * A node after measurement and layout.
 *
 * `width`/`height` are the same numbers given to the layout engine and used to
 * draw. They cannot disagree, because nothing else computes them.
 */
export interface PlacedNode extends GraphNode {
  x: number;
  y: number;
  width: number;
  height: number;
  /** Rendered text, already truncated to fit `width`. */
  lines: TextLine[];
}

export interface TextLine {
  text: string;
  /** Font size in px. Values come from the mox 9-grid type scale. */
  size: number;
  /** Emphasis tier, mapped to colour by the renderer. */
  tone: "primary" | "secondary";
}

export interface PlacedGraph {
  nodes: PlacedNode[];
  edges: GraphEdge[];
  width: number;
  height: number;
}
