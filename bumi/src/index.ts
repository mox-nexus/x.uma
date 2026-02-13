// Core types
export type { DataInput, InputMatcher, MatchingData } from "./types.ts";
// HTTP domain: import from "bumi/http"
// Test utilities (DictInput): import from "bumi/testing"

// Predicates
export {
	And,
	Not,
	Or,
	SinglePredicate,
	andPredicate,
	evaluatePredicate,
	orPredicate,
	predicateDepth,
} from "./predicate.ts";
export type { Predicate } from "./predicate.ts";

// Matcher tree
export {
	Action,
	FieldMatcher,
	MAX_DEPTH,
	Matcher,
	MatcherError,
	NestedMatcher,
	matcherFromPredicate,
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
