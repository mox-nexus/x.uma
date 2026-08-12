/**
 * Transform matcher config JSON into the renderer-independent graph model.
 *
 * Two paths:
 * - Config mode: MatcherConfig AST → tree graph
 * - HTTP mode: Route array → flat route graph
 *
 * Produces structure only. Position, size and styling are decided downstream by
 * `layout.ts` and `draw.ts`.
 */
import {
  parseMatcherConfig,
  type MatcherConfig,
  type FieldMatcherConfig,
  type PredicateConfig,
  type OnMatchConfig,
  SinglePredicateConfig,
  AndPredicateConfig,
  OrPredicateConfig,
  NotPredicateConfig,
  ActionConfig,
  MatcherOnMatchConfig,
} from "xuma";
import type { ModeKind } from "../types.js";
import type { GraphData, GraphEdge, GraphNode } from "./types.js";

export type { GraphData };

let idCounter = 0;
function nextId(prefix: string): string {
  return `${prefix}-${idCounter++}`;
}

/** Convert config JSON string to graph data. */
export function configToGraph(configJson: string, mode: ModeKind): GraphData {
  idCounter = 0;
  try {
    const parsed = JSON.parse(configJson);
    if (mode === "http") {
      return httpToGraph(parsed);
    }
    const config = parseMatcherConfig(parsed);
    return matcherConfigToGraph(config);
  } catch {
    return { nodes: [], edges: [] };
  }
}

// ---------------------------------------------------------------------------
// Config mode: MatcherConfig AST → graph
// ---------------------------------------------------------------------------

function matcherConfigToGraph(config: MatcherConfig<string>): GraphData {
  const nodes: GraphNode[] = [];
  const edges: GraphEdge[] = [];
  walkMatcher(config, nodes, edges, null);
  return { nodes, edges };
}

function walkMatcher(
  config: MatcherConfig<string>,
  nodes: GraphNode[],
  edges: GraphEdge[],
  parentId: string | null,
): string {
  const id = nextId("matcher");
  nodes.push({
    id,
    type: "matcher",
    data: { label: "Matcher", count: config.matchers.length },
  });

  if (parentId) {
    edges.push({
      id: nextId("e"),
      source: parentId,
      target: id,
    });
  }

  for (const fm of config.matchers) {
    walkFieldMatcher(fm, nodes, edges, id);
  }

  if (config.onNoMatch) {
    walkOnMatch(config.onNoMatch, nodes, edges, id, true);
  }

  return id;
}

function walkFieldMatcher(
  fm: FieldMatcherConfig<string>,
  nodes: GraphNode[],
  edges: GraphEdge[],
  matcherId: string,
): void {
  const predId = walkPredicate(fm.predicate, nodes, edges);

  edges.push({
    id: nextId("e"),
    source: matcherId,
    target: predId,
  });

  walkOnMatch(fm.onMatch, nodes, edges, predId, false);
}

function walkPredicate(
  pred: PredicateConfig,
  nodes: GraphNode[],
  edges: GraphEdge[],
): string {
  if (pred instanceof SinglePredicateConfig) {
    const id = nextId("pred");
    const key =
      (pred.input.config as Record<string, string>).key ??
      (pred.input.config as Record<string, string>).name ??
      pred.input.typeUrl.split(".").pop() ??
      "input";
    const matchLabel = formatValueMatch(pred.matcher);
    nodes.push({
      id,
      type: "predicate",
      data: { kind: "single", key, match: matchLabel },
    });
    return id;
  }

  if (pred instanceof AndPredicateConfig) {
    const id = nextId("pred");
    nodes.push({
      id,
      type: "predicate",
      data: { kind: "AND", count: pred.predicates.length },
    });
    for (const child of pred.predicates) {
      const childId = walkPredicate(child, nodes, edges);
      edges.push({
        id: nextId("e"),
        source: id,
        target: childId,
      });
    }
    return id;
  }

  if (pred instanceof OrPredicateConfig) {
    const id = nextId("pred");
    nodes.push({
      id,
      type: "predicate",
      data: { kind: "OR", count: pred.predicates.length },
    });
    for (const child of pred.predicates) {
      const childId = walkPredicate(child, nodes, edges);
      edges.push({
        id: nextId("e"),
        source: id,
        target: childId,
      });
    }
    return id;
  }

  if (pred instanceof NotPredicateConfig) {
    const id = nextId("pred");
    nodes.push({
      id,
      type: "predicate",
      data: { kind: "NOT" },
    });
    const childId = walkPredicate(pred.predicate, nodes, edges);
    edges.push({
      id: nextId("e"),
      source: id,
      target: childId,
    });
    return id;
  }

  // Fallback — shouldn't happen
  const id = nextId("pred");
  nodes.push({
    id,
    type: "predicate",
    data: { kind: "unknown" },
  });
  return id;
}

function walkOnMatch(
  onMatch: OnMatchConfig<string>,
  nodes: GraphNode[],
  edges: GraphEdge[],
  sourceId: string,
  isFallback: boolean,
): void {
  if (onMatch instanceof ActionConfig) {
    const id = nextId("action");
    nodes.push({
      id,
      type: isFallback ? "fallback" : "action",
      data: { action: onMatch.action },
    });
    edges.push({
      id: nextId("e"),
      source: sourceId,
      target: id,
      kind: isFallback ? "no-match" : undefined,
    });
    return;
  }

  if (onMatch instanceof MatcherOnMatchConfig) {
    walkMatcher(onMatch.matcher, nodes, edges, sourceId);
  }
}

function formatValueMatch(matcher: unknown): string {
  // BuiltInMatch has .variant and .value
  const m = matcher as { variant?: string; value?: string };
  if (m.variant && m.value !== undefined) {
    const v = m.value.length > 20 ? m.value.slice(0, 17) + "..." : m.value;
    return `${m.variant} "${v}"`;
  }
  return "custom";
}

// ---------------------------------------------------------------------------
// HTTP mode: Route array → flat graph
// ---------------------------------------------------------------------------

interface HttpRouteEntry {
  match: {
    method?: string;
    path?: { type: string; value: string };
    headers?: Array<{ type: string; name: string; value: string }>;
  };
  action: string;
}

function httpToGraph(data: unknown): GraphData {
  if (!Array.isArray(data)) return { nodes: [], edges: [] };
  const entries = data as HttpRouteEntry[];

  const nodes: GraphNode[] = [];
  const edges: GraphEdge[] = [];

  const rootId = nextId("routes");
  nodes.push({
    id: rootId,
    type: "matcher",
    data: { label: "Routes", count: entries.length },
  });

  for (const entry of entries) {
    const predId = nextId("pred");
    const parts: string[] = [];
    if (entry.match.method) parts.push(entry.match.method);
    if (entry.match.path) parts.push(entry.match.path.value);
    const headerCount = entry.match.headers?.length ?? 0;

    nodes.push({
      id: predId,
      type: "predicate",
      data: {
        kind: "route",
        label: parts.join(" ") || "any",
        headers: headerCount,
      },
    });

    edges.push({
      id: nextId("e"),
      source: rootId,
      target: predId,
    });

    const actionId = nextId("action");
    nodes.push({
      id: actionId,
      type: "action",
      data: { action: entry.action },
    });

    edges.push({
      id: nextId("e"),
      source: predId,
      target: actionId,
    });
  }

  return { nodes, edges };
}
