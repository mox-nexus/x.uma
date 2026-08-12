import { RegistryBuilder, parseMatcherConfig } from "xuma";
import { register as registerTest } from "xuma/testing";
import {
  register as registerHttp,
  HttpRequest,
  compileRouteMatch,
} from "xuma/http";
import { Matcher, Action, FieldMatcher } from "xuma";
import type { EvalResult } from "./types.js";
import type { HttpRouteMatch } from "xuma/http";

// Build registries once (arch-guild: build once, use forever)
const testRegistry = registerTest(new RegistryBuilder()).build();
const httpRegistry = registerHttp(new RegistryBuilder<HttpRequest>()).build();

/**
 * Evaluate a MatcherConfig JSON against key-value context.
 * Uses the test domain registry (xuma.test.v1.StringInput).
 */
export function evaluateConfig(
  configJson: string,
  context: Record<string, string>,
): EvalResult {
  try {
    const parsed = JSON.parse(configJson);
    const config = parseMatcherConfig(parsed);
    const matcher = testRegistry.loadMatcher(config);
    const result = matcher.evaluate(context);
    return result !== null
      ? { kind: "match", action: result }
      : { kind: "no-match" };
  } catch (e) {
    return toError(e);
  }
}

/**
 * Evaluate HTTP route configs against method/path/headers.
 * Uses the compiler API (HttpRouteMatch → Matcher).
 */
export function evaluateHttp(
  routesJson: string,
  method: string,
  path: string,
  headers: Record<string, string>,
): EvalResult {
  try {
    const entries = JSON.parse(routesJson) as Array<{
      match: HttpRouteMatch;
      action: string;
    }>;

    if (!Array.isArray(entries)) {
      return {
        kind: "error",
        message: "Expected an array of { match, action } objects",
      };
    }

    const req = new HttpRequest(method, path, headers);

    // First-match-wins across routes
    for (const entry of entries) {
      const matcher = compileRouteMatch(entry.match, entry.action);
      const result = matcher.evaluate(req);
      if (result !== null) {
        return { kind: "match", action: result };
      }
    }

    return { kind: "no-match" };
  } catch (e) {
    return toError(e);
  }
}

/**
 * Evaluate a MatcherConfig JSON against an HTTP request.
 * Uses the HTTP domain registry (xuma.http.v1.* type URLs).
 */
export function evaluateHttpConfig(
  configJson: string,
  method: string,
  path: string,
  headers: Record<string, string>,
): EvalResult {
  try {
    const parsed = JSON.parse(configJson);
    const config = parseMatcherConfig(parsed);
    const matcher = httpRegistry.loadMatcher(config);
    const req = new HttpRequest(method, path, headers);
    const result = matcher.evaluate(req);
    return result !== null
      ? { kind: "match", action: result }
      : { kind: "no-match" };
  } catch (e) {
    return toError(e);
  }
}

function toError(e: unknown): EvalResult {
  if (e instanceof SyntaxError) {
    return {
      kind: "error",
      message: "Invalid JSON",
      detail: e.message,
    };
  }
  if (e instanceof Error) {
    return {
      kind: "error",
      message: e.constructor.name.replace(/Error$/, "") || "Error",
      detail: e.message,
    };
  }
  return { kind: "error", message: String(e) };
}
