/**
 * The erased data type — TypeScript's union replaces Rust's MatchingData enum.
 * `null` maps to MatchingData::None / Python's None.
 */
export type MatchingValue = string | number | boolean | Uint8Array | null;

/**
 * Extract a value from a domain-specific context.
 *
 * Generic over the context type (`Ctx`). Returning `null` signals
 * "data not available" — the predicate evaluates to `false`.
 */
export interface DataInput<Ctx> {
	get(ctx: Ctx): MatchingValue;
}

/**
 * Match against a type-erased value.
 *
 * Intentionally non-generic — the same ExactMatcher works for HTTP,
 * test contexts, Claude hooks, etc. Type erasure at the data level,
 * not the predicate level.
 */
export interface InputMatcher {
	matches(value: MatchingValue): boolean;
}
