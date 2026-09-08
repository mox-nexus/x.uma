/**
 * The engine behind every live example on this site: rumi, compiled to
 * WebAssembly. The docs run the same code a user installs as `xuma-crust`.
 *
 * WebAssembly instantiates asynchronously, so callers await `ready()` once
 * before evaluating. Evaluation itself is synchronous.
 */
import init, { HttpMatcher, KvMatcher } from "xuma-crust";
import wasmUrl from "xuma-crust/xuma_crust_bg.wasm?url";
import type { EvalResult } from "./types.js";

let loading: Promise<void> | undefined;

/** Instantiate the engine. Idempotent; the first call does the work. */
export function ready(): Promise<void> {
  loading ??= init({ module_or_path: wasmUrl }).then(() => undefined);
  return loading;
}

/** Evaluate a protojson matcher config against a key-value context. */
export function evaluateKv(configJson: string, context: Record<string, string>): EvalResult {
  return attempt(() => {
    const matcher = KvMatcher.fromConfig(configJson);
    try {
      return matcher.evaluate(context);
    } finally {
      matcher.free();
    }
  });
}

/** Evaluate a protojson matcher config against an HTTP request. */
export function evaluateHttp(
  configJson: string,
  method: string,
  path: string,
  headers: Record<string, string>,
): EvalResult {
  return attempt(() => {
    const matcher = HttpMatcher.fromConfig(configJson);
    try {
      return matcher.evaluate({ method, path, headers });
    } finally {
      matcher.free();
    }
  });
}

function attempt(run: () => string | undefined): EvalResult {
  try {
    const action = run();
    return action === undefined || action === null
      ? { kind: "no-match" }
      : { kind: "match", action };
  } catch (e) {
    return toError(e);
  }
}

/** The crust raises strings; the browser raises Errors. Both become one shape. */
function toError(e: unknown): EvalResult {
  if (typeof e === "string") return { kind: "error", message: e };
  if (e instanceof SyntaxError) return { kind: "error", message: "Invalid JSON", detail: e.message };
  if (e instanceof Error) return { kind: "error", message: e.message };
  return { kind: "error", message: String(e) };
}
