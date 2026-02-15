import type { RegistryBuilder } from "./registry.ts";
import type { MatchingData } from "./types.ts";

/**
 * Simple DataInput for Record contexts — used in tests and examples.
 *
 * For real domains, implement your own DataInput with a typed context.
 */
export class DictInput {
	constructor(readonly key: string) {}

	get(ctx: Record<string, string>): MatchingData {
		return ctx[this.key] ?? null;
	}
}

/** Register the test domain DataInput with the registry builder. */
export function register(
	builder: RegistryBuilder<Record<string, string>>,
): RegistryBuilder<Record<string, string>> {
	builder.input("xuma.test.v1.StringInput", (config) => {
		const key = config.key;
		if (typeof key !== "string") {
			throw new Error("StringInput config requires 'key' (string)");
		}
		return new DictInput(key);
	});
	return builder;
}
