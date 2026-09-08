import type { Preset } from "../types.js";

const named = (name: string) => ({
  name,
  typedConfig: { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", name },
});

const input = (name: string, message: string, config: Record<string, string> = {}) => ({
  name,
  typedConfig: { "@type": `type.googleapis.com/xuma.http.v1.${message}`, ...config },
});

export const authGateway: Preset = {
  id: "auth-gateway",
  name: "Auth Gateway",
  mode: "http",
  description:
    "Require a Bearer token and a JSON content-type on POST /api/; everything else under /api/ is unauthorized",
  config: JSON.stringify(
    {
      matcherList: {
        matchers: [
          {
            predicate: {
              andMatcher: {
                predicate: [
                  {
                    singlePredicate: {
                      input: input("method", "MethodInput"),
                      valueMatch: { exact: "POST" },
                    },
                  },
                  {
                    singlePredicate: {
                      input: input("path", "PathInput"),
                      valueMatch: { prefix: "/api/" },
                    },
                  },
                  {
                    singlePredicate: {
                      input: input("authorization", "HeaderInput", { name: "authorization" }),
                      valueMatch: { safeRegex: { regex: "^Bearer .+$" } },
                    },
                  },
                  {
                    singlePredicate: {
                      input: input("content-type", "HeaderInput", { name: "content-type" }),
                      valueMatch: { exact: "application/json" },
                    },
                  },
                ],
              },
            },
            onMatch: { action: named("authorized") },
          },
          {
            predicate: {
              singlePredicate: {
                input: input("path", "PathInput"),
                valueMatch: { prefix: "/api/" },
              },
            },
            onMatch: { action: named("unauthorized") },
          },
          {
            predicate: {
              singlePredicate: {
                input: input("path", "PathInput"),
                valueMatch: { exact: "/health" },
              },
            },
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
    method: "POST",
    path: "/api/users",
    headers: {
      authorization: "Bearer eyJhbGciOiJIUzI1NiJ9.test",
      "content-type": "application/json",
    },
  },
};
