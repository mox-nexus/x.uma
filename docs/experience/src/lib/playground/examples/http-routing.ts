import type { Preset } from "../types.js";

const named = (name: string) => ({
  name,
  typedConfig: { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", name },
});

const input = (name: string, message: string) => ({
  name,
  typedConfig: { "@type": `type.googleapis.com/xuma.http.v1.${message}` },
});

const single = (name: string, message: string, valueMatch: Record<string, string>) => ({
  singlePredicate: { input: input(name, message), valueMatch },
});

const route = (method: string, action: string) => ({
  predicate: {
    andMatcher: {
      predicate: [
        single("method", "MethodInput", { exact: method }),
        single("path", "PathInput", { prefix: "/api/" }),
      ],
    },
  },
  onMatch: { action: named(action) },
});

export const apiGateway: Preset = {
  id: "api-gateway",
  name: "API Routes",
  mode: "http",
  description: "Route GET, POST and DELETE under /api/ to different backends; /health is exact",
  config: JSON.stringify(
    {
      matcherList: {
        matchers: [
          route("GET", "api_read"),
          route("POST", "api_write"),
          route("DELETE", "api_delete"),
          {
            predicate: single("path", "PathInput", { exact: "/health" }),
            onMatch: { action: named("healthcheck") },
          },
        ],
      },
    },
    null,
    2,
  ),
  context: {},
  http: {
    method: "GET",
    path: "/api/users",
    headers: {},
  },
};
