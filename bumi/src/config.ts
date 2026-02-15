/**
 * Config types for generic matcher construction.
 *
 * These types mirror rumi's config.rs -- the same JSON/YAML shape works across
 * all implementations. Config-driven matcher construction path:
 *   dict -> parseMatcherConfig() -> MatcherConfig -> Registry.loadMatcher() -> Matcher
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
	) {}
}

// =====================================================================
// Parsing (unknown -> config types)
// Same JSON shape as rumi's serde deserialization.
// =====================================================================

const STRING_MATCH_VARIANTS = new Set(["Exact", "Prefix", "Suffix", "Contains", "Regex"]);

/** Error parsing a config dict into config types. */
export class ConfigParseError extends Error {
	constructor(message: string) {
		super(message);
		this.name = "ConfigParseError";
	}
}

/**
 * Parse an unknown value into a MatcherConfig<string>.
 *
 * This is the main entry point for config loading. Accepts the same
 * JSON shape as rumi's serde deserialization.
 */
export function parseMatcherConfig(data: unknown): MatcherConfig<string> {
	if (typeof data !== "object" || data === null || Array.isArray(data)) {
		throw new ConfigParseError(`expected object, got ${typeof data}`);
	}
	const obj = data as Record<string, unknown>;

	const rawMatchers = obj.matchers;
	if (rawMatchers === undefined) {
		throw new ConfigParseError("missing required field 'matchers'");
	}
	if (!Array.isArray(rawMatchers)) {
		throw new ConfigParseError(`'matchers' must be an array, got ${typeof rawMatchers}`);
	}

	const matchers = rawMatchers.map((fm) => parseFieldMatcher(fm));

	let onNoMatch: OnMatchConfig<string> | null = null;
	if ("on_no_match" in obj && obj.on_no_match !== undefined) {
		onNoMatch = parseOnMatch(obj.on_no_match);
	}

	return new MatcherConfig(matchers, onNoMatch);
}

function parseFieldMatcher(data: unknown): FieldMatcherConfig<string> {
	if (typeof data !== "object" || data === null || Array.isArray(data)) {
		throw new ConfigParseError(`field_matcher must be an object, got ${typeof data}`);
	}
	const obj = data as Record<string, unknown>;

	if (!("predicate" in obj)) {
		throw new ConfigParseError("field_matcher missing required field 'predicate'");
	}
	if (!("on_match" in obj)) {
		throw new ConfigParseError("field_matcher missing required field 'on_match'");
	}

	const predicate = parsePredicate(obj.predicate);
	const onMatch = parseOnMatch(obj.on_match);
	return new FieldMatcherConfig(predicate, onMatch);
}

function parsePredicate(data: unknown): PredicateConfig {
	if (typeof data !== "object" || data === null || Array.isArray(data)) {
		throw new ConfigParseError(`predicate must be an object, got ${typeof data}`);
	}
	const obj = data as Record<string, unknown>;

	const predType = obj.type;
	if (predType === undefined) {
		throw new ConfigParseError("predicate missing required field 'type'");
	}

	if (predType === "single") {
		return parseSinglePredicate(obj);
	}
	if (predType === "and") {
		const children = (obj.predicates ?? []) as unknown[];
		return new AndPredicateConfig(children.map((p) => parsePredicate(p)));
	}
	if (predType === "or") {
		const children = (obj.predicates ?? []) as unknown[];
		return new OrPredicateConfig(children.map((p) => parsePredicate(p)));
	}
	if (predType === "not") {
		if (!("predicate" in obj)) {
			throw new ConfigParseError("not predicate missing required field 'predicate'");
		}
		return new NotPredicateConfig(parsePredicate(obj.predicate));
	}

	throw new ConfigParseError(`unknown predicate type: "${String(predType)}"`);
}

function parseSinglePredicate(obj: Record<string, unknown>): SinglePredicateConfig {
	if (!("input" in obj)) {
		throw new ConfigParseError("single predicate missing required field 'input'");
	}

	const inputCfg = parseTypedConfig(obj.input);
	const hasValueMatch = "value_match" in obj;
	const hasCustomMatch = "custom_match" in obj;

	if (hasValueMatch && hasCustomMatch) {
		throw new ConfigParseError(
			"exactly one of 'value_match' or 'custom_match' must be set, got both",
		);
	}
	if (!hasValueMatch && !hasCustomMatch) {
		throw new ConfigParseError("one of 'value_match' or 'custom_match' is required");
	}

	let matcher: ValueMatchConfig;
	if (hasValueMatch) {
		matcher = parseValueMatch(obj.value_match);
	} else {
		matcher = new CustomMatch(parseTypedConfig(obj.custom_match));
	}

	return new SinglePredicateConfig(inputCfg, matcher);
}

function parseValueMatch(data: unknown): BuiltInMatch {
	if (typeof data !== "object" || data === null || Array.isArray(data)) {
		throw new ConfigParseError(`value_match must be an object, got ${typeof data}`);
	}
	const obj = data as Record<string, unknown>;

	for (const variant of STRING_MATCH_VARIANTS) {
		if (variant in obj) {
			const value = obj[variant];
			if (typeof value !== "string") {
				throw new ConfigParseError(
					`value_match ${variant} value must be a string, got ${typeof value}`,
				);
			}
			return new BuiltInMatch(variant, value);
		}
	}

	const expected = [...STRING_MATCH_VARIANTS].sort();
	const keys = Object.keys(obj).sort();
	throw new ConfigParseError(
		`value_match must contain one of [${expected.join(", ")}], got keys: [${keys.join(", ")}]`,
	);
}

function parseOnMatch(data: unknown): OnMatchConfig<string> {
	if (typeof data !== "object" || data === null || Array.isArray(data)) {
		throw new ConfigParseError(`on_match must be an object, got ${typeof data}`);
	}
	const obj = data as Record<string, unknown>;

	const omType = obj.type;
	if (omType === undefined) {
		throw new ConfigParseError("on_match missing required field 'type'");
	}

	if (omType === "action") {
		if (!("action" in obj)) {
			throw new ConfigParseError("action on_match missing required field 'action'");
		}
		return new ActionConfig(obj.action as string);
	}
	if (omType === "matcher") {
		if (!("matcher" in obj)) {
			throw new ConfigParseError("matcher on_match missing required field 'matcher'");
		}
		return new MatcherOnMatchConfig(parseMatcherConfig(obj.matcher));
	}

	throw new ConfigParseError(`unknown on_match type: "${String(omType)}"`);
}

function parseTypedConfig(data: unknown): TypedConfig {
	if (typeof data !== "object" || data === null || Array.isArray(data)) {
		throw new ConfigParseError(`typed_config must be an object, got ${typeof data}`);
	}
	const obj = data as Record<string, unknown>;

	if (!("type_url" in obj)) {
		throw new ConfigParseError("typed_config missing required field 'type_url'");
	}
	const typeUrl = obj.type_url;
	if (typeof typeUrl !== "string") {
		throw new ConfigParseError(`type_url must be a string, got ${typeof typeUrl}`);
	}

	const config = (obj.config ?? {}) as Record<string, unknown>;
	if (typeof config !== "object" || config === null || Array.isArray(config)) {
		throw new ConfigParseError(`config must be an object, got ${typeof config}`);
	}

	return new TypedConfig(typeUrl, config);
}
