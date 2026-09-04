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
  // Said "Claude Code hook" until 2026-09-01, and the config it ships fails to
  // load in the real hook runner: it uses `xuma.kv.v1.MapInput`, while
  // `rumi run claude` registers `xuma.claude.v1.*` inputs and rejects anything
  // else. Verified by running this exact config through the CLI.
  //
  // The playground cannot fix that by using the real type URLs, because it
  // evaluates in the browser through bumi, and the Claude domain is a feature
  // of rumi-core with no TypeScript implementation. So the preset says what it
  // actually is: the rule *shape* a hook uses, in the domain the playground can
  // run. The shape is the transferable part — AND of three conditions, deny on
  // match, allow on no-match.
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
