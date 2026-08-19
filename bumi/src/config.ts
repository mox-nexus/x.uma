/**
 * Config types for generic matcher construction.
 *
 * These are the intermediate representation `parseProtojson` (see
 * protojson.ts) produces and `Registry.loadMatcher` consumes:
 *   dict -> parseProtojson() -> MatcherConfig -> Registry.loadMatcher() -> Matcher
 *
 * The terse dialect these types used to deserialize directly (`config.key`,
 * PascalCase `Exact`/`Prefix`) is retired -- DECISIONS.md D-026. The types
 * stay as the IR; only the hand-written reader for that dialect is gone.
 * `parseProtojson` is the only producer now.
 *
 * Relationship to runtime types:
 *
 * | Config type            | Runtime type      |
 * |------------------------|-------------------|
 * | MatcherConfig          | Matcher           |
 * | FieldMatcherConfig     | FieldMatcher      |
 * | PredicateConfig        | Predicate         |
 * | SinglePredicateConfig  | SinglePredicate   |
 * | ValueMatchConfig       | InputMatcher      |
 * | OnMatchConfig          | OnMatch           |
 * | TypedConfig            | DataInput/matcher |
 */

// =====================================================================
// Config types (classes with readonly props, mirroring rumi/core/src/config.rs)
// =====================================================================

/** Reference to a registered type with its configuration. */
export class TypedConfig {
	constructor(
		readonly typeUrl: string,
		readonly config: Record<string, unknown> = {},
	) {}
}

/** Built-in string matching (Exact, Prefix, Suffix, Contains, Regex). */
export class BuiltInMatch {
	constructor(
		readonly variant: string,
		readonly value: string,
		/**
		 * xDS StringMatcher.ignore_case. It belongs to the comparison rather
		 * than to the pattern, which is why it sits here and not in the value.
		 */
		readonly ignoreCase: boolean = false,
	) {}
}

/** Custom matcher resolved via the registry's matcher factories. */
export class CustomMatch {
	constructor(readonly typedConfig: TypedConfig) {}
}

/** Mirrors Envoy's oneof matcher in SinglePredicate. */
export type ValueMatchConfig = BuiltInMatch | CustomMatch;

/** Config for a SinglePredicate: input + value match. */
export class SinglePredicateConfig {
	constructor(
		readonly input: TypedConfig,
		readonly matcher: ValueMatchConfig,
	) {}
}

/** All child predicates must match (logical AND). */
export class AndPredicateConfig {
	constructor(readonly predicates: readonly PredicateConfig[]) {}
}

/** Any child predicate must match (logical OR). */
export class OrPredicateConfig {
	constructor(readonly predicates: readonly PredicateConfig[]) {}
}

/** Inverts the inner predicate (logical NOT). */
export class NotPredicateConfig {
	constructor(readonly predicate: PredicateConfig) {}
}

export type PredicateConfig =
	| SinglePredicateConfig
	| AndPredicateConfig
	| OrPredicateConfig
	| NotPredicateConfig;

/** Return this action when the predicate matches. */
export class ActionConfig<A> {
	constructor(readonly action: A) {}
}

/** Continue evaluation into a nested matcher. */
export class MatcherOnMatchConfig<A> {
	constructor(readonly matcher: MatcherConfig<A>) {}
}

export type OnMatchConfig<A> = ActionConfig<A> | MatcherOnMatchConfig<A>;

/** Pairs a predicate config with an on_match config. */
export class FieldMatcherConfig<A> {
	constructor(
		readonly predicate: PredicateConfig,
		readonly onMatch: OnMatchConfig<A>,
	) {}
}

/**
 * Configuration for a Matcher.
 *
 * Deserializes from JSON/YAML dicts and can be loaded into a runtime
 * Matcher via Registry.loadMatcher().
 */
export class MatcherConfig<A> {
	constructor(
		readonly matchers: readonly FieldMatcherConfig<A>[],
		readonly onNoMatch: OnMatchConfig<A> | null = null,
		/**
		 * xDS models this as `oneof matcher_type`: a list or a tree, never
		 * both. A list is by far the common case, so it stays first.
		 */
		readonly tree: MatcherTreeConfig<A> | null = null,
	) {}
}

/**
 * Configuration for a MatcherTree — xDS `Matcher.MatcherTree`.
 *
 * Carries no fallback. The proto MatcherTree has no `on_no_match` field; the
 * enclosing Matcher owns it, so there is exactly one place a miss can be
 * handled. See DECISIONS.md D-044.
 */
export class MatcherTreeConfig<A> {
	constructor(
		readonly input: TypedConfig,
		/** "exact" or "prefix" — which lookup rule applies. */
		readonly rule: "exact" | "prefix",
		readonly entries: readonly (readonly [string, OnMatchConfig<A>])[],
	) {}
}

// =====================================================================
// Errors
// =====================================================================

/** Error parsing a config dict into config types. */
export class ConfigParseError extends Error {
	constructor(message: string) {
		super(message);
		this.name = "ConfigParseError";
	}
}
