import type { MatchingValue } from "./types.ts";

/**
 * Simple DataInput for Record contexts — used in tests and examples.
 *
 * For real domains, implement your own DataInput with a typed context.
 */
export class DictInput {
	constructor(readonly key: string) {}

	get(ctx: Record<string, string>): MatchingValue {
		return ctx[this.key] ?? null;
	}
}
