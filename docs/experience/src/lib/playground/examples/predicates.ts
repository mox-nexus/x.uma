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
  // Same correction as simple-exact.ts: this is the rule shape, evaluated in
  // the key-value domain. The real hook path registers xuma.claude.v1 inputs.
  description:
    "The shape of a branch-protection hook rule — block writes on main — in the key-value domain the playground evaluates.",
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
