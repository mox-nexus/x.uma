import { type Predicate, evaluatePredicate, predicateDepth } from "./predicate.ts";

/** Maximum nesting depth for matcher trees. Validated at construction. */
export const MAX_DEPTH = 32;

/** Thrown when a matcher tree exceeds MAX_DEPTH. */
export class MatcherError extends Error {
	constructor(message: string) {
		super(message);
		this.name = "MatcherError";
	}
}

/** Terminal action — emit this value on match. */
export class Action<A> {
	constructor(readonly value: A) {}
}

/** Continue evaluation into a nested matcher. */
export class NestedMatcher<Ctx, A> {
	constructor(readonly matcher: Matcher<Ctx, A>) {}
}

/** Action XOR NestedMatcher — xDS OnMatch exclusivity. */
export type OnMatch<Ctx, A> = Action<A> | NestedMatcher<Ctx, A>;

/** Pairs a predicate with an OnMatch outcome. */
export class FieldMatcher<Ctx, A> {
	constructor(
		readonly predicate: Predicate<Ctx>,
		readonly onMatch: OnMatch<Ctx, A>,
	) {}
}

/**
 * Top-level matcher — first-match-wins semantics.
 *
 * Validates depth at construction (throws MatcherError if > MAX_DEPTH).
 */
export class Matcher<Ctx, A> {
	constructor(
		readonly matchers: readonly FieldMatcher<Ctx, A>[],
		readonly onNoMatch: OnMatch<Ctx, A> | null = null,
	) {
		this.validate();
	}

	/** Evaluate in order, return first match. */
	evaluate(ctx: Ctx): A | null {
		for (const fm of this.matchers) {
			if (evaluatePredicate(fm.predicate, ctx)) {
				const result = evaluateOnMatch(fm.onMatch, ctx);
				if (result !== null) return result;
				// xDS: nested matcher failure → continue to next field_matcher
			}
		}
		if (this.onNoMatch !== null) {
			return evaluateOnMatch(this.onNoMatch, ctx);
		}
		return null;
	}

	/** Validate depth does not exceed MAX_DEPTH. */
	validate(): void {
		const d = this.depth();
		if (d > MAX_DEPTH) {
			throw new MatcherError(`matcher depth ${d} exceeds maximum allowed depth ${MAX_DEPTH}`);
		}
	}

	/** Calculate total nesting depth. */
	depth(): number {
		let maxPredicate = 0;
		let maxNested = 0;
		for (const fm of this.matchers) {
			maxPredicate = Math.max(maxPredicate, predicateDepth(fm.predicate));
			maxNested = Math.max(maxNested, onMatchDepth(fm.onMatch));
		}
		const noMatchD = this.onNoMatch !== null ? onMatchDepth(this.onNoMatch) : 0;
		return 1 + Math.max(maxPredicate, maxNested, noMatchD);
	}
}

/**
 * Create a Matcher from a single predicate, action, and optional fallback.
 *
 * Eliminates repeated `new Matcher([new FieldMatcher(pred, new Action(action))], ...)` boilerplate.
 */
export function matcherFromPredicate<Ctx, A>(
	predicate: Predicate<Ctx>,
	action: A,
	onNoMatch?: A,
): Matcher<Ctx, A> {
	const onNoMatchOm = onNoMatch !== undefined ? new Action(onNoMatch) : null;
	return new Matcher([new FieldMatcher(predicate, new Action(action))], onNoMatchOm);
}

function evaluateOnMatch<Ctx, A>(onMatch: OnMatch<Ctx, A>, ctx: Ctx): A | null {
	if (onMatch instanceof Action) return onMatch.value;
	if (onMatch instanceof NestedMatcher) return onMatch.matcher.evaluate(ctx);
	return null;
}

function onMatchDepth<Ctx, A>(onMatch: OnMatch<Ctx, A>): number {
	if (onMatch instanceof Action) return 0;
	if (onMatch instanceof NestedMatcher) return onMatch.matcher.depth();
	return 0;
}
