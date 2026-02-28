/**
 * Tests for xuma-crust HttpMatcher.
 *
 * Config-driven HTTP matching via Rust registry, exposed through wasm-bindgen.
 * Mirrors xuma-crust (Python)'s test_http_matcher.py tests.
 */

import { beforeAll, describe, expect, test } from "bun:test";
// @ts-expect-error — generated WASM package has no TS project reference
import init, { HttpMatcher } from "../pkg/xuma_crust.js";

beforeAll(async () => {
  await init();
});

// ═══════════════════════════════════════════════════════════════════════════════
// Basic matching: single input types
// ═══════════════════════════════════════════════════════════════════════════════

describe("Basic Matching", () => {
  test("path exact match", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.http.v1.PathInput", config: {} },
          value_match: { Exact: "/api/users" },
        },
        on_match: { type: "action", action: "users" },
      }],
    });
    const matcher = HttpMatcher.fromConfig(config);
    expect(matcher.evaluate({ method: "GET", path: "/api/users" })).toBe("users");
    expect(matcher.evaluate({ method: "GET", path: "/api/posts" })).toBeUndefined();
  });

  test("path prefix match", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.http.v1.PathInput", config: {} },
          value_match: { Prefix: "/api" },
        },
        on_match: { type: "action", action: "api" },
      }],
    });
    const matcher = HttpMatcher.fromConfig(config);
    expect(matcher.evaluate({ method: "GET", path: "/api/users" })).toBe("api");
    expect(matcher.evaluate({ method: "GET", path: "/health" })).toBeUndefined();
  });

  test("method exact match", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.http.v1.MethodInput", config: {} },
          value_match: { Exact: "POST" },
        },
        on_match: { type: "action", action: "post_handler" },
      }],
    });
    const matcher = HttpMatcher.fromConfig(config);
    expect(matcher.evaluate({ method: "POST", path: "/" })).toBe("post_handler");
    expect(matcher.evaluate({ method: "GET", path: "/" })).toBeUndefined();
  });

  test("header match", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.http.v1.HeaderInput", config: { name: "content-type" } },
          value_match: { Exact: "application/json" },
        },
        on_match: { type: "action", action: "json" },
      }],
    });
    const matcher = HttpMatcher.fromConfig(config);
    expect(matcher.evaluate({
      method: "POST",
      path: "/",
      headers: { "content-type": "application/json" },
    })).toBe("json");
    expect(matcher.evaluate({
      method: "POST",
      path: "/",
      headers: { "content-type": "text/html" },
    })).toBeUndefined();
  });

  test("query param match", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.http.v1.QueryParamInput", config: { name: "page" } },
          value_match: { Exact: "1" },
        },
        on_match: { type: "action", action: "first_page" },
      }],
    });
    const matcher = HttpMatcher.fromConfig(config);
    expect(matcher.evaluate({
      method: "GET",
      path: "/list",
      queryParams: { page: "1" },
    })).toBe("first_page");
    expect(matcher.evaluate({
      method: "GET",
      path: "/list",
      queryParams: { page: "2" },
    })).toBeUndefined();
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Compound predicates: AND, OR, NOT
// ═══════════════════════════════════════════════════════════════════════════════

describe("Compound Predicates", () => {
  test("AND: path + method", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "and",
          predicates: [
            {
              type: "single",
              input: { type_url: "xuma.http.v1.PathInput", config: {} },
              value_match: { Prefix: "/api" },
            },
            {
              type: "single",
              input: { type_url: "xuma.http.v1.MethodInput", config: {} },
              value_match: { Exact: "POST" },
            },
          ],
        },
        on_match: { type: "action", action: "api_write" },
      }],
    });
    const matcher = HttpMatcher.fromConfig(config);
    expect(matcher.evaluate({ method: "POST", path: "/api/users" })).toBe("api_write");
    expect(matcher.evaluate({ method: "GET", path: "/api/users" })).toBeUndefined();
    expect(matcher.evaluate({ method: "POST", path: "/health" })).toBeUndefined();
  });

  test("OR: multiple methods", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "or",
          predicates: [
            {
              type: "single",
              input: { type_url: "xuma.http.v1.MethodInput", config: {} },
              value_match: { Exact: "PUT" },
            },
            {
              type: "single",
              input: { type_url: "xuma.http.v1.MethodInput", config: {} },
              value_match: { Exact: "PATCH" },
            },
          ],
        },
        on_match: { type: "action", action: "update" },
      }],
    });
    const matcher = HttpMatcher.fromConfig(config);
    expect(matcher.evaluate({ method: "PUT", path: "/" })).toBe("update");
    expect(matcher.evaluate({ method: "PATCH", path: "/" })).toBe("update");
    expect(matcher.evaluate({ method: "GET", path: "/" })).toBeUndefined();
  });

  test("NOT: exclude path", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "not",
          predicate: {
            type: "single",
            input: { type_url: "xuma.http.v1.PathInput", config: {} },
            value_match: { Prefix: "/health" },
          },
        },
        on_match: { type: "action", action: "not_health" },
      }],
    });
    const matcher = HttpMatcher.fromConfig(config);
    expect(matcher.evaluate({ method: "GET", path: "/api/users" })).toBe("not_health");
    expect(matcher.evaluate({ method: "GET", path: "/health" })).toBeUndefined();
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Nesting and fallback
// ═══════════════════════════════════════════════════════════════════════════════

describe("Nesting & Fallback", () => {
  test("nested matcher: method then path", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.http.v1.MethodInput", config: {} },
          value_match: { Exact: "GET" },
        },
        on_match: {
          type: "matcher",
          matcher: {
            matchers: [{
              predicate: {
                type: "single",
                input: { type_url: "xuma.http.v1.PathInput", config: {} },
                value_match: { Prefix: "/api" },
              },
              on_match: { type: "action", action: "get_api" },
            }],
          },
        },
      }],
    });
    const matcher = HttpMatcher.fromConfig(config);
    expect(matcher.evaluate({ method: "GET", path: "/api/users" })).toBe("get_api");
    expect(matcher.evaluate({ method: "GET", path: "/health" })).toBeUndefined();
    expect(matcher.evaluate({ method: "POST", path: "/api/users" })).toBeUndefined();
  });

  test("on_no_match fallback", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.http.v1.PathInput", config: {} },
          value_match: { Exact: "/api" },
        },
        on_match: { type: "action", action: "api" },
      }],
      on_no_match: { type: "action", action: "default" },
    });
    const matcher = HttpMatcher.fromConfig(config);
    expect(matcher.evaluate({ method: "GET", path: "/api" })).toBe("api");
    expect(matcher.evaluate({ method: "GET", path: "/other" })).toBe("default");
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// String match types
// ═══════════════════════════════════════════════════════════════════════════════

describe("String Match Types", () => {
  const matchTypes = [
    { name: "Exact", value_match: { Exact: "/api/users" }, hit: "/api/users", miss: "/api/users/1" },
    { name: "Prefix", value_match: { Prefix: "/api" }, hit: "/api/users", miss: "/health" },
    { name: "Suffix", value_match: { Suffix: ".json" }, hit: "/data.json", miss: "/data.xml" },
    { name: "Contains", value_match: { Contains: "api" }, hit: "/api/users", miss: "/health" },
    { name: "Regex", value_match: { Regex: "^/api/v[0-9]+/" }, hit: "/api/v2/users", miss: "/api/users" },
  ];

  for (const { name, value_match, hit, miss } of matchTypes) {
    test(`${name} match`, () => {
      const config = JSON.stringify({
        matchers: [{
          predicate: {
            type: "single",
            input: { type_url: "xuma.http.v1.PathInput", config: {} },
            value_match,
          },
          on_match: { type: "action", action: "hit" },
        }],
      });
      const matcher = HttpMatcher.fromConfig(config);
      expect(matcher.evaluate({ method: "GET", path: hit })).toBe("hit");
      expect(matcher.evaluate({ method: "GET", path: miss })).toBeUndefined();
    });
  }
});

// ═══════════════════════════════════════════════════════════════════════════════
// Error handling
// ═══════════════════════════════════════════════════════════════════════════════

describe("Errors", () => {
  test("invalid JSON", () => {
    expect(() => HttpMatcher.fromConfig("not json")).toThrow(/invalid config JSON/);
  });

  test("unknown input type", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.http.v1.UnknownInput", config: {} },
          value_match: { Exact: "test" },
        },
        on_match: { type: "action", action: "test" },
      }],
    });
    expect(() => HttpMatcher.fromConfig(config)).toThrow();
  });

  test("invalid regex", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.http.v1.PathInput", config: {} },
          value_match: { Regex: "[invalid" },
        },
        on_match: { type: "action", action: "test" },
      }],
    });
    expect(() => HttpMatcher.fromConfig(config)).toThrow(/regex/);
  });

  test("missing method in evaluate", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.http.v1.PathInput", config: {} },
          value_match: { Exact: "/" },
        },
        on_match: { type: "action", action: "test" },
      }],
    });
    const matcher = HttpMatcher.fromConfig(config);
    expect(() => matcher.evaluate({ path: "/" })).toThrow(/method is required/);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Trace
// ═══════════════════════════════════════════════════════════════════════════════

describe("Trace", () => {
  test("trace result matches evaluate", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.http.v1.PathInput", config: {} },
          value_match: { Prefix: "/api" },
        },
        on_match: { type: "action", action: "api" },
      }],
      on_no_match: { type: "action", action: "default" },
    });
    const matcher = HttpMatcher.fromConfig(config);
    const ctx = { method: "GET", path: "/api/users" };
    expect(matcher.trace(ctx).result).toBe(matcher.evaluate(ctx));
  });

  test("trace has steps and structure", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.http.v1.PathInput", config: {} },
          value_match: { Exact: "/" },
        },
        on_match: { type: "action", action: "root" },
      }],
    });
    const matcher = HttpMatcher.fromConfig(config);
    const trace = matcher.trace({ method: "GET", path: "/" });
    expect(trace.steps.length).toBeGreaterThan(0);
    expect(typeof trace.steps[0].index).toBe("number");
    expect(typeof trace.steps[0].matched).toBe("boolean");
    expect(typeof trace.steps[0].predicate).toBe("string");
  });

  test("trace shows fallback usage", () => {
    const config = JSON.stringify({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.http.v1.PathInput", config: {} },
          value_match: { Exact: "/api" },
        },
        on_match: { type: "action", action: "api" },
      }],
      on_no_match: { type: "action", action: "default" },
    });
    const matcher = HttpMatcher.fromConfig(config);
    const trace = matcher.trace({ method: "GET", path: "/other" });
    expect(trace.result).toBe("default");
    expect(trace.usedFallback).toBe(true);
  });
});
