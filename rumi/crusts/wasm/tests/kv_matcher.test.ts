/**
 * Tests for xuma-crust KvMatcher.
 *
 * Config-driven key-value matching via Rust registry, exposed through wasm-bindgen.
 * Mirrors xuma-crust (Python)'s test_kv_matcher.py tests.
 * Includes conformance fixture validation.
 */

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { beforeAll, describe, expect, test } from "bun:test";
import { pj } from "./_terse_to_protojson.ts";
// @ts-expect-error — generated WASM package has no TS project reference
import init, { KvMatcher } from "../pkg/xuma_crust.js";

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
    const matcher = KvMatcher.fromConfig(config);
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
    const matcher = KvMatcher.fromConfig(config);
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
    const matcher = KvMatcher.fromConfig(config);
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
    const matcher = KvMatcher.fromConfig(config);
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
    const matcher = KvMatcher.fromConfig(config);
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
    const matcher = KvMatcher.fromConfig(config);
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
    const matcher = KvMatcher.fromConfig(config);
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
    const matcher = KvMatcher.fromConfig(config);
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
      const results = KvMatcher.runFixtures(yaml);

      for (const result of results) {
        expect(result.passed).toBe(true);
      }
    });
  }

  // The ledger holds in both directions: a fixture this binding is not listed
  // for, but can load, is a stale exception and fails.
  test("a fixture that omits this binding must not load here", () => {
    const omitsTypescript = `
name: omits_typescript
implementations: [rust]
proto_matcher:
  matcherList:
    matchers:
      - predicate:
          singlePredicate:
            input:
              name: k
              typedConfig: {"@type": "type.googleapis.com/xuma.kv.v1.MapInput", key: k}
            valueMatch: {exact: v}
        onMatch:
          action:
            name: hit
            typedConfig: {"@type": "type.googleapis.com/xuma.core.v1.NamedAction", name: hit}
cases: []
`;
    const results = KvMatcher.runFixtures(omitsTypescript);
    expect(results).toEqual([
      {
        fixture: "omits_typescript",
        caseName: "ledger",
        passed: false,
        detail: "not listed in `implementations`, but it loads here — add this binding to the ledger",
      },
    ]);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Error handling
// ═══════════════════════════════════════════════════════════════════════════════

describe("Errors", () => {
  test("invalid JSON", () => {
    expect(() => KvMatcher.fromConfig("not json")).toThrow(/invalid config JSON/);
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
    expect(() => KvMatcher.fromConfig(config)).toThrow();
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
    expect(() => KvMatcher.fromConfig(config)).toThrow(/regex/);
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
    const matcher = KvMatcher.fromConfig(config);
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
    const matcher = KvMatcher.fromConfig(config);
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
    const matcher = KvMatcher.fromConfig(config);
    const trace = matcher.trace({ role: "user" });
    expect(trace.result).toBe("default");
    expect(trace.usedFallback).toBe(true);
  });
});


// ═══════════════════════════════════════════════════════════════════════════════
// The decision object, the action table, and metadata (D-056)
// ═══════════════════════════════════════════════════════════════════════════════

function named(name: string, metadata?: Record<string, string>) {
  const action: Record<string, unknown> = {
    "@type": "type.googleapis.com/xuma.core.v1.NamedAction",
    name,
  };
  if (metadata) action.metadata = metadata;
  return { action: { name, typedConfig: action } };
}

function rule(key: string, value: string, onMatch: unknown) {
  return {
    predicate: {
      singlePredicate: {
        input: { name: key, typedConfig: { "@type": "type.googleapis.com/xuma.kv.v1.MapInput", key } },
        valueMatch: { exact: value },
      },
    },
    onMatch,
  };
}

const TWO_RULES_AND_A_FALLBACK = JSON.stringify({
  matcherList: {
    matchers: [
      rule("role", "admin", named("allow", { processor: "audit" })),
      rule("role", "viewer", named("read_only")),
    ],
  },
  onNoMatch: named("deny"),
});

describe("Decision", () => {
  test("a hit is a decision with its index", () => {
    const matcher = KvMatcher.fromConfig(TWO_RULES_AND_A_FALLBACK);
    expect(matcher.decide({ role: "viewer" })).toEqual({
      matched: true,
      action: "read_only",
      index: 1,
      fallback: false,
    });
  });

  test("the fallback says so", () => {
    const matcher = KvMatcher.fromConfig(TWO_RULES_AND_A_FALLBACK);
    expect(matcher.decide({ role: "auditor" })).toEqual({
      matched: true,
      action: "deny",
      index: 2,
      fallback: true,
    });
  });

  test("no decision at all", () => {
    const matcher = KvMatcher.fromConfig(
      JSON.stringify({ matcherList: { matchers: [rule("role", "admin", named("allow"))] } }),
    );
    expect(matcher.decide({ role: "nobody" })).toEqual({
      matched: false,
      action: undefined,
      index: undefined,
      fallback: false,
    });
  });

  test("evaluate and decide agree", () => {
    const matcher = KvMatcher.fromConfig(TWO_RULES_AND_A_FALLBACK);
    for (const ctx of [{ role: "admin" }, { role: "viewer" }, { role: "x" }, {}]) {
      expect(matcher.decide(ctx).action).toBe(matcher.evaluate(ctx));
    }
  });
});

describe("Action table", () => {
  test("actions are listed in document order with the fallback last", () => {
    const matcher = KvMatcher.fromConfig(TWO_RULES_AND_A_FALLBACK);
    expect(matcher.actions()).toEqual(["allow", "read_only", "deny"]);
  });

  test("metadata is carried, not dropped", () => {
    const matcher = KvMatcher.fromConfig(TWO_RULES_AND_A_FALLBACK);
    expect(matcher.actionMetadata(0)).toEqual({ processor: "audit" });
    expect(matcher.actionMetadata(1)).toEqual({});
  });

  test("metadata out of range is an error", () => {
    const matcher = KvMatcher.fromConfig(TWO_RULES_AND_A_FALLBACK);
    expect(() => matcher.actionMetadata(3)).toThrow("no action at index 3");
  });

  test("the trace carries the index", () => {
    const matcher = KvMatcher.fromConfig(TWO_RULES_AND_A_FALLBACK);
    const trace = matcher.trace({ role: "admin" });
    expect(trace.index).toBe(0);
    expect(matcher.actions()[trace.index]).toBe(trace.result);
    expect(matcher.trace({ role: "nope" }).index).toBe(2);
  });
});


describe("Document budget", () => {
  test("a document over a mebibyte is refused by its size", () => {
    const padding = "x".repeat(1 << 20);
    const document = JSON.stringify({ matcherList: { matchers: [] }, pad: padding });
    expect(() => KvMatcher.fromConfig(document)).toThrow("bytes");
  });
});
