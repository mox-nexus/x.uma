import { MatcherError, TooManyFieldMatchersError, TooManyPredicatesError } from "./errors.ts";
import { MAX_FIELD_MATCHERS, MAX_PREDICATES_PER_COMPOUND } from "./limits.ts";
import { And, Not, Or, type Predicate, evaluatePredicate, predicateDepth } from "./predicate.ts";
import type { DataInput } from "./types.ts";

export { MatcherError } from "./errors.ts";

/** Maximum nesting depth for matcher trees. Validated at construction. */
export const MAX_DEPTH = 32;

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
/**
 * Map-based matching — xDS `Matcher.MatcherTree`.
 *
 * Extracts a key via a DataInput, then looks it up either exactly or by
 * longest matching prefix. The prefix rule is the one behaviour a matcher list
 * cannot express: a list is first-match-wins in written order, so it returns
 * `/api` for `/api/v2` whenever `/api` is listed first.
 *
 * Carries no fallback — the enclosing Matcher owns it. See DECISIONS.md D-044.
 *
 * rumi backs the prefix rule with a radix tree, O(k) in the key length. This
 * scans the entries instead, O(n·k). The conformance suite pins behaviour, not
 * the data structure.
 */
export class MatcherTree<Ctx, A> {
	constructor(
		readonly input: DataInput<Ctx>,
		readonly rule: "exact" | "prefix",
		readonly entries: readonly (readonly [string, OnMatch<Ctx, A>])[],
	) {}

	/** The lookup key, or null if the input produced no usable string. */
	keyFor(ctx: Ctx): string | null {
		const data = this.input.get(ctx);
		return typeof data === "string" ? data : null;
	}

	/** The entry a key selects, and which entry key won. */
	lookup(key: string): readonly [string, OnMatch<Ctx, A>] | null {
		if (this.rule === "exact") {
			for (const entry of this.entries) {
				if (entry[0] === key) return entry;
			}
			return null;
		}
		let best: readonly [string, OnMatch<Ctx, A>] | null = null;
		for (const entry of this.entries) {
			if (key.startsWith(entry[0]) && (best === null || entry[0].length > best[0].length)) {
				best = entry;
			}
		}
		return best;
	}

	/** Look up and dispatch. A miss is null; the Matcher owns the fallback. */
	evaluate(ctx: Ctx): A | null {
		const key = this.keyFor(ctx);
		if (key === null) return null;
		const hit = this.lookup(key);
		if (hit === null) return null;
		return evaluateOnMatch(hit[1], ctx);
	}

	/**
	 * Deepest nesting reachable through this tree's entries.
	 *
	 * Entries hold OnMatch, which can hold a Matcher, which can hold another
	 * tree. Not walking this is what let such a config report depth 1 and pass
	 * validation — see DECISIONS.md D-045.
	 */
	depth(): number {
		let max = 0;
		for (const [, om] of this.entries) {
			max = Math.max(max, onMatchDepth(om));
		}
		return max;
	}
}

export class Matcher<Ctx, A> {
	constructor(
		readonly matchers: readonly FieldMatcher<Ctx, A>[],
		readonly onNoMatch: OnMatch<Ctx, A> | null = null,
		readonly tree: MatcherTree<Ctx, A> | null = null,
	) {
		this.validate();
	}

	/** Evaluate in order, return first match. */
	evaluate(ctx: Ctx): A | null {
		if (this.tree !== null) {
			// A tree miss and a tree hit whose nested matcher returned null
			// both arrive as null, and both then reach onNoMatch — the same
			// rule the list follows when it falls off the end.
			const result = this.tree.evaluate(ctx);
			if (result !== null) return result;
		} else {
			for (const fm of this.matchers) {
				if (evaluatePredicate(fm.predicate, ctx)) {
					const result = evaluateOnMatch(fm.onMatch, ctx);
					if (result !== null) return result;
					// xDS: nested matcher failure → continue to the next one.
				}
			}
		}
		if (this.onNoMatch !== null) {
			return evaluateOnMatch(this.onNoMatch, ctx);
		}
		return null;
	}

	/** Validate depth and width against the declared limits. */
	validate(): void {
		const d = this.depth();
		if (d > MAX_DEPTH) {
			throw new MatcherError(`matcher depth ${d} exceeds maximum allowed depth ${MAX_DEPTH}`);
		}
		this.validateWidths();
	}

	/**
	 * Reject a list or compound predicate wider than its declared limit.
	 *
	 * The widths lived only in the registry, so every path that did not go
	 * through `loadMatcher` — the gateway compiler above all — accepted a
	 * matcher of any width. rumi closed this in #32 by moving the checks onto
	 * `Matcher::validate`; puma and bumi were not carried across, and a
	 * 257-child compound compiled here without complaint until 2026-08-23.
	 * The rule the security review named: the type that holds the resource
	 * owns the limit on that resource.
	 */
	private validateWidths(): void {
		if (this.tree === null && this.matchers.length > MAX_FIELD_MATCHERS) {
			throw new TooManyFieldMatchersError(this.matchers.length, MAX_FIELD_MATCHERS);
		}
		for (const fm of this.matchers) {
			validatePredicateWidth(fm.predicate);
			if (fm.onMatch instanceof NestedMatcher) fm.onMatch.matcher.validate();
		}
		if (this.onNoMatch instanceof NestedMatcher) this.onNoMatch.matcher.validate();
		// A tree's entries are bounded by MAX_TREE_ENTRIES at load, for a
		// different reason — see limits.ts. Its nested matchers still validate
		// themselves at construction.
	}

	/** Calculate total nesting depth. */
	depth(): number {
		let body = 0;
		if (this.tree !== null) {
			body = this.tree.depth();
		} else {
			for (const fm of this.matchers) {
				body = Math.max(body, predicateDepth(fm.predicate), onMatchDepth(fm.onMatch));
			}
		}
		const noMatchD = this.onNoMatch !== null ? onMatchDepth(this.onNoMatch) : 0;
		return 1 + Math.max(body, noMatchD);
	}
}

/** Recursively bound compound-predicate width. See `Matcher.validateWidths`. */
function validatePredicateWidth<Ctx>(p: Predicate<Ctx>): void {
	if (p instanceof And || p instanceof Or) {
		if (p.predicates.length > MAX_PREDICATES_PER_COMPOUND) {
			throw new TooManyPredicatesError(p.predicates.length, MAX_PREDICATES_PER_COMPOUND);
		}
		for (const sub of p.predicates) validatePredicateWidth(sub);
		return;
	}
	if (p instanceof Not) validatePredicateWidth(p.predicate);
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
