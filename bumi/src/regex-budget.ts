/**
 * Compile-time budget for regex patterns.
 *
 * `re2js` implements neither of C++ RE2's two compile-time guards: no `max_mem`
 * program budget and no nested-repetition product limit. Compiled program size
 * grows as the product of nested `{n}` counts with no ceiling, and pattern
 * *length* is the wrong axis to bound it on — measured on re2js 0.4.3:
 *
 * ```
 * a{100}                 6 chars     2ms     35MB
 * (a{100}){100}         13 chars     5ms     48MB
 * ((a{100}){100}){100}  20 chars   282ms    286MB
 * ```
 *
 * One more nesting level reaches multiple seconds and gigabytes. `puma` is
 * immune because `google-re2` rejects these outright, and Rust's `regex` crate
 * rejects on its 10 MB size limit. bumi was the only implementation without a
 * compile-time guard.
 *
 * This supplies one, mirroring RE2's own rule: the product of nested repetition
 * counts may not exceed {@link MAX_REPEAT_PRODUCT}.
 *
 * See `DECISIONS.md` D-029 and `reference/security-review-2026-08-16.md` F-01.
 */

/**
 * Maximum product of nested repetition counts.
 *
 * 1000 is C++ RE2's own `kMaxRepeat`. Chosen to match rather than invented, so
 * a pattern accepted here is one upstream RE2 would also accept.
 */
export const MAX_REPEAT_PRODUCT = 1000;

/** A repetition budget violation, with the computed product for the message. */
export class RepeatBudgetError extends Error {
	constructor(
		readonly product: number,
		readonly max: number,
	) {
		super(
			`regex repetition product is ${product}, but maximum allowed is ${max} — nested counted repetition compiles to a program of that size`,
		);
		this.name = "RepeatBudgetError";
	}
}

/** Parse `{n}`, `{n,}` or `{n,m}` at `i`; returns the factor and next index. */
function readRepeat(p: string, i: number): { factor: number; next: number } | null {
	if (p[i] !== "{") return null;
	const close = p.indexOf("}", i);
	if (close === -1) return null;
	const body = p.slice(i + 1, close);
	// {n} | {n,} | {n,m} — the upper bound drives program size, so prefer it.
	const m = /^(\d+)(?:,(\d*))?$/.exec(body);
	if (!m) return null;
	const lo = Number.parseInt(m[1] as string, 10);
	const hiRaw = m[2];
	const hi = hiRaw === undefined || hiRaw === "" ? lo : Number.parseInt(hiRaw, 10);
	return { factor: Math.max(lo, hi, 1), next: close + 1 };
}

/**
 * Compute the largest product of nested repetition counts in `pattern`.
 *
 * Walks the pattern maintaining a stack of group frames. Each frame records the
 * largest repeat product seen directly inside it; closing a group multiplies
 * that by the group's own repeat count and folds it into the parent.
 *
 * Deliberately approximate in the safe direction: it does not parse alternation
 * or backreferences, and it counts the largest single path rather than the sum.
 * It exists to reject the unbounded class, not to predict program size exactly.
 */
export function maxRepeatProduct(pattern: string): number {
	const stack: number[] = [1];
	let i = 0;

	while (i < pattern.length) {
		const c = pattern[i];

		if (c === "\\") {
			i += 2; // escaped char — never a group or quantifier
			continue;
		}

		if (c === "[") {
			// Character class: skip to the unescaped ']'. No groups inside.
			i += 1;
			if (pattern[i] === "]") i += 1; // leading ']' is literal
			while (i < pattern.length && pattern[i] !== "]") {
				i += pattern[i] === "\\" ? 2 : 1;
			}
			i += 1;
			const rep = readRepeat(pattern, i);
			if (rep) {
				stack[stack.length - 1] = Math.max(stack[stack.length - 1] as number, rep.factor);
				i = rep.next;
			}
			continue;
		}

		if (c === "(") {
			stack.push(1);
			i += 1;
			continue;
		}

		if (c === ")") {
			const inner = (stack.pop() ?? 1) as number;
			i += 1;
			const rep = readRepeat(pattern, i);
			const factor = rep ? rep.factor : 1;
			if (rep) i = rep.next;
			if (stack.length === 0) stack.push(1); // unbalanced ')' — stay safe
			stack[stack.length - 1] = Math.max(stack[stack.length - 1] as number, inner * factor);
			continue;
		}

		// A literal or metacharacter, possibly quantified.
		i += 1;
		const rep = readRepeat(pattern, i);
		if (rep) {
			stack[stack.length - 1] = Math.max(stack[stack.length - 1] as number, rep.factor);
			i = rep.next;
		}
	}

	// Unbalanced '(' leaves frames on the stack; take the largest.
	return Math.max(...stack.map((n) => n));
}

/**
 * Throw if `pattern` exceeds the repetition budget.
 *
 * @throws {RepeatBudgetError}
 */
export function assertRepeatBudget(pattern: string): void {
	const product = maxRepeatProduct(pattern);
	if (product > MAX_REPEAT_PRODUCT) {
		throw new RepeatBudgetError(product, MAX_REPEAT_PRODUCT);
	}
}
