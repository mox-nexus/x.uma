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
 * An empty `matches` array creates a catch-all matcher that matches every
 * request (Gateway API semantics: no conditions = match all).
 */
export function compileRouteMatches<A>(
	matches: readonly HttpRouteMatch[],
	action: A,
	onNoMatch?: A,
): Matcher<HttpRequest, A> {
	const predicates = matches.map((m) => routeMatchToPredicate(m));
	return matcherFromPredicate(orPredicate(predicates, catchAll()), action, onNoMatch);
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
