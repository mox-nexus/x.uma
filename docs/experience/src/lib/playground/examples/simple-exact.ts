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

export const blockDangerousCommands: Preset = {
  id: "claude-block-rm",
  name: "Block rm -rf",
  mode: "config",
  description:
    "Claude Code hook: block dangerous Bash commands containing rm -rf",
  config: JSON.stringify(
    {
      matcherList: {
        matchers: [
          {
            predicate: {
              andMatcher: {
                predicate: [
                  single("event", { exact: "PreToolUse" }),
                  single("tool_name", { exact: "Bash" }),
                  single("argument.command", { contains: "rm -rf" }),
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
    tool_name: "Bash",
    "argument.command": "rm -rf /important",
  },
};
