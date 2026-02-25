import type { Preset } from "../types.js";

export const authGateway: Preset = {
  id: "auth-gateway",
  name: "Auth Gateway",
  mode: "http",
  description:
    "Require Bearer token and JSON content-type on /api paths",
  config: JSON.stringify(
    [
      {
        match: {
          method: "POST",
          path: { type: "PathPrefix", value: "/api/" },
          headers: [
            {
              type: "RegularExpression",
              name: "authorization",
              value: "^Bearer .+$",
            },
            {
              type: "Exact",
              name: "content-type",
              value: "application/json",
            },
          ],
        },
        action: "authorized",
      },
      {
        match: {
          path: { type: "PathPrefix", value: "/api/" },
        },
        action: "unauthorized",
      },
      {
        match: {
          path: { type: "Exact", value: "/health" },
        },
        action: "healthcheck",
      },
    ],
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
