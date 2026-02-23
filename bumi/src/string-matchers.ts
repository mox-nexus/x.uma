import { RE2JS } from "re2js";

import { MatcherError } from "./matcher.ts";
import type { MatchingData } from "./types.ts";

/** Exact string equality. Pre-lowercases at construction when ignore_case. */
export class ExactMatcher {
	private readonly cmpValue: string;

	constructor(
		readonly value: string,
		readonly ignoreCase: boolean = false,
	) {
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

	constructor(readonly pattern: string) {
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
