import type { DataInput, InputMatcher, MatchingValue } from "./types.ts";

/** Pairs a domain-specific input with a domain-agnostic matcher. */
export class SinglePredicate<Ctx> {
	constructor(
		readonly input: DataInput<Ctx>,
		readonly matcher: InputMatcher,
	) {}

	evaluate(ctx: Ctx): boolean {
		const value: MatchingValue = this.input.get(ctx);
		if (value === null) return false; // INV: null → false
		return this.matcher.matches(value);
	}
}

/** All predicates must be true. Empty AND is vacuously true. */
export class And<Ctx> {
	constructor(readonly predicates: readonly Predicate<Ctx>[]) {}

	evaluate(ctx: Ctx): boolean {
		return this.predicates.every((p) => evaluatePredicate(p, ctx));
	}
}

/** At least one predicate must be true. Empty OR is vacuously false. */
export class Or<Ctx> {
	constructor(readonly predicates: readonly Predicate<Ctx>[]) {}

	evaluate(ctx: Ctx): boolean {
		return this.predicates.some((p) => evaluatePredicate(p, ctx));
	}
}

/** Inverts a predicate. */
export class Not<Ctx> {
	constructor(readonly predicate: Predicate<Ctx>) {}

	evaluate(ctx: Ctx): boolean {
		return !evaluatePredicate(this.predicate, ctx);
	}
}

/** Discriminated union of all predicate types. */
export type Predicate<Ctx> = SinglePredicate<Ctx> | And<Ctx> | Or<Ctx> | Not<Ctx>;

/** Evaluate any predicate variant. */
export function evaluatePredicate<Ctx>(p: Predicate<Ctx>, ctx: Ctx): boolean {
	return p.evaluate(ctx);
}

/** Calculate nesting depth of a predicate tree. */
export function predicateDepth<Ctx>(p: Predicate<Ctx>): number {
	if (p instanceof SinglePredicate) return 1;
	if (p instanceof And || p instanceof Or) {
		const maxChild = p.predicates.reduce((max, sub) => Math.max(max, predicateDepth(sub)), 0);
		return 1 + maxChild;
	}
	if (p instanceof Not) return 1 + predicateDepth(p.predicate);
	return 0;
}
