export type ModeKind = "config" | "http";

export type EvalResult =
  | { kind: "match"; action: string }
  | { kind: "no-match" }
  | { kind: "error"; message: string; detail?: string };

export interface Preset {
  id: string;
  name: string;
  mode: ModeKind;
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
