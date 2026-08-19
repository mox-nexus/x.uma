/**
 * Canonical protojson — the config format users write.
 *
 * protojson is protobuf's own JSON mapping, so a config file here is an
 * `xds.type.matcher.v3.Matcher` written the way protobuf says to write one:
 * lowerCamelCase field names, a `oneof` as a single key, and an `Any` carried
 * as its payload's fields beside an `@type` URL.
 *
 * This reads it into the same `MatcherConfig` the registry already loads, so
 * only the *reader* is new.
 *
 * ## Why hand-written
 *
 * bumi carries no protobuf runtime, and that is a decision rather than an
 * omission. Measured 2026-08-18: ts-proto's generated `fromJSON` never inspects
 * the input's key set — given `{"kye": "role"}` for `MapInput` it returns
 * `{key: ""}` and does not fail. An empty key is exactly the fail-open x.uma
 * rejects everywhere else, and generated code that is lenient is worse than a
 * hand-written reader, because nobody audits a file headed "DO NOT EDIT".
 *
 * The cost is that the dependency on `proto/xuma` is no longer an arrow a build
 * can see. The conformance suite carries it instead — see the fixture coverage
 * check, which fails when a message or field has no fixture.
 *
 * ## Strictness
 *
 * Unknown fields are errors at every level. A typo in a deny rule must not
 * produce a rule that silently never fires. The xDS tree is checked here
 * against the frozen upstream schema; payload fields are checked by the factory
 * that consumes them, because that is where the schema knowledge lives.
 */

import {
	ActionConfig,
	AndPredicateConfig,
	BuiltInMatch,
	CustomMatch,
	FieldMatcherConfig,
	MatcherConfig,
	MatcherOnMatchConfig,
	MatcherTreeConfig,
	NotPredicateConfig,
	type OnMatchConfig,
	OrPredicateConfig,
	type PredicateConfig,
	SinglePredicateConfig,
	TypedConfig,
	type ValueMatchConfig,
} from "./config.ts";
import { ConfigParseError } from "./config.ts";

const TYPE_KEY = "@type";
const TYPE_PREFIX = "type.googleapis.com/";

/**
 * How deep a document may nest before it is refused.
 *
 * This walk runs over untrusted input before any matcher exists, so MAX_DEPTH —
 * which is checked on a built matcher — cannot protect it.
 */
const MAX_JSON_DEPTH = 128;

/**
 * protojson accepts both the proto field name and its lowerCamelCase form.
 * Listing both is what makes a *third* spelling an error rather than a shrug.
 */
const MATCHER_FIELDS = new Set([
	"matcher_list",
	"matcherList",
	"matcher_tree",
	"matcherTree",
	"on_no_match",
	"onNoMatch",
]);
const ON_MATCH_FIELDS = new Set(["matcher", "action", "keep_matching", "keepMatching"]);
const FIELD_MATCHER_FIELDS = new Set(["predicate", "on_match", "onMatch"]);
const PREDICATE_FIELDS = new Set([
	"single_predicate",
	"singlePredicate",
	"or_matcher",
	"orMatcher",
	"and_matcher",
	"andMatcher",
	"not_matcher",
	"notMatcher",
]);
const SINGLE_PREDICATE_FIELDS = new Set([
	"input",
	"value_match",
	"valueMatch",
	"custom_match",
	"customMatch",
]);
const TYPED_EXTENSION_FIELDS = new Set(["name", "typed_config", "typedConfig"]);

const STRING_MATCH_PATTERNS: Record<string, string> = {
	exact: "Exact",
	prefix: "Prefix",
	suffix: "Suffix",
	contains: "Contains",
};
const STRING_MATCHER_FIELDS = new Set([
	...Object.keys(STRING_MATCH_PATTERNS),
	"safe_regex",
	"safeRegex",
	"ignore_case",
	"ignoreCase",
	"custom",
]);

type Obj = Record<string, unknown>;

/** Turns an action's `Any` payload into the value the engine returns. */
export type ActionReader<A> = (config: TypedConfig) => A;

/**
 * Read `xuma.core.v1.NamedAction` into the string the engine returns.
 *
 * In xDS an action is a `TypedExtensionConfig` like any other extension; in
 * this engine the action type is a plain string. This is the adapter, and the
 * default because it is the only action type x.uma ships.
 *
 * An empty `name` is refused. Every other empty identifier in the schema makes
 * a predicate *false* — no decision. This one would make the rule **fire** and
 * return `""`, leaving a host that discriminates on `action === "deny"` to
 * decide the polarity by accident.
 */
export function namedAction(config: TypedConfig): string {
	if (config.typeUrl !== "xuma.core.v1.NamedAction") {
		throw new ConfigParseError(
			`action type "${config.typeUrl}" is not registered; this engine ships only "xuma.core.v1.NamedAction"`,
		);
	}
	const unknown = Object.keys(config.config).filter((k) => k !== "name" && k !== "metadata");
	if (unknown.length > 0) {
		throw new ConfigParseError(
			`NamedAction: unknown field${unknown.length > 1 ? "s" : ""} ${unknown.map((u) => `"${u}"`).join(", ")}`,
		);
	}
	const name = config.config.name;
	if (typeof name !== "string" || name === "") {
		throw new ConfigParseError(
			"NamedAction.name must be a non-empty string; an empty action name " +
				"makes the rule fire and return nothing",
		);
	}
	return name;
}

/**
 * Read a canonical protojson matcher.
 *
 * @throws ConfigParseError if the document is not a valid
 * `xds.type.matcher.v3.Matcher`. Unknown fields are errors.
 */
export function parseProtojson(
	document: unknown,
	action: ActionReader<string> = namedAction,
): MatcherConfig<string> {
	return readMatcher(document, "matcher", 0, action);
}

function asObject(value: unknown, where: string): Obj {
	if (typeof value !== "object" || value === null || Array.isArray(value)) {
		throw new ConfigParseError(`${where}: expected an object, got ${describe(value)}`);
	}
	return value as Obj;
}

function describe(value: unknown): string {
	if (value === null) return "null";
	return Array.isArray(value) ? "array" : typeof value;
}

function checkDepth(depth: number, where: string): void {
	if (depth > MAX_JSON_DEPTH) {
		throw new ConfigParseError(`${where}: config nests deeper than ${MAX_JSON_DEPTH} levels`);
	}
}

/**
 * The reason this module exists.
 *
 * A field the schema does not define is a load error, never a shrug. The
 * hand-written config types this replaces had no such check, so a misspelled key
 * in a deny rule produced a rule that never fired.
 */
function rejectUnknown(data: Obj, allowed: Set<string>, where: string): void {
	const unknown = Object.keys(data)
		.filter((k) => !allowed.has(k))
		.sort();
	if (unknown.length > 0) {
		throw new ConfigParseError(
			`${where}: unknown field${unknown.length > 1 ? "s" : ""} ${unknown.map((u) => `'${u}'`).join(", ")}; expected one of ${[
				...allowed,
			]
				.sort()
				.map((a) => `'${a}'`)
				.join(", ")}`,
		);
	}
}

/** Read a protobuf `oneof`: at most one member may be present. */
function oneOf(data: Obj, names: readonly string[], where: string): [string, unknown] | null {
	const found = names.filter((n) => n in data);
	if (found.length > 1) {
		throw new ConfigParseError(
			`${where}: ${found.map((n) => `'${n}'`).join(", ")} are alternatives; set only one`,
		);
	}
	return found.length === 1 ? [found[0] as string, data[found[0] as string]] : null;
}

function readMatcher(
	value: unknown,
	where: string,
	depth: number,
	action: ActionReader<string>,
): MatcherConfig<string> {
	checkDepth(depth, where);
	const data = asObject(value, where);
	rejectUnknown(data, MATCHER_FIELDS, where);

	const chosen = oneOf(data, ["matcher_list", "matcherList", "matcher_tree", "matcherTree"], where);
	if (chosen === null) {
		throw new ConfigParseError(`${where}: one of 'matcherList' or 'matcherTree' is required`);
	}

	const [key, listValue] = chosen;
	if (key === "matcher_tree" || key === "matcherTree") {
		return new MatcherConfig<string>(
			[],
			readMatcherFallback(data, where, depth, action),
			readMatcherTree(listValue, `${where}.matcherTree`, depth + 1, action),
		);
	}

	const listing = asObject(listValue, `${where}.matcherList`);
	rejectUnknown(listing, new Set(["matchers"]), `${where}.matcherList`);
	const raw = listing.matchers ?? [];
	if (!Array.isArray(raw)) {
		throw new ConfigParseError(`${where}.matcherList.matchers: expected a list`);
	}

	const matchers = raw.map((fm, i) =>
		readFieldMatcher(fm, `${where}.matchers[${i}]`, depth + 1, action),
	);

	return new MatcherConfig(matchers, readMatcherFallback(data, where, depth, action));
}

/** A Matcher's `onNoMatch`, wherever the matcher_type lands. */
function readMatcherFallback(
	data: Record<string, unknown>,
	where: string,
	depth: number,
	action: ActionReader<string>,
): OnMatchConfig<string> | null {
	let onNoMatch: OnMatchConfig<string> | null = null;
	for (const k of ["on_no_match", "onNoMatch"]) {
		if (k in data) onNoMatch = readOnMatch(data[k], `${where}.onNoMatch`, depth + 1, action);
	}
	return onNoMatch;
}

const TREE_FIELDS = new Set([
	"input",
	"exact_match_map",
	"exactMatchMap",
	"prefix_match_map",
	"prefixMatchMap",
	"custom_match",
	"customMatch",
]);

/** Read an xDS `MatcherTree`. */
function readMatcherTree(
	value: unknown,
	where: string,
	depth: number,
	action: ActionReader<string>,
): MatcherTreeConfig<string> {
	checkDepth(depth, where);
	const data = asObject(value, where);
	rejectUnknown(data, TREE_FIELDS, where);

	if (!("input" in data)) {
		throw new ConfigParseError(`${where}: 'input' is required`);
	}
	const treeInput = readTypedExtension(data.input, `${where}.input`);

	const chosen = oneOf(
		data,
		[
			"exact_match_map",
			"exactMatchMap",
			"prefix_match_map",
			"prefixMatchMap",
			"custom_match",
			"customMatch",
		],
		where,
	);
	if (chosen === null) {
		// Fail closed. An empty tree matches nothing, so it would fall straight
		// through to onNoMatch and silently turn a deny rule into whatever the
		// fallback says.
		throw new ConfigParseError(`${where}: one of 'exactMatchMap' or 'prefixMatchMap' is required`);
	}

	const [key, mapValue] = chosen;
	if (key === "custom_match" || key === "customMatch") {
		// Refused by name rather than falling into the branch above: reporting
		// "no map set" for a config that plainly sets one sends the author
		// looking in the wrong place.
		throw new ConfigParseError(
			`${where}: 'customMatch' is not supported; use 'exactMatchMap' or 'prefixMatchMap'`,
		);
	}

	const rule = key === "prefix_match_map" || key === "prefixMatchMap" ? "prefix" : "exact";
	const label = rule === "prefix" ? "prefixMatchMap" : "exactMatchMap";

	const holder = asObject(mapValue, `${where}.${label}`);
	rejectUnknown(holder, new Set(["map"]), `${where}.${label}`);
	const raw = holder.map ?? {};
	if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
		throw new ConfigParseError(`${where}.${label}.map: expected an object`);
	}

	// Sorted so a duplicate-key error names the same key on every run, and so
	// the loaded config is a deterministic function of the document.
	const entries = Object.keys(raw as Record<string, unknown>)
		.sort()
		.map(
			(k) =>
				[
					k,
					readOnMatch(
						(raw as Record<string, unknown>)[k],
						`${where}.${label}.map[${JSON.stringify(k)}]`,
						depth + 1,
						action,
					),
				] as const,
		);

	return new MatcherTreeConfig(treeInput, rule, entries);
}

function readFieldMatcher(
	value: unknown,
	where: string,
	depth: number,
	action: ActionReader<string>,
): FieldMatcherConfig<string> {
	checkDepth(depth, where);
	const data = asObject(value, where);
	rejectUnknown(data, FIELD_MATCHER_FIELDS, where);

	if (!("predicate" in data)) {
		throw new ConfigParseError(`${where}: missing required field 'predicate'`);
	}
	const onMatch = oneOf(data, ["on_match", "onMatch"], where);
	if (onMatch === null) {
		throw new ConfigParseError(`${where}: missing required field 'onMatch'`);
	}

	return new FieldMatcherConfig(
		readPredicate(data.predicate, `${where}.predicate`, depth + 1),
		readOnMatch(onMatch[1], `${where}.onMatch`, depth + 1, action),
	);
}

function readPredicate(value: unknown, where: string, depth: number): PredicateConfig {
	checkDepth(depth, where);
	const data = asObject(value, where);
	rejectUnknown(data, PREDICATE_FIELDS, where);

	const chosen = oneOf(
		data,
		[
			"single_predicate",
			"singlePredicate",
			"or_matcher",
			"orMatcher",
			"and_matcher",
			"andMatcher",
			"not_matcher",
			"notMatcher",
		],
		where,
	);
	if (chosen === null) {
		throw new ConfigParseError(
			`${where}: a predicate must set one of singlePredicate, andMatcher, orMatcher, notMatcher`,
		);
	}

	const [key, inner] = chosen;
	if (key === "single_predicate" || key === "singlePredicate") {
		return readSinglePredicate(inner, `${where}.singlePredicate`, depth + 1);
	}
	if (key === "not_matcher" || key === "notMatcher") {
		return new NotPredicateConfig(readPredicate(inner, `${where}.notMatcher`, depth + 1));
	}

	// and_matcher / or_matcher carry a PredicateList: { "predicate": [...] }
	const listing = asObject(inner, `${where}.${key}`);
	rejectUnknown(listing, new Set(["predicate"]), `${where}.${key}`);
	const raw = listing.predicate ?? [];
	if (!Array.isArray(raw)) {
		throw new ConfigParseError(`${where}.${key}.predicate: expected a list`);
	}
	const children = raw.map((p, i) =>
		readPredicate(p, `${where}.${key}.predicate[${i}]`, depth + 1),
	);
	return key === "and_matcher" || key === "andMatcher"
		? new AndPredicateConfig(children)
		: new OrPredicateConfig(children);
}

function readSinglePredicate(value: unknown, where: string, depth: number): SinglePredicateConfig {
	checkDepth(depth, where);
	const data = asObject(value, where);
	rejectUnknown(data, SINGLE_PREDICATE_FIELDS, where);

	if (!("input" in data)) {
		throw new ConfigParseError(`${where}: missing required field 'input'`);
	}
	const input = readTypedExtension(data.input, `${where}.input`);

	const chosen = oneOf(data, ["value_match", "valueMatch", "custom_match", "customMatch"], where);
	if (chosen === null) {
		throw new ConfigParseError(`${where}: one of 'valueMatch' or 'customMatch' is required`);
	}

	const [key, matchValue] = chosen;
	const matcher: ValueMatchConfig =
		key === "custom_match" || key === "customMatch"
			? new CustomMatch(readTypedExtension(matchValue, `${where}.customMatch`))
			: readStringMatcher(matchValue, `${where}.valueMatch`);

	return new SinglePredicateConfig(input, matcher);
}

function readStringMatcher(value: unknown, where: string): BuiltInMatch {
	const data = asObject(value, where);
	rejectUnknown(data, STRING_MATCHER_FIELDS, where);

	if ("custom" in data) {
		throw new ConfigParseError(`${where}: custom StringMatcher extensions are not implemented`);
	}

	const ignoreCase = Boolean(data.ignore_case ?? data.ignoreCase ?? false);

	const chosen = oneOf(
		data,
		[...Object.keys(STRING_MATCH_PATTERNS), "safe_regex", "safeRegex"],
		where,
	);
	if (chosen === null) {
		throw new ConfigParseError(
			`${where}: a StringMatcher must set one of exact, prefix, suffix, contains, safeRegex`,
		);
	}

	const [key, patternValue] = chosen;
	if (key === "safe_regex" || key === "safeRegex") {
		const regex = asObject(patternValue, `${where}.safeRegex`);
		rejectUnknown(regex, new Set(["regex", "google_re2", "googleRe2"]), `${where}.safeRegex`);
		const pattern = regex.regex;
		if (typeof pattern !== "string") {
			throw new ConfigParseError(`${where}.safeRegex: missing required field 'regex'`);
		}
		return new BuiltInMatch("Regex", pattern, ignoreCase);
	}

	if (typeof patternValue !== "string") {
		throw new ConfigParseError(`${where}.${key}: expected a string, got ${describe(patternValue)}`);
	}
	return new BuiltInMatch(STRING_MATCH_PATTERNS[key] as string, patternValue, ignoreCase);
}

function readOnMatch(
	value: unknown,
	where: string,
	depth: number,
	action: ActionReader<string>,
): OnMatchConfig<string> {
	checkDepth(depth, where);
	const data = asObject(value, where);
	rejectUnknown(data, ON_MATCH_FIELDS, where);

	// keepMatching records the action and keeps evaluating in xDS; this engine
	// returns the first match. Accepting it would answer a different question
	// than the config asked, so it is refused rather than ignored.
	if (data.keep_matching === true || data.keepMatching === true) {
		throw new ConfigParseError(
			`${where}: keepMatching is not implemented. In xDS it records the action and continues evaluating; this engine returns the first match. Remove it, or restructure the rule.`,
		);
	}

	const chosen = oneOf(data, ["matcher", "action"], where);
	if (chosen === null) {
		throw new ConfigParseError(`${where}: one of 'matcher' or 'action' is required`);
	}

	const [key, inner] = chosen;
	if (key === "matcher") {
		return new MatcherOnMatchConfig(readMatcher(inner, `${where}.matcher`, depth + 1, action));
	}
	return new ActionConfig(action(readTypedExtension(inner, `${where}.action`)));
}

/** Read a `TypedExtensionConfig` — a name and an `Any` payload. */
function readTypedExtension(value: unknown, where: string): TypedConfig {
	const data = asObject(value, where);
	rejectUnknown(data, TYPED_EXTENSION_FIELDS, where);

	let payload: Obj | null = null;
	for (const k of ["typed_config", "typedConfig"]) {
		if (k in data) payload = asObject(data[k], `${where}.typedConfig`);
	}
	if (payload === null) {
		throw new ConfigParseError(`${where}: missing required field 'typedConfig'`);
	}

	const url = payload[TYPE_KEY];
	if (typeof url !== "string") {
		throw new ConfigParseError(`${where}.typedConfig: missing required field '${TYPE_KEY}'`);
	}
	if (!url.startsWith(TYPE_PREFIX)) {
		throw new ConfigParseError(
			`${where}.typedConfig: '${TYPE_KEY}' must be a full type URL beginning ` +
				`'${TYPE_PREFIX}', got "${url}"`,
		);
	}

	// The payload body is handed to its factory unwalked. Its fields belong to
	// the payload's own schema, and the factory is where that knowledge lives.
	const body: Obj = {};
	for (const [k, v] of Object.entries(payload)) if (k !== TYPE_KEY) body[k] = v;
	return new TypedConfig(url.slice(TYPE_PREFIX.length), body);
}
