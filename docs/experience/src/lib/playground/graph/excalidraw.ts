/**
 * Export a laid-out matcher tree as an `.excalidraw` file.
 *
 * The format is plain JSON, so this writes it directly rather than depending on
 * `@excalidraw/excalidraw` (a React package) to produce a file the app opens
 * anyway.
 *
 * Colours here are the vivid trichrome, not the soft OKLCH used on screen. Per
 * the mox system the soft values are the documentation layer and the vivid ones
 * are the brand layer; an exported file travels into decks and design docs, so
 * it is a brand-layer artifact.
 */
import { textWidth } from "./measure.js";
import type { NodeKind, PlacedGraph, PlacedNode } from "./types.js";

const SPARK = "#4da6ff";
const CONSTRAINT = "#d45555";
const EMERGENCE = "#00ff41";
const NEUTRAL = "#adb5bd";
const INK = "#e9ecef";

const STROKE: Record<NodeKind, string> = {
  matcher: SPARK,
  predicate: NEUTRAL,
  action: EMERGENCE,
  fallback: CONSTRAINT,
};

/** Excalidraw's hand-drawn face. */
const FONT_HAND = 1;

interface ExcalidrawElement {
  [key: string]: unknown;
}

const base = (id: string, seed: number): ExcalidrawElement => ({
  id,
  angle: 0,
  strokeWidth: 1,
  strokeStyle: "solid",
  roughness: 1,
  opacity: 100,
  groupIds: [],
  frameId: null,
  roundness: null,
  seed,
  version: 1,
  versionNonce: seed,
  isDeleted: false,
  boundElements: [],
  updated: 1,
  link: null,
  locked: false,
});

let counter = 0;
const nextSeed = () => (counter = (counter + 104729) % 2147483647);

function rectangle(node: PlacedNode): ExcalidrawElement {
  return {
    ...base(`rect-${node.id}`, nextSeed()),
    type: "rectangle",
    x: node.x,
    y: node.y,
    width: node.width,
    height: node.height,
    strokeColor: STROKE[node.type],
    backgroundColor: "transparent",
    fillStyle: "solid",
  };
}

function label(node: PlacedNode): ExcalidrawElement[] {
  const lineHeightPx = 18;
  const blockHeight = node.lines.length * lineHeightPx;
  const top = node.y + (node.height - blockHeight) / 2;

  return node.lines.map((line, i) => {
    const width = textWidth(line.text, line.size);
    return {
      ...base(`text-${node.id}-${i}`, nextSeed()),
      type: "text",
      x: node.x + (node.width - width) / 2,
      y: top + i * lineHeightPx,
      width,
      height: lineHeightPx,
      strokeColor: line.tone === "primary" ? INK : NEUTRAL,
      backgroundColor: "transparent",
      fillStyle: "solid",
      text: line.text,
      originalText: line.text,
      fontSize: line.size,
      fontFamily: FONT_HAND,
      textAlign: "center",
      verticalAlign: "middle",
      containerId: null,
      lineHeight: lineHeightPx / line.size,
    };
  });
}

function connector(from: PlacedNode, to: PlacedNode, id: string): ExcalidrawElement {
  const x1 = from.x + from.width / 2;
  const y1 = from.y + from.height;
  const x2 = to.x + to.width / 2;
  const y2 = to.y;

  return {
    ...base(`edge-${id}`, nextSeed()),
    type: "line",
    x: x1,
    y: y1,
    width: Math.abs(x2 - x1),
    height: Math.abs(y2 - y1),
    strokeColor: NEUTRAL,
    backgroundColor: "transparent",
    fillStyle: "solid",
    // Excalidraw stores points relative to the element origin.
    points: [
      [0, 0],
      [0, (y2 - y1) / 2],
      [x2 - x1, (y2 - y1) / 2],
      [x2 - x1, y2 - y1],
    ],
    lastCommittedPoint: null,
    startBinding: null,
    endBinding: null,
    startArrowhead: null,
    endArrowhead: null,
  };
}

/** Serialize a laid-out graph to `.excalidraw` file contents. */
export function toExcalidraw(graph: PlacedGraph): string {
  counter = 0;
  const byId = new Map(graph.nodes.map((n) => [n.id, n]));
  const elements: ExcalidrawElement[] = [];

  for (const edge of graph.edges) {
    const from = byId.get(edge.source);
    const to = byId.get(edge.target);
    if (from && to) elements.push(connector(from, to, edge.id));
  }

  for (const node of graph.nodes) {
    elements.push(rectangle(node), ...label(node));
  }

  return JSON.stringify(
    {
      type: "excalidraw",
      version: 2,
      source: "https://github.com/mox-nexus/x.uma",
      elements,
      appState: { gridSize: null, viewBackgroundColor: "#ffffff" },
      files: {},
    },
    null,
    2,
  );
}
