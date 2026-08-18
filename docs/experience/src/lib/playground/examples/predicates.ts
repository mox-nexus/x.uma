import type { Preset } from "../types.js";

const named = (name: string) => ({
  name,
  typedConfig: { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", name },
});

const single = (key: string, valueMatch: Record<string, string>) => ({
  singlePredicate: {
    input: {
      name: key,
      typedConfig: { "@type": "type.googleapis.com/xuma.kv.v1.MapInput", key },
    },
    valueMatch,
  },
});

export const branchProtection: Preset = {
  id: "claude-branch-protect",
  name: "Protect Main",
  mode: "config",
  description:
    "Claude Code hook: block file writes when on the main branch",
  config: JSON.stringify(
    {
      matcherList: {
        matchers: [
          {
            predicate: {
              andMatcher: {
                predicate: [
                  single("event", { exact: "PreToolUse" }),
                  single("tool_name", { exact: "Write" }),
                  single("git_branch", { exact: "main" }),
                ],
              },
            },
            onMatch: { action: named("BLOCK") },
          },
        ],
      },
      onNoMatch: { action: named("ALLOW") },
    },
    null,
    2,
  ),
  context: {
    event: "PreToolUse",
    tool_name: "Write",
    git_branch: "main",
  },
};
