/**
 * Translates the retired terse config dialect into canonical protojson.
 *
 * Test-only, and deliberately so: the *shipped* schema is protojson, full
 * stop -- rumi's MatcherConfig, puma's _protojson.py, bumi's protojson.ts
 * know nothing else. This module exists because these crust tests were
 * written against the terse dialect's compact literal shape, and rewriting a
 * thousand lines of nested object literals by hand is exactly the kind of
 * transcription work that introduces the bug it's supposed to avoid -- the
 * spec/tests converter had one, caught by a fixture, before it was fixed to
 * raise on a both-set oneof instead of silently picking one.
 *
 * So: test bodies keep expressing intent in the terse shape, `pj()`
 * translates it once at the boundary, and the actual call into `fromConfig`
 * is always real protojson -- the crust behavior under test is 100% real;
 * only the literal syntax written by hand in tests is compact.
 *
 * Mirrors `spec/tests`'s own migration script and the Python crust's
 * `_terse_to_protojson.py` structurally. If this drifts from either, that is
 * a bug in one of the three, not a feature of any.
 */

const PREFIX = "type.googleapis.com/";
const INPUT_TYPE = `${PREFIX}xuma.kv.v1.MapInput`;
const ACTION_TYPE = `${PREFIX}xuma.core.v1.NamedAction`;

const VALUE_MATCH: Record<string, string> = {
	Exact: "exact",
	Prefix: "prefix",
	Suffix: "suffix",
	Contains: "contains",
};

// biome-ignore lint/suspicious/noExplicitAny: mirroring loosely-typed terse-dialect literals
type Any = any;

function valueMatch(vm: Record<string, string>): Any {
	const [kind, val] = Object.entries(vm)[0] as [string, string];
	if (kind in VALUE_MATCH) return { [VALUE_MATCH[kind] as string]: val };
	if (kind === "Regex") return { safeRegex: { regex: val } };
	throw new Error(`unmapped value_match variant ${kind}`);
}

function typedConfig(ref: Any, typeUrlField = "type_url"): Any {
	const url = ref[typeUrlField];
	const payload = { ...(ref.config ?? {}) };
	return { "@type": `${PREFIX}${url}`, ...payload };
}

function inputName(typeUrl: string): string {
	const parts = typeUrl.split(".");
	return parts[parts.length - 1] as string;
}

function predicate(p: Any): Any {
	const kind = p.type;
	if (kind === "single") {
		const sp: Any = {
			input: { name: inputName(p.input.type_url), typedConfig: typedConfig(p.input) },
		};
		// Emit BOTH when both are set -- they are a oneof, and an if/else here
		// would silently pick one and hide the illegal-config test it is for.
		if ("value_match" in p) sp.valueMatch = valueMatch(p.value_match);
		if ("custom_match" in p) {
			sp.customMatch = { name: "custom", typedConfig: typedConfig(p.custom_match) };
		}
		if (!("valueMatch" in sp) && !("customMatch" in sp)) {
			throw new Error("single predicate has neither value_match nor custom_match");
		}
		return { singlePredicate: sp };
	}
	if (kind === "and") {
		return { andMatcher: { predicate: p.predicates.map(predicate) } };
	}
	if (kind === "or") {
		return { orMatcher: { predicate: p.predicates.map(predicate) } };
	}
	if (kind === "not") {
		return { notMatcher: predicate(p.predicate) };
	}
	throw new Error(`unmapped predicate type ${kind}`);
}

function onMatch(om: Any): Any {
	const kind = om.type;
	if (kind === "action") {
		const name = om.action;
		return { action: { name, typedConfig: { "@type": ACTION_TYPE, name } } };
	}
	if (kind === "matcher") {
		return { matcher: matcher(om.matcher) };
	}
	throw new Error(`unmapped on_match type ${kind}`);
}

function matcher(cfg: Any): Any {
	const out: Any = {
		matcherList: {
			matchers: cfg.matchers.map((fm: Any) => ({
				predicate: predicate(fm.predicate),
				onMatch: onMatch(fm.on_match),
			})),
		},
	};
	if (cfg.on_no_match != null) out.onNoMatch = onMatch(cfg.on_no_match);
	return out;
}

/** Translate a terse-dialect matcher config object into canonical protojson. */
export function pj(cfg: Any): Any {
	return matcher(cfg);
}
