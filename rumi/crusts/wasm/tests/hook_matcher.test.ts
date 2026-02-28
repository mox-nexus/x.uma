/**
 * Tests for xuma-crust HookMatcher.
 *
 * Mirrors xuma-crust (Python)'s 37 pytest tests:
 * - Conformance: same input → same result as Rust rumi::claude
 * - Security: fail-closed, input limits, invalid regex
 * - Trace: debugging visibility
 * - DX: StringMatch factories, bare string convenience
 * - Session semantics: preserve semantic distinctions
 */

import { beforeAll, describe, expect, test } from "bun:test";
// @ts-expect-error — generated WASM package has no TS project reference
import init, { HookMatcher, StringMatch } from "../pkg/xuma_crust.js";

beforeAll(async () => {
  await init();
});

// ═══════════════════════════════════════════════════════════════════════════════
// Conformance: must produce identical results to rumi::claude Rust tests
// ═══════════════════════════════════════════════════════════════════════════════

describe("Conformance", () => {
  test("event match", () => {
    const matcher = HookMatcher.compile(
      [{ event: "PreToolUse" }],
      "matched",
    );
    expect(matcher.evaluate({ event: "PreToolUse" })).toBe("matched");
    expect(matcher.evaluate({ event: "PostToolUse" })).toBeUndefined();
  });

  test("tool exact match (bare string)", () => {
    const matcher = HookMatcher.compile(
      [{ toolName: "Bash" }],
      "matched",
    );
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "Bash" })).toBe("matched");
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "Write" })).toBeUndefined();
  });

  test("tool prefix match", () => {
    const matcher = HookMatcher.compile(
      [{ toolName: StringMatch.prefix("mcp__") }],
      "matched",
    );
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "mcp__db__delete" })).toBe("matched");
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "Write" })).toBeUndefined();
  });

  test("tool regex match", () => {
    const matcher = HookMatcher.compile(
      [{ toolName: StringMatch.regex("^(Write|Edit)$") }],
      "matched",
    );
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "Write" })).toBe("matched");
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "Edit" })).toBe("matched");
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "Bash" })).toBeUndefined();
  });

  test("argument contains match", () => {
    const matcher = HookMatcher.compile(
      [{ arguments: [["command", StringMatch.contains("rm -rf")]] }],
      "blocked",
    );
    const result = matcher.evaluate({
      event: "PreToolUse",
      toolName: "Bash",
      arguments: { command: "sudo rm -rf /" },
    });
    expect(result).toBe("blocked");
  });

  test("argument missing returns no match", () => {
    const matcher = HookMatcher.compile(
      [{ arguments: [["command", StringMatch.contains("rm")]] }],
      "blocked",
    );
    // No arguments provided
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "Bash" })).toBeUndefined();
  });

  test("combined event and tool", () => {
    const matcher = HookMatcher.compile(
      [{ event: "PreToolUse", toolName: "Bash" }],
      "matched",
    );
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "Bash" })).toBe("matched");
    // Wrong event
    expect(matcher.evaluate({ event: "PostToolUse", toolName: "Bash" })).toBeUndefined();
    // Wrong tool
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "Write" })).toBeUndefined();
  });

  test("cwd prefix match", () => {
    const matcher = HookMatcher.compile(
      [{ cwd: StringMatch.prefix("/home/user") }],
      "matched",
    );
    expect(matcher.evaluate({ event: "PreToolUse", cwd: "/home/user/project" })).toBe("matched");
    expect(matcher.evaluate({ event: "PreToolUse", cwd: "/tmp" })).toBeUndefined();
  });

  test("git branch match", () => {
    const matcher = HookMatcher.compile(
      [{ gitBranch: "main" }],
      "matched",
    );
    expect(matcher.evaluate({ event: "PreToolUse", gitBranch: "main" })).toBe("matched");
    expect(matcher.evaluate({ event: "PreToolUse", gitBranch: "dev" })).toBeUndefined();
    // No branch
    expect(matcher.evaluate({ event: "PreToolUse" })).toBeUndefined();
  });

  test("combined all fields", () => {
    const matcher = HookMatcher.compile(
      [{
        event: "PreToolUse",
        toolName: "Bash",
        arguments: [["command", StringMatch.contains("rm")]],
        cwd: StringMatch.prefix("/home"),
        gitBranch: "main",
      }],
      "blocked",
    );
    const result = matcher.evaluate({
      event: "PreToolUse",
      toolName: "Bash",
      arguments: { command: "rm -rf /" },
      cwd: "/home/user",
      gitBranch: "main",
    });
    expect(result).toBe("blocked");
  });

  test("multiple rules OR", () => {
    const matcher = HookMatcher.compile(
      [
        { toolName: "Bash" },
        { toolName: "Write" },
      ],
      "matched",
    );
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "Bash" })).toBe("matched");
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "Write" })).toBe("matched");
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "Read" })).toBeUndefined();
  });

  test("fallback action", () => {
    const matcher = HookMatcher.compile(
      [{ toolName: "Bash" }],
      "deny",
      "allow",
    );
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "Bash" })).toBe("deny");
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "Write" })).toBe("allow");
  });

  test("match all explicit", () => {
    const matcher = HookMatcher.compile(
      [{ matchAll: true }],
      "catch_all",
    );
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "anything" })).toBe("catch_all");
    expect(matcher.evaluate({ event: "Stop" })).toBe("catch_all");
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Scenarios: real-world Claude Code hook patterns
// ═══════════════════════════════════════════════════════════════════════════════

describe("Scenarios", () => {
  test("block dangerous bash", () => {
    const matcher = HookMatcher.compile(
      [{
        event: "PreToolUse",
        toolName: "Bash",
        arguments: [["command", StringMatch.contains("rm -rf")]],
      }],
      "deny",
      "allow",
    );
    // Dangerous
    expect(matcher.evaluate({
      event: "PreToolUse",
      toolName: "Bash",
      arguments: { command: "rm -rf /important" },
    })).toBe("deny");
    // Safe
    expect(matcher.evaluate({
      event: "PreToolUse",
      toolName: "Bash",
      arguments: { command: "ls -la" },
    })).toBe("allow");
  });

  test("block MCP deletes", () => {
    const matcher = HookMatcher.compile(
      [{ toolName: StringMatch.regex("^mcp__.*__delete") }],
      "deny",
      "allow",
    );
    expect(matcher.evaluate({
      event: "PreToolUse",
      toolName: "mcp__db__delete_row",
    })).toBe("deny");
    expect(matcher.evaluate({
      event: "PreToolUse",
      toolName: "mcp__db__read_row",
    })).toBe("allow");
  });

  test("branch protection", () => {
    const matcher = HookMatcher.compile(
      [{
        event: "PreToolUse",
        toolName: StringMatch.regex("^(Write|Edit)$"),
        gitBranch: "main",
      }],
      "deny",
      "allow",
    );
    // On main: blocked
    expect(matcher.evaluate({
      event: "PreToolUse",
      toolName: "Write",
      gitBranch: "main",
    })).toBe("deny");
    // On feature branch: allowed
    expect(matcher.evaluate({
      event: "PreToolUse",
      toolName: "Write",
      gitBranch: "feat/test",
    })).toBe("allow");
  });

  test("all hook events", () => {
    const events = [
      "PreToolUse", "PostToolUse", "Stop", "SubagentStop",
      "UserPromptSubmit", "SessionStart", "SessionEnd",
      "PreCompact", "Notification",
    ];
    for (const event of events) {
      const matcher = HookMatcher.compile(
        [{ event }],
        "matched",
      );
      expect(matcher.evaluate({ event })).toBe("matched");
    }
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Security: fail-closed, input limits, error handling (Vector requirements)
// ═══════════════════════════════════════════════════════════════════════════════

describe("Security", () => {
  test("empty match rejected (V-BYPASS-1)", () => {
    expect(() => HookMatcher.compile([{}], "blocked")).toThrow(/empty HookMatch/);
  });

  test("empty match allowed with matchAll", () => {
    const matcher = HookMatcher.compile(
      [{ matchAll: true }],
      "allow",
    );
    expect(matcher.evaluate({ event: "PreToolUse" })).toBe("allow");
  });

  test("invalid regex raises", () => {
    expect(() =>
      HookMatcher.compile(
        [{ toolName: StringMatch.regex("[invalid") }],
        "blocked",
      )
    ).toThrow(/regex/);
  });

  test("unknown event in evaluate raises", () => {
    const matcher = HookMatcher.compile(
      [{ event: "PreToolUse" }],
      "test",
    );
    expect(() => matcher.evaluate({ event: "NotAnEvent" })).toThrow(/unknown hook event/);
  });

  test("unknown event in config raises", () => {
    expect(() =>
      HookMatcher.compile(
        [{ event: "FakeEvent" }],
        "test",
      )
    ).toThrow(/unknown hook event/);
  });

  test("too many rules raises", () => {
    const rules = Array.from({ length: 257 }, () => ({ event: "PreToolUse" }));
    expect(() => HookMatcher.compile(rules, "blocked")).toThrow(/too many rules/);
  });

  test("too many arguments raises", () => {
    const args = Array.from({ length: 65 }, (_, i) => [
      `arg${i}`,
      StringMatch.exact("v"),
    ]);
    expect(() =>
      HookMatcher.compile(
        [{ event: "PreToolUse", arguments: args }],
        "blocked",
      )
    ).toThrow(/too many arguments/);
  });

  test("oversized pattern raises", () => {
    const big = "a".repeat(8193);
    expect(() =>
      HookMatcher.compile(
        [{ toolName: StringMatch.exact(big) }],
        "blocked",
      )
    ).toThrow(/exceeds limit/);
  });

  test("oversized regex raises", () => {
    const big = "a".repeat(4097);
    expect(() =>
      HookMatcher.compile(
        [{ toolName: StringMatch.regex(big) }],
        "blocked",
      )
    ).toThrow(/exceeds limit/);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Trace: debugging visibility (opt-in per Vector)
// ═══════════════════════════════════════════════════════════════════════════════

describe("Trace", () => {
  test("trace result matches evaluate", () => {
    const matcher = HookMatcher.compile(
      [{ event: "PreToolUse", toolName: "Bash" }],
      "block",
      "allow",
    );
    const ctx = { event: "PreToolUse", toolName: "Bash" };
    const evalResult = matcher.evaluate(ctx);
    const trace = matcher.trace(ctx);
    expect(trace.result).toBe(evalResult);
  });

  test("trace has steps", () => {
    const matcher = HookMatcher.compile(
      [{ event: "PreToolUse", toolName: "Bash" }],
      "block",
    );
    const trace = matcher.trace({ event: "PreToolUse", toolName: "Bash" });
    expect(trace.steps.length).toBeGreaterThan(0);
    expect(trace.steps[0].matched).toBe(true);
  });

  test("trace no match uses fallback", () => {
    const matcher = HookMatcher.compile(
      [{ toolName: "Bash" }],
      "block",
      "allow",
    );
    const trace = matcher.trace({ event: "PreToolUse", toolName: "Write" });
    expect(trace.result).toBe("allow");
    expect(trace.usedFallback).toBe(true);
  });

  test("trace step structure", () => {
    const matcher = HookMatcher.compile(
      [{ event: "PreToolUse" }],
      "matched",
    );
    const trace = matcher.trace({ event: "PreToolUse" });
    const step = trace.steps[0];
    expect(typeof step.index).toBe("number");
    expect(typeof step.matched).toBe("boolean");
    expect(typeof step.predicate).toBe("string");
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// DX: convenience features (Ace recommendations)
// ═══════════════════════════════════════════════════════════════════════════════

describe("Developer Experience", () => {
  test("bare string is exact match", () => {
    const matcher = HookMatcher.compile(
      [{ toolName: "Bash" }],
      "matched",
    );
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "Bash" })).toBe("matched");
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "BashScript" })).toBeUndefined();
  });

  test("StringMatch factory methods", () => {
    const factories = [
      () => StringMatch.exact("Bash"),
      () => StringMatch.prefix("mcp__"),
      () => StringMatch.suffix(".rs"),
      () => StringMatch.contains("rm"),
      () => StringMatch.regex("^Bash$"),
    ];
    for (const factory of factories) {
      const sm = factory();
      // Verify it's usable in compile
      HookMatcher.compile([{ toolName: sm }], "test");
    }
  });

  test("StringMatch returns discriminated objects", () => {
    expect(StringMatch.exact("Bash")).toEqual({ type: "exact", value: "Bash" });
    expect(StringMatch.prefix("mcp__")).toEqual({ type: "prefix", value: "mcp__" });
    expect(StringMatch.suffix(".rs")).toEqual({ type: "suffix", value: ".rs" });
    expect(StringMatch.contains("rm")).toEqual({ type: "contains", value: "rm" });
    expect(StringMatch.regex("^Bash$")).toEqual({ type: "regex", pattern: "^Bash$" });
  });

  test("raw discriminated objects work without factories", () => {
    // Users can skip StringMatch and pass plain objects directly
    const matcher = HookMatcher.compile(
      [{ toolName: { type: "prefix", value: "mcp__" } }],
      "matched",
    );
    expect(matcher.evaluate({ event: "PreToolUse", toolName: "mcp__db__delete" })).toBe("matched");
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Session semantics (Dijkstra: preserve session_id="" vs git_branch=undefined)
// ═══════════════════════════════════════════════════════════════════════════════

describe("Session Semantics", () => {
  test("session_id empty string matches", () => {
    const matcher = HookMatcher.compile(
      [{ sessionId: StringMatch.exact("") }],
      "matched",
    );
    // Empty string should match
    expect(matcher.evaluate({ event: "PreToolUse", sessionId: "" })).toBe("matched");
  });

  test("git branch undefined vs present", () => {
    const matcher = HookMatcher.compile(
      [{ gitBranch: "main" }],
      "matched",
    );
    // Present and matching
    expect(matcher.evaluate({ event: "PreToolUse", gitBranch: "main" })).toBe("matched");
    // Absent (not in a git repo)
    expect(matcher.evaluate({ event: "PreToolUse" })).toBeUndefined();
  });
});
