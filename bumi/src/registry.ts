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
 *   builder.input("xuma.test.v1.StringInput", (cfg) => new DictInput(cfg.key as string));
 *   const registry = builder.build();
 *
 *   const config = parseMatcherConfig(jsonData);
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

export const MAX_FIELD_MATCHERS = 256;
export const MAX_PREDICATES_PER_COMPOUND = 256;
export const MAX_PATTERN_LENGTH = 8192;
export const MAX_REGEX_PATTERN_LENGTH = 4096;

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

/** A match pattern exceeds the length limit. */
export class PatternTooLongError extends MatcherError {
	readonly length: number;
	readonly max: number;

	constructor(length: number, max: number) {
		super(`pattern length ${length} exceeds maximum ${max}`);
		this.name = "PatternTooLongError";
		this.length = length;
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
			return compileBuiltIn(config.variant, config.value);
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

function compileBuiltIn(variant: string, value: string): InputMatcher {
	checkPatternLength(variant, value);

	switch (variant) {
		case "Exact":
			return new ExactMatcher(value);
		case "Prefix":
			return new PrefixMatcher(value);
		case "Suffix":
			return new SuffixMatcher(value);
		case "Contains":
			return new ContainsMatcher(value);
		case "Regex":
			try {
				return new RegexMatcher(value);
			} catch (e) {
				throw new InvalidConfigError(
					`invalid regex pattern: ${e instanceof Error ? e.message : String(e)}`,
				);
			}
		default:
			throw new InvalidConfigError(`unknown built-in match variant: "${variant}"`);
	}
}
