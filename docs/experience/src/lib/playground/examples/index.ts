import type { Preset } from "../types.js";
import { blockDangerousCommands } from "./simple-exact.js";
import { branchProtection } from "./predicates.js";
import { tieredRouting } from "./nested.js";
import { apiGateway } from "./http-routing.js";
import { authGateway } from "./fallback.js";

export const presets: Preset[] = [
  blockDangerousCommands,
  branchProtection,
  tieredRouting,
  apiGateway,
  authGateway,
];

export {
  blockDangerousCommands,
  branchProtection,
  tieredRouting,
  apiGateway,
  authGateway,
};
