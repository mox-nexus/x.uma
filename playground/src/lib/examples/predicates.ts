import type { Preset } from "../types.js";

export const branchProtection: Preset = {
  id: "claude-branch-protect",
  name: "Protect Main",
  mode: "config",
  description:
    "Claude Code hook: block file writes when on the main branch",
  config: JSON.stringify(
    {
      matchers: [
        {
          predicate: {
            type: "and",
            predicates: [
              {
                type: "single",
                input: {
                  type_url: "xuma.test.v1.StringInput",
                  config: { key: "event" },
                },
                value_match: { Exact: "PreToolUse" },
              },
              {
                type: "single",
                input: {
                  type_url: "xuma.test.v1.StringInput",
                  config: { key: "tool_name" },
                },
                value_match: { Exact: "Write" },
              },
              {
                type: "single",
                input: {
                  type_url: "xuma.test.v1.StringInput",
                  config: { key: "git_branch" },
                },
                value_match: { Exact: "main" },
              },
            ],
          },
          on_match: { type: "action", action: "BLOCK" },
        },
      ],
      on_no_match: { type: "action", action: "ALLOW" },
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
