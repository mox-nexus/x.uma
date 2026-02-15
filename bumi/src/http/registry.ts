/**
 * HTTP domain registration for the registry.
 *
 * Registers PathInput, MethodInput, HeaderInput, QueryParamInput
 * with their xuma type URLs.
 */

import type { RegistryBuilder } from "../registry.ts";
import { HeaderInput, MethodInput, PathInput, QueryParamInput } from "./inputs.ts";
import type { HttpRequest } from "./request.ts";

/** Register all HTTP domain DataInputs with the registry builder. */
export function register(builder: RegistryBuilder<HttpRequest>): RegistryBuilder<HttpRequest> {
	builder
		.input("xuma.http.v1.PathInput", () => new PathInput())
		.input("xuma.http.v1.MethodInput", () => new MethodInput())
		.input("xuma.http.v1.HeaderInput", (config) => {
			const name = config.name;
			if (typeof name !== "string") {
				throw new Error("HeaderInput config requires 'name' (string)");
			}
			return new HeaderInput(name);
		})
		.input("xuma.http.v1.QueryParamInput", (config) => {
			const name = config.name;
			if (typeof name !== "string") {
				throw new Error("QueryParamInput config requires 'name' (string)");
			}
			return new QueryParamInput(name);
		});
	return builder;
}
