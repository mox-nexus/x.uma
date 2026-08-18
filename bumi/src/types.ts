/**
 * The erased data type — TypeScript's union replaces Rust's MatchingData enum.
 * `null` maps to MatchingData::None / Python's None.
 */
export type MatchingData = string | number | boolean | Uint8Array | null;

/**
 * Extract a value from a domain-specific context.
 *
 * Generic over the context type (`Ctx`). Returning `null` signals
 * "data not available" — the predicate evaluates to `false`.
 */
export interface DataInput<Ctx> {
	get(ctx: Ctx): MatchingData;

	/**
	 * The kind of value `get` returns: "string", "int", "bool", "bytes".
	 *
	 * Checked against the matcher's `supportedTypes` at load time, so a config
	 * pairing a string input with a boolean matcher is a load error rather than
	 * a rule that silently never fires.
	 *
	 * Optional; absent means "string", matching rumi's default.
	 */
	dataType?(): string;
}

/**
 * Match against a type-erased value.
 *
 * Intentionally non-generic — the same ExactMatcher works for HTTP,
 * test contexts, Claude hooks, etc. Type erasure at the data level,
 * not the predicate level.
 */
export interface InputMatcher {
	matches(value: MatchingData): boolean;

	/**
	 * The value kinds this matcher can compare.
	 *
	 * Optional; absent means ["string"], matching rumi's default. See
	 * `DataInput.dataType`.
	 */
	supportedTypes?(): readonly string[];
}
