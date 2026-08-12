import type { Preset } from "../types.js";

export const apiGateway: Preset = {
  id: "api-gateway",
  name: "API Routes",
  mode: "http",
  description: "Gateway API: route GET/POST/DELETE to different backends",
  config: JSON.stringify(
    [
      {
        match: {
          method: "GET",
          path: { type: "PathPrefix", value: "/api/" },
        },
        action: "api_read",
      },
      {
        match: {
          method: "POST",
          path: { type: "PathPrefix", value: "/api/" },
        },
        action: "api_write",
      },
      {
        match: {
          method: "DELETE",
          path: { type: "PathPrefix", value: "/api/" },
        },
        action: "api_delete",
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
    method: "GET",
    path: "/api/users",
    headers: {},
  },
};
