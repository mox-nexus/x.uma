import type { Preset } from "../types.js";

const named = (name: string) => ({
  name,
  typedConfig: { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", name },
});

const single = (key: string, valueMatch: Record<string, string>) => ({
  singlePredicate: {
    input: {
      name: key,
      typedConfig: { "@type": "type.googleapis.com/xuma.core.v1.MapInput", key },
    },
    valueMatch,
  },
});

export const blockDangerousCommands: Preset = {
  id: "claude-block-rm",
  name: "Block rm -rf",
  mode: "kv",
  // The rule *shape* a hook uses, in the domain the playground evaluates.
  // `rumi run claude` registers `xuma.claude.v1.*` inputs and rejects anything
  // else, so this exact config fails to load in the real hook runner; the
  // playground's engine registers `xuma.core.v1.MapInput`. The shape is the
  // transferable part: AND of three conditions, deny on match, allow on
  // no-match.
  description:
    "The shape of a Claude Code hook rule — AND three conditions, deny on match — in the key-value domain the playground evaluates. `rumi run claude` uses xuma.claude.v1 inputs.",
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
