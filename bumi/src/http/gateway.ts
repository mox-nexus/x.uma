/**
 * Gateway API compiler — HttpRouteMatch → Matcher<HttpRequest, A>.
 *
 * Translates Gateway API-style route configuration into xuma Matcher trees.
 * Pure TypeScript types mirroring the Gateway API spec (no k8s dependency).
 */

import { type Matcher, MatcherError, matcherFromPredicate } from "../matcher.ts";
import { SinglePredicate, andPredicate, orPredicate } from "../predicate.ts";
import type { Predicate } from "../predicate.ts";
import { ExactMatcher, PrefixMatcher, RegexMatcher } from "../string-matchers.ts";
import { HeaderInput, MethodInput, PathInput, QueryParamInput } from "./inputs.ts";
import type { HttpRequest } from "./request.ts";

export interface HttpPathMatch {
	readonly type: "Exact" | "PathPrefix" | "RegularExpression";
	readonly value: string;
}

export interface HttpHeaderMatch {
	readonly type: "Exact" | "RegularExpression";
	readonly name: string;
	readonly value: string;
}

export interface HttpQueryParamMatch {
	readonly type: "Exact" | "RegularExpression";
	readonly name: string;
	readonly value: string;
}

/** Gateway API HttpRouteMatch — all conditions ANDed together. */
export interface HttpRouteMatch {
	readonly path?: HttpPathMatch;
	readonly method?: string;
	readonly headers?: readonly HttpHeaderMatch[];
	readonly queryParams?: readonly HttpQueryParamMatch[];
}

/** Compile a single route match into a Matcher. */
export function compileRouteMatch<A>(
	routeMatch: HttpRouteMatch,
	action: A,
): Matcher<HttpRequest, A> {
	return matcherFromPredicate(routeMatchToPredicate(routeMatch), action);
}

/**
 * Compile multiple route matches (ORed) into a single Matcher.
 *
 * Substituting a catch-all for an empty array was never spec behaviour. xDS is
 * explicit: "if no matcher above matched and this field is not populated, the
 * match will be considered unsuccessful" — an empty list is a *no-match*. The
 * config path already honours that; only this convenience layer disagreed with
 * the engine underneath it.
 *
 * Not fixed by copying the loader, because there `onNoMatch` is config the
 * operator wrote, while here it is an argument: `([], "allow", "deny")` and
 * `([], "deny", "allow")` are opposite outcomes from the same empty input. An
 * empty list is also almost never written on purpose — it is a config that
 * failed to load, or a filter that removed every rule.
 *
 * @throws {MatcherError} if `matches` is empty, or any entry has no conditions.
 */
export function compileRouteMatches<A>(
	matches: readonly HttpRouteMatch[],
	action: A,
	onNoMatch?: A,
): Matcher<HttpRequest, A> {
	if (matches.length === 0) {
		throw new MatcherError(
			"no route matches, which would match every request. Use compileCatchAll() " +
				"if that is intended, or onNoMatch for a default route.",
		);
	}
	const predicates = matches.map((m) => routeMatchToPredicate(m));
	return matcherFromPredicate(orPredicate(predicates, catchAll()), action, onNoMatch);
}

/**
 * Build a matcher that matches every request.
 *
 * The explicit form of what `compileRouteMatches` now refuses to do by
 * accident. A catch-all is a legitimate route; it just has to be asked for, and
 * it is greppable when someone later asks why a gate admits everything.
 */
export function compileCatchAll<A>(action: A): Matcher<HttpRequest, A> {
	return matcherFromPredicate(catchAll(), action);
}

/** A catch-all predicate that matches any HTTP request. */
function catchAll(): Predicate<HttpRequest> {
	return new SinglePredicate(new PathInput(), new PrefixMatcher(""));
}

function routeMatchToPredicate(rm: HttpRouteMatch): Predicate<HttpRequest> {
	const predicates: SinglePredicate<HttpRequest>[] = [];

	if (rm.path !== undefined) {
		predicates.push(compilePathMatch(rm.path));
	}
	if (rm.method !== undefined) {
		predicates.push(new SinglePredicate(new MethodInput(), new ExactMatcher(rm.method)));
	}
	for (const h of rm.headers ?? []) {
		predicates.push(compileHeaderMatch(h));
	}
	for (const q of rm.queryParams ?? []) {
		predicates.push(compileQueryParamMatch(q));
	}

	// An empty conjunction is vacuously true, so andPredicate would hand back
	// catchAll(). Reaching this is rarely deliberate: HttpRouteMatch is an
	// interface with every field optional and no runtime schema at all, so a
	// JSON config saying `pathPrefix` where it meant `path` arrives here with
	// nothing set and nothing to signal it.
	//
	// This is also the only moment the mistake is visible. After substitution
	// the predicate is PrefixMatcher("") on the path and is indistinguishable
	// from a deliberate catch-all, which is why Matcher.validate() cannot catch
	// it and never could.
	if (predicates.length === 0) {
		throw new MatcherError(
			"HttpRouteMatch has no conditions, so it matches every request — check " +
				"for a misspelled field. Use compileCatchAll() if a catch-all is intended.",
		);
	}

	return andPredicate(predicates, catchAll());
}

function compilePathMatch(pm: HttpPathMatch): SinglePredicate<HttpRequest> {
	switch (pm.type) {
		case "Exact":
			return new SinglePredicate(new PathInput(), new ExactMatcher(pm.value));
		case "PathPrefix":
			return new SinglePredicate(new PathInput(), new PrefixMatcher(pm.value));
		case "RegularExpression":
			return new SinglePredicate(new PathInput(), new RegexMatcher(pm.value));
		default:
			throw new MatcherError(`Unknown path match type: ${pm.type}`);
	}
}

function compileHeaderMatch(hm: HttpHeaderMatch): SinglePredicate<HttpRequest> {
	const input = new HeaderInput(hm.name);
	switch (hm.type) {
		case "Exact":
			return new SinglePredicate(input, new ExactMatcher(hm.value));
		case "RegularExpression":
			return new SinglePredicate(input, new RegexMatcher(hm.value));
		default:
			throw new MatcherError(`Unknown header match type: ${hm.type}`);
	}
}

function compileQueryParamMatch(qm: HttpQueryParamMatch): SinglePredicate<HttpRequest> {
	const input = new QueryParamInput(qm.name);
	switch (qm.type) {
		case "Exact":
			return new SinglePredicate(input, new ExactMatcher(qm.value));
		case "RegularExpression":
			return new SinglePredicate(input, new RegexMatcher(qm.value));
		default:
			throw new MatcherError(`Unknown query param match type: ${qm.type}`);
	}
}
