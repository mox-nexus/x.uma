/**
 * Tests for xuma-crust TestMatcher.
 *
 * Config-driven key-value matching via Rust registry, exposed through wasm-bindgen.
 * Mirrors xuma-crust (Python)'s test_test_matcher.py tests.
 * Includes conformance fixture validation.
 */

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { beforeAll, describe, expect, test } from "bun:test";
import { pj } from "./_terse_to_protojson.ts";
// @ts-expect-error — generated WASM package has no TS project reference
import init, { TestMatcher } from "../pkg/xuma_crust.js";

beforeAll(async () => {
  await init();
});

// ═══════════════════════════════════════════════════════════════════════════════
// Basic matching: single input
// ═══════════════════════════════════════════════════════════════════════════════

describe("Basic Matching", () => {
  test("exact match", () => {
    const config = JSON.stringify(pj({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.kv.v1.MapInput", config: { key: "role" } },
          value_match: { Exact: "admin" },
        },
        on_match: { type: "action", action: "admin_route" },
      }],
    }));
    const matcher = TestMatcher.fromConfig(config);
    expect(matcher.evaluate({ role: "admin" })).toBe("admin_route");
    expect(matcher.evaluate({ role: "user" })).toBeUndefined();
  });

  test("prefix match", () => {
    const config = JSON.stringify(pj({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.kv.v1.MapInput", config: { key: "path" } },
          value_match: { Prefix: "/api" },
        },
        on_match: { type: "action", action: "api_route" },
      }],
    }));
    const matcher = TestMatcher.fromConfig(config);
    expect(matcher.evaluate({ path: "/api/users" })).toBe("api_route");
    expect(matcher.evaluate({ path: "/health" })).toBeUndefined();
  });

  test("missing key returns no match", () => {
    const config = JSON.stringify(pj({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.kv.v1.MapInput", config: { key: "role" } },
          value_match: { Exact: "admin" },
        },
        on_match: { type: "action", action: "admin_route" },
      }],
    }));
    const matcher = TestMatcher.fromConfig(config);
    expect(matcher.evaluate({ org: "acme" })).toBeUndefined();
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Compound predicates
// ═══════════════════════════════════════════════════════════════════════════════

describe("Compound Predicates", () => {
  test("AND: role + org", () => {
    const config = JSON.stringify(pj({
      matchers: [{
        predicate: {
          type: "and",
          predicates: [
            {
              type: "single",
              input: { type_url: "xuma.kv.v1.MapInput", config: { key: "role" } },
              value_match: { Exact: "admin" },
            },
            {
              type: "single",
              input: { type_url: "xuma.kv.v1.MapInput", config: { key: "org" } },
              value_match: { Exact: "acme" },
            },
          ],
        },
        on_match: { type: "action", action: "acme_admin" },
      }],
    }));
    const matcher = TestMatcher.fromConfig(config);
    expect(matcher.evaluate({ role: "admin", org: "acme" })).toBe("acme_admin");
    expect(matcher.evaluate({ role: "admin", org: "other" })).toBeUndefined();
    expect(matcher.evaluate({ role: "user", org: "acme" })).toBeUndefined();
  });

  test("OR: multiple roles", () => {
    const config = JSON.stringify(pj({
      matchers: [{
        predicate: {
          type: "or",
          predicates: [
            {
              type: "single",
              input: { type_url: "xuma.kv.v1.MapInput", config: { key: "role" } },
              value_match: { Exact: "admin" },
            },
            {
              type: "single",
              input: { type_url: "xuma.kv.v1.MapInput", config: { key: "role" } },
              value_match: { Exact: "superadmin" },
            },
          ],
        },
        on_match: { type: "action", action: "elevated" },
      }],
    }));
    const matcher = TestMatcher.fromConfig(config);
    expect(matcher.evaluate({ role: "admin" })).toBe("elevated");
    expect(matcher.evaluate({ role: "superadmin" })).toBe("elevated");
    expect(matcher.evaluate({ role: "user" })).toBeUndefined();
  });

  test("NOT: exclude role", () => {
    const config = JSON.stringify(pj({
      matchers: [{
        predicate: {
          type: "not",
          predicate: {
            type: "single",
            input: { type_url: "xuma.kv.v1.MapInput", config: { key: "role" } },
            value_match: { Exact: "guest" },
          },
        },
        on_match: { type: "action", action: "authenticated" },
      }],
    }));
    const matcher = TestMatcher.fromConfig(config);
    expect(matcher.evaluate({ role: "admin" })).toBe("authenticated");
    expect(matcher.evaluate({ role: "guest" })).toBeUndefined();
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Nesting and fallback
// ═══════════════════════════════════════════════════════════════════════════════

describe("Nesting & Fallback", () => {
  test("nested matcher: role then org", () => {
    const config = JSON.stringify(pj({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.kv.v1.MapInput", config: { key: "role" } },
          value_match: { Exact: "admin" },
        },
        on_match: {
          type: "matcher",
          matcher: {
            matchers: [{
              predicate: {
                type: "single",
                input: { type_url: "xuma.kv.v1.MapInput", config: { key: "org" } },
                value_match: { Exact: "acme" },
              },
              on_match: { type: "action", action: "acme_admin" },
            }],
          },
        },
      }],
    }));
    const matcher = TestMatcher.fromConfig(config);
    expect(matcher.evaluate({ role: "admin", org: "acme" })).toBe("acme_admin");
    expect(matcher.evaluate({ role: "admin", org: "other" })).toBeUndefined();
    expect(matcher.evaluate({ role: "user", org: "acme" })).toBeUndefined();
  });

  test("on_no_match fallback", () => {
    const config = JSON.stringify(pj({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.kv.v1.MapInput", config: { key: "role" } },
          value_match: { Exact: "admin" },
        },
        on_match: { type: "action", action: "admin" },
      }],
      on_no_match: { type: "action", action: "default" },
    }));
    const matcher = TestMatcher.fromConfig(config);
    expect(matcher.evaluate({ role: "admin" })).toBe("admin");
    expect(matcher.evaluate({ role: "user" })).toBe("default");
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Conformance fixtures
// ═══════════════════════════════════════════════════════════════════════════════

describe("Conformance Fixtures", () => {
  const fixtureDir = join(__dirname, "../../../../spec/tests/07_protojson");
  const fixtureFiles = readdirSync(fixtureDir).filter(f => f.endsWith(".yaml")).sort();

  // Not an exact count: that has to be bumped on every fixture added, which is
  // noise rather than a guard. What matters here is that the runner is not
  // inert — a loop over zero files passes and proves nothing. Under-coverage is
  // caught by the fixture-coverage check, which knows the schema.
  test("the fixture corpus is not empty", () => {
    expect(fixtureFiles.length).toBeGreaterThan(0);
  });

  for (const file of fixtureFiles) {
    test(`fixture: ${file}`, () => {
      const yaml = readFileSync(join(fixtureDir, file), "utf-8");
      const results = TestMatcher.runFixtures(yaml);

      for (const result of results) {
        expect(result.passed).toBe(true);
      }
    });
  }
});

// ═══════════════════════════════════════════════════════════════════════════════
// Error handling
// ═══════════════════════════════════════════════════════════════════════════════

describe("Errors", () => {
  test("invalid JSON", () => {
    expect(() => TestMatcher.fromConfig("not json")).toThrow(/invalid config JSON/);
  });

  test("unknown input type", () => {
    const config = JSON.stringify(pj({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.unknown.v1.Input", config: {} },
          value_match: { Exact: "test" },
        },
        on_match: { type: "action", action: "test" },
      }],
    }));
    expect(() => TestMatcher.fromConfig(config)).toThrow();
  });

  test("invalid regex", () => {
    const config = JSON.stringify(pj({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.kv.v1.MapInput", config: { key: "x" } },
          value_match: { Regex: "[invalid" },
        },
        on_match: { type: "action", action: "test" },
      }],
    }));
    expect(() => TestMatcher.fromConfig(config)).toThrow(/regex/);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Trace
// ═══════════════════════════════════════════════════════════════════════════════

describe("Trace", () => {
  test("trace result matches evaluate", () => {
    const config = JSON.stringify(pj({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.kv.v1.MapInput", config: { key: "role" } },
          value_match: { Exact: "admin" },
        },
        on_match: { type: "action", action: "admin" },
      }],
      on_no_match: { type: "action", action: "default" },
    }));
    const matcher = TestMatcher.fromConfig(config);
    const ctx = { role: "admin" };
    expect(matcher.trace(ctx).result).toBe(matcher.evaluate(ctx));
  });

  test("trace has steps", () => {
    const config = JSON.stringify(pj({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.kv.v1.MapInput", config: { key: "role" } },
          value_match: { Exact: "admin" },
        },
        on_match: { type: "action", action: "admin" },
      }],
    }));
    const matcher = TestMatcher.fromConfig(config);
    const trace = matcher.trace({ role: "admin" });
    expect(trace.steps.length).toBeGreaterThan(0);
    expect(typeof trace.steps[0].index).toBe("number");
    expect(typeof trace.steps[0].matched).toBe("boolean");
    expect(typeof trace.steps[0].predicate).toBe("string");
  });

  test("trace shows fallback usage", () => {
    const config = JSON.stringify(pj({
      matchers: [{
        predicate: {
          type: "single",
          input: { type_url: "xuma.kv.v1.MapInput", config: { key: "role" } },
          value_match: { Exact: "admin" },
        },
        on_match: { type: "action", action: "admin" },
      }],
      on_no_match: { type: "action", action: "default" },
    }));
    const matcher = TestMatcher.fromConfig(config);
    const trace = matcher.trace({ role: "user" });
    expect(trace.result).toBe("default");
    expect(trace.usedFallback).toBe(true);
  });
});
