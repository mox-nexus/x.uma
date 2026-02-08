// Core types
export type { DataInput, InputMatcher, MatchingValue } from "./types.ts";

// Predicates
export { And, Not, Or, SinglePredicate, evaluatePredicate, predicateDepth } from "./predicate.ts";
export type { Predicate } from "./predicate.ts";

// Matcher tree
export {
	Action,
	FieldMatcher,
	MAX_DEPTH,
	Matcher,
	MatcherError,
	NestedMatcher,
} from "./matcher.ts";
export type { OnMatch } from "./matcher.ts";

// String matchers
export {
	ContainsMatcher,
	ExactMatcher,
	PrefixMatcher,
	RegexMatcher,
	SuffixMatcher,
} from "./string-matchers.ts";
