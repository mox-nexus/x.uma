/**
 * Conformance fixture loader for bumi.
 *
 * Loads YAML fixtures from spec/tests/ and converts them to bumi types
 * for parametrized testing. Handles both core format (01-04) and HTTP
 * format (05).
 */

import { readFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { loadAll } from "js-yaml";

import { compileRouteMatches } from "../../src/http/gateway.ts";
import type {
	HttpHeaderMatch,
	HttpPathMatch,
	HttpQueryParamMatch,
	HttpRouteMatch,
} from "../../src/http/gateway.ts";
import { HttpRequest } from "../../src/http/request.ts";
import { Action, FieldMatcher, Matcher, NestedMatcher } from "../../src/matcher.ts";
import { And, Not, Or, SinglePredicate } from "../../src/predicate.ts";
import type { Predicate } from "../../src/predicate.ts";
import {
	ContainsMatcher,
	ExactMatcher,
	PrefixMatcher,
	RegexMatcher,
	SuffixMatcher,
} from "../../src/string-matchers.ts";
import { DictInput } from "../../src/testing.ts";
import type { InputMatcher } from "../../src/types.ts";

const SPEC_DIR = resolve(import.meta.dir, "..", "..", "..", "spec", "tests");

// ─── Fixture interfaces ────────────────────────────────────────────

export interface FixtureCase {
	fixtureName: string;
	caseName: string;
	matcher: Matcher<Record<string, string>, string>;
	context: Record<string, string>;
	expect: string | null;
}

export interface HttpFixtureCase {
	fixtureName: string;
	caseName: string;
	matcher: Matcher<HttpRequest, string>;
	request: HttpRequest;
	expect: string | null;
}

// ─── YAML → bumi type conversion ──────────────────────────────────

// biome-ignore lint/suspicious/noExplicitAny: YAML parsing requires dynamic types
function parseValueMatch(spec: any): InputMatcher {
	if ("exact" in spec) return new ExactMatcher(spec.exact, spec.ignore_case ?? false);
	if ("prefix" in spec) return new PrefixMatcher(spec.prefix, spec.ignore_case ?? false);
	if ("suffix" in spec) return new SuffixMatcher(spec.suffix, spec.ignore_case ?? false);
	if ("contains" in spec) return new ContainsMatcher(spec.contains, spec.ignore_case ?? false);
	if ("regex" in spec) return new RegexMatcher(spec.regex);
	throw new Error(`Unknown value_match type: ${JSON.stringify(spec)}`);
}

// biome-ignore lint/suspicious/noExplicitAny: YAML parsing requires dynamic types
function parsePredicate(spec: any): Predicate<Record<string, string>> {
	if ("single" in spec) {
		const { input, value_match } = spec.single;
		return new SinglePredicate(new DictInput(input.key), parseValueMatch(value_match));
	}
	if ("and" in spec) {
		// biome-ignore lint/suspicious/noExplicitAny: YAML recursive parsing
		return new And(spec.and.map((p: any) => parsePredicate(p)));
	}
	if ("or" in spec) {
		// biome-ignore lint/suspicious/noExplicitAny: YAML recursive parsing
		return new Or(spec.or.map((p: any) => parsePredicate(p)));
	}
	if ("not" in spec) {
		return new Not(parsePredicate(spec.not));
	}
	throw new Error(`Unknown predicate type: ${JSON.stringify(spec)}`);
}

// biome-ignore lint/suspicious/noExplicitAny: YAML parsing requires dynamic types
function parseOnMatch(spec: any): Action<string> | NestedMatcher<Record<string, string>, string> {
	if ("action" in spec) return new Action(spec.action);
	if ("matcher" in spec) return new NestedMatcher(parseMatcher(spec.matcher));
	throw new Error(`Unknown on_match type: ${JSON.stringify(spec)}`);
}

// biome-ignore lint/suspicious/noExplicitAny: YAML parsing requires dynamic types
function parseMatcher(spec: any): Matcher<Record<string, string>, string> {
	const fieldMatchers: FieldMatcher<Record<string, string>, string>[] = [];
	for (const fm of spec.matchers ?? []) {
		fieldMatchers.push(new FieldMatcher(parsePredicate(fm.predicate), parseOnMatch(fm.on_match)));
	}

	const onNoMatch = "on_no_match" in spec ? parseOnMatch(spec.on_no_match) : null;
	return new Matcher(fieldMatchers, onNoMatch);
}

// ─── Fixture loading ──────────────────────────────────────────────

export function loadCoreFixtures(): FixtureCase[] {
	const cases: FixtureCase[] = [];
	const subdirs = readdirSync(SPEC_DIR, { withFileTypes: true })
		.filter((d) => d.isDirectory() && /^0[1-4]_/.test(d.name))
		.sort((a, b) => a.name.localeCompare(b.name));

	for (const subdir of subdirs) {
		const dirPath = join(SPEC_DIR, subdir.name);
		const files = readdirSync(dirPath)
			.filter((f) => f.endsWith(".yaml"))
			.sort();

		for (const file of files) {
			cases.push(...loadCoreFile(join(dirPath, file)));
		}
	}
	return cases;
}

// ─── HTTP fixture loading ─────────────────────────────────────────

export function loadHttpFixtures(): HttpFixtureCase[] {
	const cases: HttpFixtureCase[] = [];
	const httpDir = join(SPEC_DIR, "05_http");

	let files: string[];
	try {
		files = readdirSync(httpDir)
			.filter((f) => f.endsWith(".yaml"))
			.sort();
	} catch {
		return cases;
	}

	for (const file of files) {
		cases.push(...loadHttpFile(join(httpDir, file)));
	}
	return cases;
}

function loadHttpFile(path: string): HttpFixtureCase[] {
	const cases: HttpFixtureCase[] = [];
	const content = readFileSync(path, "utf-8");
	const docs = loadAll(content) as unknown[];

	for (const doc of docs) {
		if (doc == null || typeof doc !== "object") continue;
		// biome-ignore lint/suspicious/noExplicitAny: YAML document
		const d = doc as any;
		const fixtureName: string = d.name;
		const action: string = d.action;
		const onNoMatch: string | undefined = d.on_no_match;
		const matcher = compileHttpFixture(d, action, onNoMatch);

		// biome-ignore lint/suspicious/noExplicitAny: YAML case parsing
		for (const c of d.cases as any[]) {
			cases.push({
				fixtureName,
				caseName: c.name,
				matcher,
				request: parseHttpRequest(c.http_request),
				expect: c.expect ?? null,
			});
		}
	}
	return cases;
}

function compileHttpFixture(
	// biome-ignore lint/suspicious/noExplicitAny: YAML document
	doc: any,
	action: string,
	onNoMatch?: string,
): Matcher<HttpRequest, string> {
	if ("http_route_match" in doc) {
		const routeMatch = parseRouteMatch(doc.http_route_match);
		if (onNoMatch !== undefined) {
			return compileRouteMatches([routeMatch], action, onNoMatch);
		}
		return compileRouteMatches([routeMatch], action);
	}
	if ("http_route_matches" in doc) {
		// biome-ignore lint/suspicious/noExplicitAny: YAML route match array
		const routeMatches = doc.http_route_matches.map((rm: any) => parseRouteMatch(rm));
		return compileRouteMatches(routeMatches, action, onNoMatch);
	}
	throw new Error(`HTTP fixture must have 'http_route_match' or 'http_route_matches'`);
}

// biome-ignore lint/suspicious/noExplicitAny: YAML route match
function parseRouteMatch(spec: any): HttpRouteMatch {
	const rm: HttpRouteMatch & {
		path?: HttpPathMatch;
		method?: string;
		headers?: HttpHeaderMatch[];
		queryParams?: HttpQueryParamMatch[];
	} = {};

	if ("path" in spec) {
		rm.path = { type: spec.path.type, value: spec.path.value };
	}
	if ("method" in spec) {
		rm.method = String(spec.method);
	}
	if ("headers" in spec) {
		// biome-ignore lint/suspicious/noExplicitAny: YAML header match
		rm.headers = spec.headers.map((h: any) => ({
			type: h.type,
			name: h.name,
			value: String(h.value),
		}));
	}
	if ("query_params" in spec) {
		// biome-ignore lint/suspicious/noExplicitAny: YAML query param match
		rm.queryParams = spec.query_params.map((q: any) => ({
			type: q.type,
			name: q.name,
			value: String(q.value),
		}));
	}

	return rm;
}

// biome-ignore lint/suspicious/noExplicitAny: YAML http request
function parseHttpRequest(spec: any): HttpRequest {
	const headers: Record<string, string> = {};
	if (spec.headers) {
		for (const [k, v] of Object.entries(spec.headers)) {
			headers[String(k)] = String(v);
		}
	}
	return new HttpRequest(String(spec.method ?? "GET"), String(spec.path ?? "/"), headers);
}

function loadCoreFile(path: string): FixtureCase[] {
	const cases: FixtureCase[] = [];
	const content = readFileSync(path, "utf-8");
	const docs = loadAll(content) as unknown[];

	for (const doc of docs) {
		if (doc == null || typeof doc !== "object") continue;
		// biome-ignore lint/suspicious/noExplicitAny: YAML document
		const d = doc as any;
		const fixtureName: string = d.name;
		const matcher = parseMatcher(d.matcher);

		// biome-ignore lint/suspicious/noExplicitAny: YAML case parsing
		for (const c of d.cases as any[]) {
			const context: Record<string, string> = {};
			if (c.context) {
				for (const [k, v] of Object.entries(c.context)) {
					context[String(k)] = String(v);
				}
			}
			cases.push({
				fixtureName,
				caseName: c.name,
				matcher,
				context,
				expect: c.expect ?? null,
			});
		}
	}
	return cases;
}
