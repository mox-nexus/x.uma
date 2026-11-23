/**
 * Transform a protojson matcher config into the renderer-independent graph
 * model.
 *
 * The walk reads the document a user wrote, not an engine's internal form, so
 * the diagram cannot disagree with the config it stands beside. Field names
 * are accepted in `lowerCamelCase` (canonical protojson) and `snake_case`, as
 * the config format itself accepts them.
 *
 * Produces structure only. Position, size and styling are decided downstream
 * by `layout.ts` and `draw.ts`.
 */
import type { GraphData, GraphEdge, GraphNode } from "./types.js";

export type { GraphData };

type Json = Record<string, unknown>;

let idCounter = 0;
function nextId(prefix: string): string {
  return `${prefix}-${idCounter++}`;
}

/** Convert a protojson config string to graph data. */
export function configToGraph(configJson: string): GraphData {
  idCounter = 0;
  const nodes: GraphNode[] = [];
  const edges: GraphEdge[] = [];
  try {
    const config = JSON.parse(configJson);
    if (!isObject(config)) return { nodes: [], edges: [] };
    walkMatcher(config, nodes, edges, null, false);
  } catch {
    return { nodes: [], edges: [] };
  }
  return { nodes, edges };
}

function isObject(v: unknown): v is Json {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/** The first of `names` present on `o`. */
function field(o: unknown, ...names: string[]): unknown {
  if (!isObject(o)) return undefined;
  for (const n of names) if (n in o) return o[n];
  return undefined;
}

function list(v: unknown): unknown[] {
  return Array.isArray(v) ? v : [];
}

/**
 * Everything reachable from an `onNoMatch` is on the fallback path, however
 * deep. The flag once stopped at the first nested matcher, so a fallback that
 * nested instead of ending in a bare action was drawn exactly like a match.
 */
function walkMatcher(
  matcher: Json,
  nodes: GraphNode[],
  edges: GraphEdge[],
  parentId: string | null,
  isFallback: boolean,
): string {
  const id = nextId("matcher");
  const tree = field(matcher, "matcherTree", "matcher_tree");
  const rules = list(field(field(matcher, "matcherList", "matcher_list"), "matchers"));

  if (tree !== undefined) {
    const [rule, entries] = treeEntries(tree);
    nodes.push({
      id,
      type: "matcher",
      data: { label: `Matcher (${rule} tree)`, count: entries.length },
    });
    link(edges, parentId, id, isFallback);
    // An entry is a key and what the key dispatches to. There is no predicate
    // to draw: a tree looks a key up rather than evaluating a condition.
    for (const [key, onMatch] of entries) {
      const entryId = nextId("entry");
      nodes.push({ id: entryId, type: "predicate", data: { label: `${rule}: ${key}` } });
      edges.push({ id: nextId("e"), source: id, target: entryId });
      walkOnMatch(onMatch, nodes, edges, entryId, isFallback);
    }
  } else {
    nodes.push({ id, type: "matcher", data: { label: "Matcher", count: rules.length } });
    link(edges, parentId, id, isFallback);
    for (const rule of rules) {
      const predId = walkPredicate(field(rule, "predicate"), nodes, edges);
      edges.push({ id: nextId("e"), source: id, target: predId });
      walkOnMatch(field(rule, "onMatch", "on_match"), nodes, edges, predId, isFallback);
    }
  }

  const onNoMatch = field(matcher, "onNoMatch", "on_no_match");
  if (onNoMatch !== undefined) walkOnMatch(onNoMatch, nodes, edges, id, true);

  return id;
}

function link(edges: GraphEdge[], from: string | null, to: string, isFallback: boolean): void {
  if (from === null) return;
  edges.push({ id: nextId("e"), source: from, target: to, kind: isFallback ? "no-match" : undefined });
}

/** A tree's dispatch rule and its `[key, onMatch]` entries. */
function treeEntries(tree: unknown): [string, Array<[string, unknown]>] {
  const exact = field(tree, "exactMatchMap", "exact_match_map");
  const prefix = field(tree, "prefixMatchMap", "prefix_match_map");
  const [rule, map] = exact !== undefined ? ["exact", exact] : prefix !== undefined ? ["prefix", prefix] : ["custom", undefined];
  const entries = field(map, "map");
  return [rule, isObject(entries) ? Object.entries(entries) : []];
}

function walkPredicate(pred: unknown, nodes: GraphNode[], edges: GraphEdge[]): string {
  const id = nextId("pred");

  const single = field(pred, "singlePredicate", "single_predicate");
  if (single !== undefined) {
    nodes.push({
      id,
      type: "predicate",
      data: { kind: "single", key: inputLabel(field(single, "input")), match: matchLabel(single) },
    });
    return id;
  }

  const not = field(pred, "notMatcher", "not_matcher");
  if (not !== undefined) {
    nodes.push({ id, type: "predicate", data: { kind: "NOT" } });
    edges.push({ id: nextId("e"), source: id, target: walkPredicate(not, nodes, edges) });
    return id;
  }

  const and = field(pred, "andMatcher", "and_matcher");
  const compound = and !== undefined ? and : field(pred, "orMatcher", "or_matcher");
  if (compound !== undefined) {
    const children = list(field(compound, "predicate"));
    nodes.push({
      id,
      type: "predicate",
      data: { kind: and !== undefined ? "AND" : "OR", count: children.length },
    });
    for (const child of children) {
      edges.push({ id: nextId("e"), source: id, target: walkPredicate(child, nodes, edges) });
    }
    return id;
  }

  nodes.push({ id, type: "predicate", data: { kind: "unknown" } });
  return id;
}

/** What an input reads: the key it was configured with, else the author's name for it. */
function inputLabel(input: unknown): string {
  const typed = field(input, "typedConfig", "typed_config");
  const candidates = [field(typed, "key"), field(typed, "name"), field(input, "name")];
  for (const c of candidates) if (typeof c === "string" && c !== "") return c;
  const url = field(typed, "@type");
  return typeof url === "string" ? (url.split(".").pop() ?? "input") : "input";
}

function matchLabel(single: unknown): string {
  const vm = field(single, "valueMatch", "value_match");
  for (const variant of ["exact", "prefix", "suffix", "contains"]) {
    const v = field(vm, variant);
    if (typeof v === "string") return `${variant} "${truncate(v)}"`;
  }
  const regex = field(field(vm, "safeRegex", "safe_regex"), "regex");
  if (typeof regex === "string") return `regex "${truncate(regex)}"`;
  return "custom";
}

function truncate(v: string): string {
  return v.length > 20 ? `${v.slice(0, 17)}...` : v;
}

function walkOnMatch(
  onMatch: unknown,
  nodes: GraphNode[],
  edges: GraphEdge[],
  sourceId: string,
  isFallback: boolean,
): void {
  const nested = field(onMatch, "matcher");
  if (isObject(nested)) {
    walkMatcher(nested, nodes, edges, sourceId, isFallback);
    return;
  }

  const action = field(onMatch, "action");
  if (action === undefined) return;
  const id = nextId("action");
  nodes.push({
    id,
    type: isFallback ? "fallback" : "action",
    data: { action: actionLabel(action) },
  });
  link(edges, sourceId, id, isFallback);
}

/** A NamedAction's name, else the extension's own name. */
function actionLabel(action: unknown): string {
  const named = field(field(action, "typedConfig", "typed_config"), "name");
  if (typeof named === "string" && named !== "") return named;
  const name = field(action, "name");
  return typeof name === "string" ? name : "action";
}
