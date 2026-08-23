/**
 * Error types, at the bottom of the module graph.
 *
 * They used to live wherever they were first thrown — `MatcherError` in
 * `matcher.ts`, the width errors in `registry.ts` — which meant `limits.ts`
 * imported `matcher.ts`, and `matcher.ts` could not import `limits.ts` back to
 * enforce a width without a cycle. That is the mechanical reason the width
 * limits stayed stranded in the loader after rumi moved them onto
 * `Matcher::validate` in #32: the file that needed them could not reach them.
 *
 * `matcher.ts` and `registry.ts` re-export every name here, so existing imports
 * keep working.
 */

/** Thrown for matcher construction errors (depth exceeded, invalid regex pattern). */
export class MatcherError extends Error {
	constructor(message: string) {
		super(message);
		this.name = "MatcherError";
	}
}

/** Config has too many field matchers (width-based limit). */
export class TooManyFieldMatchersError extends MatcherError {
	readonly count: number;
	readonly max: number;

	constructor(count: number, max: number) {
		super(`too many field matchers: ${count} exceeds maximum ${max}`);
		this.name = "TooManyFieldMatchersError";
		this.count = count;
		this.max = max;
	}
}

/** Compound predicate has too many children (width-based limit). */
export class TooManyPredicatesError extends MatcherError {
	readonly count: number;
	readonly max: number;

	constructor(count: number, max: number) {
		super(`too many predicates in compound: ${count} exceeds maximum ${max}`);
		this.name = "TooManyPredicatesError";
		this.count = count;
		this.max = max;
	}
}
