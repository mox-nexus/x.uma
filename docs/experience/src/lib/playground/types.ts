/** Which context a config is evaluated against: a string map, or an HTTP request. */
export type ModeKind = "kv" | "http";

export type EvalResult =
  | { kind: "match"; action: string }
  | { kind: "no-match" }
  | { kind: "error"; message: string; detail?: string };

export interface Preset {
  id: string;
  name: string;
  mode: ModeKind;
  /** Canonical protojson for `xds.type.matcher.v3.Matcher`. */
  config: string;
  context: Record<string, string>;
  /** For HTTP mode: method, path, headers */
  http?: {
    method: string;
    path: string;
    headers: Record<string, string>;
  };
  description: string;
}
