/**
 * Resource limits, mirroring rumi core's constants.
 *
 * These live in their own module rather than in `registry.ts` so that
 * `string-matchers.ts` can enforce them in its constructors without importing
 * the registry, which imports it. The limits belong to the types that hold the
 * resource, not to the loader.
 *
 * `registry.ts` re-exports every name here, so existing imports keep working.
 */

import { MatcherError } from "./matcher.ts";

/** Maximum field matchers in one matcher list. */
export const MAX_FIELD_MATCHERS = 256;

/**
 * Maximum entries in a single MatcherTree.
 *
 * Deliberately not MAX_FIELD_MATCHERS, and for a different reason. That limit
 * is about evaluation: a list is O(n) per request, so its width is a
 * per-request cost. A tree is a keyed lookup, so entry count costs nothing at
 * evaluation time — and large routing tables are the entire reason to reach
 * for one. What a tree's width costs is memory at config load.
 */
export const MAX_TREE_ENTRIES = 65_536;

/** Maximum predicates inside one compound predicate. */
export const MAX_PREDICATES_PER_COMPOUND = 256;

/** Maximum length of a literal match pattern (8 KB). */
export const MAX_PATTERN_LENGTH = 8192;

/**
 * Maximum length of a regex pattern (4 KB).
 *
 * Length alone does not bound compile cost — see `regex-budget.ts`, which
 * bounds the axis that actually drives it.
 */
export const MAX_REGEX_PATTERN_LENGTH = 4096;

/** A match pattern exceeds the length limit. */
export class PatternTooLongError extends MatcherError {
	readonly length: number;
	readonly max: number;

	constructor(length: number, max: number) {
		super(`pattern length ${length} exceeds maximum ${max}`);
		this.name = "PatternTooLongError";
		this.length = length;
		this.max = max;
	}
}
