import type { MatchingValue } from "./types.ts";

/** Exact string equality. Pre-lowercases at construction when ignore_case. */
export class ExactMatcher {
	private readonly cmpValue: string;

	constructor(
		readonly value: string,
		readonly ignoreCase: boolean = false,
	) {
		this.cmpValue = ignoreCase ? value.toLowerCase() : value;
	}

	matches(value: MatchingValue): boolean {
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

	matches(value: MatchingValue): boolean {
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

	matches(value: MatchingValue): boolean {
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

	matches(value: MatchingValue): boolean {
		if (typeof value !== "string") return false;
		const input = this.ignoreCase ? value.toLowerCase() : value;
		return input.includes(this.cmpSubstring);
	}
}

/**
 * Regular expression match. Uses RegExp.test() which searches anywhere
 * in the string (equivalent to Python's re.search, Rust's regex find).
 *
 * WARNING: JavaScript's RegExp engine uses backtracking and is vulnerable
 * to ReDoS. Use crusty-bumi (Rust WASM bindings) for linear-time regex.
 */
export class RegexMatcher {
	private readonly compiled: RegExp;

	constructor(readonly pattern: string) {
		this.compiled = new RegExp(pattern);
	}

	matches(value: MatchingValue): boolean {
		if (typeof value !== "string") return false;
		return this.compiled.test(value);
	}
}
