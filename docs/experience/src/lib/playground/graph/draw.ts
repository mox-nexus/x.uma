/**
 * roughjs drawing for the matcher tree.
 *
 * Colour is never defined here. Role names resolve to mox brand tokens at draw
 * time, so the diagram inherits the palette and cannot drift from it.
 *
 * Every shape is drawn with a seed derived from its node id. roughjs is
 * otherwise non-deterministic, and an unseeded redraw makes the whole diagram
 * twitch on each keystroke.
 */
import rough from "roughjs";
import { metrics } from "./measure.js";
import type { NodeKind, PlacedGraph, PlacedNode } from "./types.js";

/**
 * Trichrome by role, not by colour.
 *
 * matcher   structure you navigate            spark      (thesis)
 * predicate the condition under test          neutral    (unmarked)
 * action    the decision reached              emergence  (synthesis)
 * fallback  the boundary when nothing matched constraint (antithesis)
 */
const ROLE: Record<NodeKind, { stroke: string; fill: string }> = {
  matcher: { stroke: "--ci-blue", fill: "--glyph-blue" },
  predicate: { stroke: "--dao-border", fill: "--dao-surface" },
  action: { stroke: "--ci-green", fill: "--glyph-green" },
  fallback: { stroke: "--ci-red", fill: "--glyph-red" },
};

const TONE = { primary: "--dao-text", secondary: "--dao-muted" } as const;

const EDGE_STROKE = "--dao-border";

/** Used only when a token fails to resolve, so the graph still draws. */
const UNRESOLVED = "#888888";

const SVG_NS = "http://www.w3.org/2000/svg";

function tokens(root: Element) {
  const style = getComputedStyle(root);
  const cache = new Map<string, string>();
  return (name: string): string => {
    let v = cache.get(name);
    if (v === undefined) {
      v = style.getPropertyValue(name).trim() || UNRESOLVED;
      cache.set(name, v);
    }
    return v;
  };
}

/** Stable 31-bit hash, so a node's sketch is identical across redraws. */
function seedOf(id: string): number {
  let h = 2166136261;
  for (let i = 0; i < id.length; i++) {
    h ^= id.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return Math.abs(h) % 2 ** 31;
}

function textEl(x: number, y: number, text: string, size: number, fill: string): SVGTextElement {
  const el = document.createElementNS(SVG_NS, "text");
  el.setAttribute("x", String(x));
  el.setAttribute("y", String(y));
  el.setAttribute("text-anchor", "middle");
  el.setAttribute("dominant-baseline", "middle");
  el.setAttribute("fill", fill);
  el.setAttribute("style", `font-family: var(--font-mono); font-size: ${size}px`);
  el.textContent = text;
  return el;
}

/** Orthogonal elbow from the bottom of `from` to the top of `to`. */
function elbow(from: PlacedNode, to: PlacedNode): [number, number][] {
  const x1 = from.x + from.width / 2;
  const y1 = from.y + from.height;
  const x2 = to.x + to.width / 2;
  const y2 = to.y;
  const mid = y1 + (y2 - y1) / 2;
  return x1 === x2
    ? [
        [x1, y1],
        [x2, y2],
      ]
    : [
        [x1, y1],
        [x1, mid],
        [x2, mid],
        [x2, y2],
      ];
}

/** Draw `graph` into `svg`, replacing anything already there. */
export function draw(svg: SVGSVGElement, graph: PlacedGraph): void {
  svg.replaceChildren();
  if (graph.nodes.length === 0) return;

  const token = tokens(svg.ownerDocument.documentElement);
  const rc = rough.svg(svg);
  const byId = new Map(graph.nodes.map((n) => [n.id, n]));

  // Edges first so nodes sit above them.
  for (const edge of graph.edges) {
    const from = byId.get(edge.source);
    const to = byId.get(edge.target);
    if (!from || !to) continue;

    const noMatch = edge.kind === "no-match";
    const path = elbow(from, to);

    svg.appendChild(
      rc.linearPath(path, {
        stroke: token(noMatch ? "--ci-red" : EDGE_STROKE),
        strokeWidth: 1.5,
        roughness: 1.1,
        bowing: 1.4,
        seed: seedOf(edge.id),
        ...(noMatch ? { strokeLineDash: [8, 8] } : {}),
      }),
    );

    if (noMatch) {
      // Label sits on the horizontal run of the elbow, mid-descent.
      const [mx, my] = path[Math.floor(path.length / 2)];
      svg.appendChild(textEl(mx, my - 9, "no match", 11, token("--ci-red")));
    }
  }

  for (const node of graph.nodes) {
    const role = ROLE[node.type];
    svg.appendChild(
      rc.rectangle(node.x, node.y, node.width, node.height, {
        stroke: token(role.stroke),
        strokeWidth: 1.5,
        fill: token(role.fill),
        fillStyle: "solid",
        roughness: 1.2,
        bowing: 1.6,
        seed: seedOf(node.id),
      }),
    );

    // Text block is centred vertically within the box.
    const blockHeight = node.lines.length * metrics.LINE_HEIGHT;
    let y = node.y + (node.height - blockHeight) / 2 + metrics.LINE_HEIGHT / 2;
    for (const line of node.lines) {
      svg.appendChild(
        textEl(node.x + node.width / 2, y, line.text, line.size, token(TONE[line.tone])),
      );
      y += metrics.LINE_HEIGHT;
    }
  }
}
