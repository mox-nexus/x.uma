/**
 * Node text and sizing.
 *
 * This module is the ONLY place node dimensions are computed. Layout and
 * rendering both read the result, so the laid-out box and the drawn box are
 * the same box by construction.
 *
 * Sizing is derived arithmetically rather than measured from the DOM. The
 * playground sets a monospace face (mox `--font-mono`, IBM Plex Mono), whose
 * advance width is a fixed fraction of the font size, so character count is
 * enough. That keeps sizing deterministic, identical on server and client, and
 * free of the measure-then-layout race that DOM measurement introduces.
 */
import type { GraphNode, TextLine } from "./types.js";

/** IBM Plex Mono advance width as a fraction of font size. */
const MONO_ADVANCE = 0.6;

/** Type sizes from the mox 9-grid scale (--type-sm, --type-xs). */
const SIZE_PRIMARY = 15;
const SIZE_SECONDARY = 11;

/** Every line occupies one 9-grid step, whatever its font size. */
const LINE_HEIGHT = 18;

/** Padding on the 9-grid: --space-2 horizontal, --space-1 vertical. */
const PAD_X = 18;
const PAD_Y = 9;

/** Width bounds, both multiples of 9. */
const MIN_WIDTH = 108;
const MAX_WIDTH = 270;

const ELLIPSIS = "…";

export function textWidth(text: string, size: number): number {
  return text.length * size * MONO_ADVANCE;
}

/** Longest string of `size` that fits in `max` px, ellipsised if truncated. */
function fit(text: string, size: number, max: number): string {
  if (textWidth(text, size) <= max) return text;
  const budget = Math.max(1, Math.floor(max / (size * MONO_ADVANCE)) - 1);
  return text.slice(0, budget) + ELLIPSIS;
}

/**
 * The label lines for a node, before truncation.
 *
 * Mirrors the `data` shapes produced by `config-to-graph.ts`. A node type with
 * no case here renders its kind, so an unhandled shape degrades to something
 * readable instead of an empty box.
 */
function rawLines(node: GraphNode): TextLine[] {
  const d = node.data as Record<string, string | number | undefined>;
  const primary = (text: string): TextLine => ({ text, size: SIZE_PRIMARY, tone: "primary" });
  const secondary = (text: string): TextLine => ({ text, size: SIZE_SECONDARY, tone: "secondary" });

  switch (node.type) {
    case "matcher": {
      const lines = [primary(String(d.label ?? "Matcher"))];
      const count = Number(d.count ?? 0);
      if (count > 0) lines.push(secondary(`${count} ${count === 1 ? "matcher" : "matchers"}`));
      return lines;
    }

    case "action":
    case "fallback":
      return [primary(String(d.action ?? "action"))];

    case "predicate": {
      const kind = String(d.kind ?? "unknown");

      if (kind === "single") {
        const lines = [primary(String(d.key ?? "input"))];
        if (d.match !== undefined) lines.push(secondary(String(d.match)));
        return lines;
      }

      if (kind === "route") {
        const lines = [primary(String(d.label ?? "any"))];
        const headers = Number(d.headers ?? 0);
        if (headers > 0) {
          lines.push(secondary(`+${headers} header${headers === 1 ? "" : "s"}`));
        }
        return lines;
      }

      // AND / OR / NOT / unknown
      const lines = [primary(kind)];
      const count = Number(d.count ?? 0);
      if (count > 0) lines.push(secondary(`${count} predicates`));
      return lines;
    }
  }
}

export interface Measured {
  width: number;
  height: number;
  lines: TextLine[];
}

/** Compute a node's box and its truncated text. */
export function measureNode(node: GraphNode): Measured {
  const lines = rawLines(node);
  const maxText = MAX_WIDTH - PAD_X * 2;

  const natural = Math.max(...lines.map((l) => textWidth(l.text, l.size)));
  const width = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, Math.ceil(natural) + PAD_X * 2));
  const height = lines.length * LINE_HEIGHT + PAD_Y * 2;

  return {
    width,
    height,
    lines: lines.map((l) => ({ ...l, text: fit(l.text, l.size, maxText) })),
  };
}

export const metrics = { LINE_HEIGHT, PAD_X, PAD_Y };
