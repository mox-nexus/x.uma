/**
 * Gateway API compiler — HttpRouteMatch → Matcher<HttpRequest, A>.
 *
 * Translates Gateway API-style route configuration into bumi Matcher trees.
 * Pure TypeScript types mirroring the Gateway API spec (no k8s dependency).
 */

import { Action, FieldMatcher, Matcher } from "../matcher.ts";
import { And, Or, SinglePredicate } from "../predicate.ts";
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
	const predicate = routeMatchToPredicate(routeMatch);
	return new Matcher([new FieldMatcher(predicate, new Action(action))]);
}

/** Compile multiple route matches (ORed) into a single Matcher. */
export function compileRouteMatches<A>(
	matches: readonly HttpRouteMatch[],
	action: A,
	onNoMatch?: A,
): Matcher<HttpRequest, A> {
	const onNoMatchOm = onNoMatch !== undefined ? new Action(onNoMatch) : null;

	if (matches.length === 0) {
		return new Matcher(
			[
				new FieldMatcher(
					new SinglePredicate(new PathInput(), new PrefixMatcher("")),
					new Action(action),
				),
			],
			onNoMatchOm,
		);
	}

	if (matches.length === 1) {
		const predicate = routeMatchToPredicate(matches[0]!);
		return new Matcher([new FieldMatcher(predicate, new Action(action))], onNoMatchOm);
	}

	const predicates = matches.map((m) => routeMatchToPredicate(m));
	return new Matcher([new FieldMatcher(new Or(predicates), new Action(action))], onNoMatchOm);
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

	if (predicates.length === 0) {
		return new SinglePredicate(new PathInput(), new PrefixMatcher(""));
	}
	if (predicates.length === 1) return predicates[0]!;
	return new And(predicates);
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
			throw new Error(`Unknown path match type: ${pm.type}`);
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
			throw new Error(`Unknown header match type: ${hm.type}`);
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
			throw new Error(`Unknown query param match type: ${qm.type}`);
	}
}
