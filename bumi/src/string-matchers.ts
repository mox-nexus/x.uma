import { RE2JS } from "re2js";

import { MAX_PATTERN_LENGTH, MAX_REGEX_PATTERN_LENGTH, PatternTooLongError } from "./limits.ts";

/**
 * Enforce the literal pattern limit at construction.
 *
 * The limit belongs to the type that holds the pattern, not to the config
 * loader — before 2026-08-17 only the loader checked, so the gateway, direct
 * construction, and the playground's graph renderer were all unguarded.
 * See `DECISIONS.md` D-029.
 */
function checkLiteralLength(pattern: string): void {
	if (pattern.length > MAX_PATTERN_LENGTH) {
		throw new PatternTooLongError(pattern.length, MAX_PATTERN_LENGTH);
	}
}
import { MatcherError } from "./matcher.ts";
import { assertRepeatBudget } from "./regex-budget.ts";
import type { MatchingData } from "./types.ts";

/** Exact string equality. Pre-lowercases at construction when ignore_case. */
export class ExactMatcher {
	private readonly cmpValue: string;

	constructor(
		readonly value: string,
		readonly ignoreCase: boolean = false,
	) {
		checkLiteralLength(value);
		this.cmpValue = ignoreCase ? value.toLowerCase() : value;
	}

	matches(value: MatchingData): boolean {
		if (typeof value !== "string") return false;
		const input = this.ignoreCase ? value.toLowerCase() : value;
		return input === this.cmpValue;
	}
}

/** String prefix match. Pre-lowercases at construction when ignore_case. */
export class PrefixMatcher {
	private readonly cmpPrefix: string;

	constructor(
		readonly prefix: string,
		readonly ignoreCase: boolean = false,
	) {
		checkLiteralLength(prefix);
		this.cmpPrefix = ignoreCase ? prefix.toLowerCase() : prefix;
	}

	matches(value: MatchingData): boolean {
		if (typeof value !== "string") return false;
		const input = this.ignoreCase ? value.toLowerCase() : value;
		return input.startsWith(this.cmpPrefix);
	}
}

/** String suffix match. Pre-lowercases at construction when ignore_case. */
export class SuffixMatcher {
	private readonly cmpSuffix: string;

	constructor(
		readonly suffix: string,
		readonly ignoreCase: boolean = false,
	) {
		checkLiteralLength(suffix);
		this.cmpSuffix = ignoreCase ? suffix.toLowerCase() : suffix;
	}

	matches(value: MatchingData): boolean {
		if (typeof value !== "string") return false;
		const input = this.ignoreCase ? value.toLowerCase() : value;
		return input.endsWith(this.cmpSuffix);
	}
}

/** Substring containment. Pre-lowercases pattern at construction when ignore_case. */
export class ContainsMatcher {
	private readonly cmpSubstring: string;

	constructor(
		readonly substring: string,
		readonly ignoreCase: boolean = false,
	) {
		checkLiteralLength(substring);
		this.cmpSubstring = ignoreCase ? substring.toLowerCase() : substring;
	}

	matches(value: MatchingData): boolean {
		if (typeof value !== "string") return false;
		const input = this.ignoreCase ? value.toLowerCase() : value;
		return input.includes(this.cmpSubstring);
	}
}

/**
 * Regular expression match using RE2 for guaranteed linear-time matching.
 * Uses RE2JS.compile().matcher().find() which searches anywhere in the string
 * (equivalent to Python's re.search, Rust's regex find).
 *
 * RE2 does not support backreferences or lookahead/lookbehind because they
 * require backtracking. Patterns using them are rejected at compile time.
 */
export class RegexMatcher {
	private readonly compiled: RE2JS;

	/**
	 * @throws {PatternTooLongError} if the pattern exceeds the length limit
	 * @throws {MatcherError} if the repetition budget is exceeded, or the
	 *   pattern does not compile
	 *
	 * Both limits are enforced **here**, in the constructor that owns the
	 * compiled program, not in the config loader. Every caller inherits them —
	 * the registry, the gateway, and the playground's graph renderer, which
	 * calls `parseMatcherConfig` without `loadMatcher` and so previously
	 * inherited nothing. See D-029.
	 */
	constructor(readonly pattern: string) {
		if (pattern.length > MAX_REGEX_PATTERN_LENGTH) {
			throw new PatternTooLongError(pattern.length, MAX_REGEX_PATTERN_LENGTH);
		}
		// re2js supplies no compile-time budget of its own — see regex-budget.ts.
		try {
			assertRepeatBudget(pattern);
		} catch (e) {
			throw new MatcherError(
				`invalid regex pattern "${pattern}": ${e instanceof Error ? e.message : String(e)}`,
			);
		}
		try {
			this.compiled = RE2JS.compile(pattern);
		} catch (e) {
			throw new MatcherError(
				`invalid regex pattern "${pattern}": ${e instanceof Error ? e.message : String(e)}`,
			);
		}
	}

	matches(value: MatchingData): boolean {
		if (typeof value !== "string") return false;
		return this.compiled.matcher(value).find();
	}
}
