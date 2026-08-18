/**
 * Type registry for config-driven matcher construction.
 *
 * The registry enables generic config loading: JSON/YAML config -> compiled
 * Matcher without domain-specific compile code.
 *
 * Architecture mirrors rumi's registry.rs:
 * - RegistryBuilder<Ctx> -> .build() -> Registry<Ctx> (immutable)
 * - Factories are plain functions: (config) -> DataInput<Ctx> or InputMatcher
 * - loadMatcher() walks the config tree and constructs runtime types
 *
 * Example:
 *
 *   const builder = new RegistryBuilder<Record<string, string>>();
 *   builder.input("xuma.kv.v1.MapInput", (cfg) => new DictInput(cfg.key as string));
 *   const registry = builder.build();
 *
 *   const config = parseProtojson(jsonData); // from ./protojson.ts
 *   const matcher = registry.loadMatcher(config);
 */

import {
	ActionConfig,
	AndPredicateConfig,
	BuiltInMatch,
	CustomMatch,
	type FieldMatcherConfig,
	type MatcherConfig,
	MatcherOnMatchConfig,
	NotPredicateConfig,
	type OnMatchConfig,
	OrPredicateConfig,
	type PredicateConfig,
	SinglePredicateConfig,
	type ValueMatchConfig,
} from "./config.ts";
import { Action, FieldMatcher, Matcher, MatcherError, NestedMatcher } from "./matcher.ts";
import type { OnMatch } from "./matcher.ts";
import { And, Not, Or, SinglePredicate } from "./predicate.ts";
import type { Predicate } from "./predicate.ts";
import {
	BoolMatcher,
	ContainsMatcher,
	ExactMatcher,
	PrefixMatcher,
	RegexMatcher,
	SuffixMatcher,
} from "./string-matchers.ts";
import type { DataInput, InputMatcher } from "./types.ts";

// =====================================================================
// Limits (matching rumi core constants)
// =====================================================================

// Defined in ./limits.ts so string-matchers.ts can enforce them without a
// circular import. Re-exported here so existing call sites are unaffected.
export {
	MAX_FIELD_MATCHERS,
	MAX_PATTERN_LENGTH,
	MAX_PREDICATES_PER_COMPOUND,
	MAX_REGEX_PATTERN_LENGTH,
	PatternTooLongError,
} from "./limits.ts";
import {
	MAX_FIELD_MATCHERS,
	MAX_PATTERN_LENGTH,
	MAX_PREDICATES_PER_COMPOUND,
	MAX_REGEX_PATTERN_LENGTH,
	PatternTooLongError,
} from "./limits.ts";

// =====================================================================
// Error types
// =====================================================================

/** A type_url was not found in the registry. */
export class UnknownTypeUrlError extends MatcherError {
	readonly typeUrl: string;
	readonly registry: string;
	readonly available: string[];

	constructor(typeUrl: string, registry: string, available: string[]) {
		const sorted = [...available].sort();
		const msg =
			sorted.length > 0
				? `unknown ${registry} type_url: "${typeUrl}" (registered: ${sorted.join(", ")})`
				: `unknown ${registry} type_url: "${typeUrl}" (no ${registry} types are registered)`;
		super(msg);
		this.name = "UnknownTypeUrlError";
		this.typeUrl = typeUrl;
		this.registry = registry;
		this.available = sorted;
	}
}

/** A config payload was malformed or semantically invalid. */
export class InvalidConfigError extends MatcherError {
	readonly source: string;

	constructor(source: string) {
		super(`invalid config: ${source}`);
		this.name = "InvalidConfigError";
		this.source = source;
	}
}

/** Config has too many field matchers (width-based limit). */
export class TooManyFieldMatchersError extends MatcherError {
	readonly count: number;
	readonly max: number;

	constructor(count: number, max: number) {
		super(`too many field matchers: ${count} exceeds maximum ${max}`);
		this.name = "TooManyFieldMatchersError";
		this.count = count;
		this.max = max;
	}
}

/** Compound predicate has too many children (width-based limit). */
export class TooManyPredicatesError extends MatcherError {
	readonly count: number;
	readonly max: number;

	constructor(count: number, max: number) {
		super(`too many predicates in compound: ${count} exceeds maximum ${max}`);
		this.name = "TooManyPredicatesError";
		this.count = count;
		this.max = max;
	}
}

// =====================================================================
// Factory types
// =====================================================================

type InputFactory<Ctx> = (config: Record<string, unknown>) => DataInput<Ctx>;
type MatcherFactory = (config: Record<string, unknown>) => InputMatcher;

// =====================================================================
// Builder
// =====================================================================

/**
 * Builder for constructing a Registry.
 *
 * Register DataInput and InputMatcher factories with type URLs, then call
 * build() to produce an immutable Registry.
 *
 * Arch-guild constraint: immutability after build. No runtime registration.
 */
export class RegistryBuilder<Ctx> {
	private readonly inputFactories = new Map<string, InputFactory<Ctx>>();
	private readonly matcherFactories = new Map<string, MatcherFactory>();

	/** Register a DataInput factory with a type URL. */
	input(typeUrl: string, factory: InputFactory<Ctx>): this {
		this.inputFactories.set(typeUrl, factory);
		return this;
	}

	/** Register an InputMatcher factory with a type URL. */
	matcher(typeUrl: string, factory: MatcherFactory): this {
		this.matcherFactories.set(typeUrl, factory);
		return this;
	}

	/** Freeze the registry. No further registration is possible. */
	build(): Registry<Ctx> {
		return new Registry(new Map(this.inputFactories), new Map(this.matcherFactories));
	}
}

// =====================================================================
// Registry
// =====================================================================

/**
 * Immutable registry of DataInput and InputMatcher factories.
 *
 * Constructed via RegistryBuilder. Use loadMatcher() to compile
 * config into a runtime Matcher.
 */
export class Registry<Ctx> {
	private readonly inputFactories: ReadonlyMap<string, InputFactory<Ctx>>;
	private readonly matcherFactories: ReadonlyMap<string, MatcherFactory>;

	constructor(
		inputFactories: Map<string, InputFactory<Ctx>>,
		matcherFactories: Map<string, MatcherFactory>,
	) {
		this.inputFactories = inputFactories;
		this.matcherFactories = matcherFactories;
		Object.freeze(this);
	}

	/**
	 * Load a Matcher from configuration.
	 *
	 * Walks the config tree, constructs DataInputs and InputMatchers via
	 * registered factories, builds predicates and field matchers, and
	 * validates depth constraints.
	 */
	loadMatcher(config: MatcherConfig<string>): Matcher<Ctx, string> {
		if (config.matchers.length > MAX_FIELD_MATCHERS) {
			throw new TooManyFieldMatchersError(config.matchers.length, MAX_FIELD_MATCHERS);
		}

		const matchers = config.matchers.map((fm) => this.loadFieldMatcher(fm));

		let onNoMatch: OnMatch<Ctx, string> | null = null;
		if (config.onNoMatch !== null) {
			onNoMatch = this.loadOnMatch(config.onNoMatch);
		}

		return new Matcher(matchers, onNoMatch);
	}

	/** Number of registered input types. */
	get inputCount(): number {
		return this.inputFactories.size;
	}

	/** Number of registered matcher types. */
	get matcherCount(): number {
		return this.matcherFactories.size;
	}

	/** Check if an input type URL is registered. */
	containsInput(typeUrl: string): boolean {
		return this.inputFactories.has(typeUrl);
	}

	/** Check if a matcher type URL is registered. */
	containsMatcher(typeUrl: string): boolean {
		return this.matcherFactories.has(typeUrl);
	}

	/** Return all registered input type URLs (sorted). */
	inputTypeUrls(): string[] {
		return [...this.inputFactories.keys()].sort();
	}

	/** Return all registered matcher type URLs (sorted). */
	matcherTypeUrls(): string[] {
		return [...this.matcherFactories.keys()].sort();
	}

	// -- Private loading methods -------------------------------------------

	private loadFieldMatcher(config: FieldMatcherConfig<string>): FieldMatcher<Ctx, string> {
		const predicate = this.loadPredicate(config.predicate);
		const onMatch = this.loadOnMatch(config.onMatch);
		return new FieldMatcher(predicate, onMatch);
	}

	private loadPredicate(config: PredicateConfig): Predicate<Ctx> {
		if (config instanceof SinglePredicateConfig) {
			return this.loadSingle(config);
		}
		if (config instanceof AndPredicateConfig) {
			if (config.predicates.length > MAX_PREDICATES_PER_COMPOUND) {
				throw new TooManyPredicatesError(config.predicates.length, MAX_PREDICATES_PER_COMPOUND);
			}
			return new And(config.predicates.map((p) => this.loadPredicate(p)));
		}
		if (config instanceof OrPredicateConfig) {
			if (config.predicates.length > MAX_PREDICATES_PER_COMPOUND) {
				throw new TooManyPredicatesError(config.predicates.length, MAX_PREDICATES_PER_COMPOUND);
			}
			return new Or(config.predicates.map((p) => this.loadPredicate(p)));
		}
		if (config instanceof NotPredicateConfig) {
			return new Not(this.loadPredicate(config.predicate));
		}
		throw new InvalidConfigError("unknown predicate config type");
	}

	private loadSingle(config: SinglePredicateConfig): SinglePredicate<Ctx> {
		const factory = this.inputFactories.get(config.input.typeUrl);
		if (factory === undefined) {
			throw new UnknownTypeUrlError(config.input.typeUrl, "input", [...this.inputFactories.keys()]);
		}

		let dataInput: DataInput<Ctx>;
		try {
			dataInput = factory(config.input.config);
		} catch (e) {
			throw new InvalidConfigError(String(e));
		}

		const matcher = this.loadValueMatch(config.matcher);
		return new SinglePredicate(dataInput, matcher);
	}

	private loadValueMatch(config: ValueMatchConfig): InputMatcher {
		if (config instanceof BuiltInMatch) {
			return compileBuiltIn(config.variant, config.value, config.ignoreCase);
		}
		if (config instanceof CustomMatch) {
			const factory = this.matcherFactories.get(config.typedConfig.typeUrl);
			if (factory === undefined) {
				throw new UnknownTypeUrlError(config.typedConfig.typeUrl, "matcher", [
					...this.matcherFactories.keys(),
				]);
			}
			try {
				return factory(config.typedConfig.config);
			} catch (e) {
				throw new InvalidConfigError(String(e));
			}
		}
		throw new InvalidConfigError("unknown value_match config type");
	}

	private loadOnMatch(config: OnMatchConfig<string>): OnMatch<Ctx, string> {
		if (config instanceof ActionConfig) {
			return new Action(config.action);
		}
		if (config instanceof MatcherOnMatchConfig) {
			const nested = this.loadMatcher(config.matcher);
			return new NestedMatcher(nested);
		}
		throw new InvalidConfigError("unknown on_match config type");
	}
}

// =====================================================================
// Built-in matcher compilation
// =====================================================================

function checkPatternLength(variant: string, value: string): void {
	if (variant === "Regex") {
		if (value.length > MAX_REGEX_PATTERN_LENGTH) {
			throw new PatternTooLongError(value.length, MAX_REGEX_PATTERN_LENGTH);
		}
	} else if (value.length > MAX_PATTERN_LENGTH) {
		throw new PatternTooLongError(value.length, MAX_PATTERN_LENGTH);
	}
}

/**
 * Does this pattern clear the `i` flag with an inline group?
 *
 * ignoreCase asks the engine for a case-insensitive match, and an inline
 * `(?-i)` overrides that — measured in Rust both ways on 2026-08-18, and it is
 * correct regex semantics rather than an engine quirk, so no choice of
 * construction fixes it. The combination is refused instead.
 *
 * Scans flag groups: `(?` followed by flag letters, ended by `)` or `:`. A
 * group clears `i` only if an `i` follows a `-` inside it, so `(?i-s)` is fine
 * and `(?-si)` is not. An escaped `\(` is not a group.
 */
function disablesCaseInsensitivity(pattern: string): boolean {
	let i = 0;
	while (i + 1 < pattern.length) {
		if (pattern[i] === "\\") {
			i += 2;
			continue;
		}
		if (pattern[i] !== "(" || pattern[i + 1] !== "?") {
			i += 1;
			continue;
		}
		let j = i + 2;
		let clearing = false;
		while (j < pattern.length) {
			const c = pattern[j] as string;
			if (c === "-") clearing = true;
			else if (c === "i" && clearing) return true;
			else if (!"imsuxU".includes(c)) break;
			j += 1;
		}
		i = Math.max(j, i + 2);
	}
	return false;
}

function compileBuiltIn(variant: string, value: string, ignoreCase = false): InputMatcher {
	checkPatternLength(variant, value);

	switch (variant) {
		case "Exact":
			return new ExactMatcher(value, ignoreCase);
		case "Prefix":
			return new PrefixMatcher(value, ignoreCase);
		case "Suffix":
			return new SuffixMatcher(value, ignoreCase);
		case "Contains":
			return new ContainsMatcher(value, ignoreCase);
		case "Regex":
			if (ignoreCase && disablesCaseInsensitivity(value)) {
				throw new InvalidConfigError(
					`ignore_case is set, but the pattern "${value}" turns case-insensitivity off inline with a (?-i) flag. An inline flag wins, so this rule would read case-insensitive and not be. Remove one of the two.`,
				);
			}
			try {
				return new RegexMatcher(ignoreCase ? `(?i)${value}` : value);
			} catch (e) {
				throw new InvalidConfigError(
					`invalid regex pattern: ${e instanceof Error ? e.message : String(e)}`,
				);
			}
		default:
			throw new InvalidConfigError(`unknown built-in match variant: "${variant}"`);
	}
}

/**
 * Register core built-in matchers.
 *
 * Call this in domain `register()` functions to avoid duplicating core matcher
 * registrations.
 *
 * bumi had no equivalent of this until 2026-08-18, so `xuma.core.v1.BoolMatcher`
 * resolved in rumi and not here — a cross-language divergence in what a config
 * may name, which is exactly what the conformance suite exists to prevent.
 *
 * `xuma.core.v1.StringMatcher` is deliberately absent: it was a second way to
 * say what `valueMatch` already says, and `customMatch` exists for comparisons
 * that oneof cannot express, not for duplicating it.
 */
export function registerCoreMatchers<Ctx>(builder: RegistryBuilder<Ctx>): RegistryBuilder<Ctx> {
	builder.matcher("xuma.core.v1.BoolMatcher", (config) => {
		return new BoolMatcher(Boolean(config.expected));
	});
	return builder;
}
