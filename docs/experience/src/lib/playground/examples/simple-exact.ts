import type { Preset } from "../types.js";

export const blockDangerousCommands: Preset = {
  id: "claude-block-rm",
  name: "Block rm -rf",
  mode: "config",
  description:
    "Claude Code hook: block dangerous Bash commands containing rm -rf",
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
                value_match: { Exact: "Bash" },
              },
              {
                type: "single",
                input: {
                  type_url: "xuma.test.v1.StringInput",
                  config: { key: "argument.command" },
                },
                value_match: { Contains: "rm -rf" },
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
    tool_name: "Bash",
    "argument.command": "rm -rf /important",
  },
};
