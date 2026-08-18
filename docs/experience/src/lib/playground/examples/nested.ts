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

export const tieredRouting: Preset = {
  id: "tiered-routing",
  name: "Tiered Routing",
  mode: "config",
  description:
    "Nested matcher: premium users route by region, free users get default",
  config: JSON.stringify(
    {
      matcherList: {
        matchers: [
          {
            predicate: single("tier", { exact: "premium" }),
            onMatch: {
              matcher: {
                matcherList: {
                  matchers: [
                    {
                      predicate: single("region", { exact: "us-east" }),
                      onMatch: { action: named("premium_us_east") },
                    },
                    {
                      predicate: single("region", { exact: "eu-west" }),
                      onMatch: { action: named("premium_eu_west") },
                    },
                  ],
                },
                onNoMatch: { action: named("premium_default") },
              },
            },
          },
        ],
      },
      onNoMatch: { action: named("free_tier") },
    },
    null,
    2,
  ),
  context: { tier: "premium", region: "us-east" },
};
