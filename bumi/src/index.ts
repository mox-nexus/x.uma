// Core types
export type { DataInput, InputMatcher, MatchingData } from "./types.ts";
// HTTP domain: import from "bumi/http"
// Test utilities (DictInput): import from "bumi/testing"

// Config types
export {
	ActionConfig,
	AndPredicateConfig,
	BuiltInMatch,
	ConfigParseError,
	CustomMatch,
	FieldMatcherConfig,
	MatcherConfig,
	MatcherOnMatchConfig,
	NotPredicateConfig,
	OrPredicateConfig,
	SinglePredicateConfig,
	TypedConfig,
	parseMatcherConfig,
} from "./config.ts";
export type { OnMatchConfig, PredicateConfig, ValueMatchConfig } from "./config.ts";

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

// Registry
export {
	InvalidConfigError,
	MAX_FIELD_MATCHERS,
	MAX_PATTERN_LENGTH,
	MAX_PREDICATES_PER_COMPOUND,
	MAX_REGEX_PATTERN_LENGTH,
	PatternTooLongError,
	Registry,
	RegistryBuilder,
	TooManyFieldMatchersError,
	TooManyPredicatesError,
	UnknownTypeUrlError,
} from "./registry.ts";
