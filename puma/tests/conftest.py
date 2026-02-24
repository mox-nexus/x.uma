"""Conformance fixture loader for puma.

Loads YAML fixtures from spec/tests/ and converts them to puma types
for parametrized testing. Handles both core format (01-04) and HTTP
format (05).
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any

import yaml

from xuma import (
    Action,
    And,
    ContainsMatcher,
    ExactMatcher,
    FieldMatcher,
    Matcher,
    NestedMatcher,
    Not,
    Or,
    PrefixMatcher,
    RegexMatcher,
    SinglePredicate,
    SuffixMatcher,
)
from xuma.http import (
    HttpHeaderMatch,
    HttpPathMatch,
    HttpQueryParamMatch,
    HttpRequest,
    HttpRouteMatch,
    compile_route_matches,
)
from xuma.testing import DictInput

SPEC_DIR = Path(__file__).resolve().parent.parent.parent / "spec" / "tests"


@dataclass
class FixtureCase:
    """A single test case from a conformance fixture."""

    fixture_name: str
    case_name: str
    matcher: Matcher[dict[str, str], str]
    context: dict[str, str]
    expect: str | None


@dataclass
class HttpFixtureCase:
    """A single test case from an HTTP conformance fixture."""

    fixture_name: str
    case_name: str
    matcher: Matcher[Any, str]
    request: HttpRequest
    expect: str | None


# ─── YAML → puma type conversion ────────────────────────────────────────────


def parse_value_match(spec: dict[str, Any]) -> Any:
    """Parse a value_match spec into an InputMatcher."""
    if "exact" in spec:
        return ExactMatcher(spec["exact"], ignore_case=spec.get("ignore_case", False))
    if "prefix" in spec:
        return PrefixMatcher(spec["prefix"], ignore_case=spec.get("ignore_case", False))
    if "suffix" in spec:
        return SuffixMatcher(spec["suffix"], ignore_case=spec.get("ignore_case", False))
    if "contains" in spec:
        return ContainsMatcher(spec["contains"], ignore_case=spec.get("ignore_case", False))
    if "regex" in spec:
        return RegexMatcher(spec["regex"])
    msg = f"Unknown value_match type: {spec}"
    raise ValueError(msg)


def parse_predicate(spec: dict[str, Any]) -> Any:
    """Parse a predicate spec into a Predicate."""
    if "single" in spec:
        single = spec["single"]
        input_spec = single["input"]
        key = input_spec["key"]
        matcher = parse_value_match(single["value_match"])
        return SinglePredicate(DictInput(key), matcher)
    if "and" in spec:
        return And(tuple(parse_predicate(p) for p in spec["and"]))
    if "or" in spec:
        return Or(tuple(parse_predicate(p) for p in spec["or"]))
    if "not" in spec:
        return Not(parse_predicate(spec["not"]))
    msg = f"Unknown predicate type: {spec}"
    raise ValueError(msg)


def parse_on_match(spec: dict[str, Any]) -> Any:
    """Parse an on_match spec into an OnMatch."""
    if "action" in spec:
        return Action(spec["action"])
    if "matcher" in spec:
        return NestedMatcher(parse_matcher(spec["matcher"]))
    msg = f"Unknown on_match type: {spec}"
    raise ValueError(msg)


def parse_matcher(spec: dict[str, Any]) -> Matcher[dict[str, str], str]:
    """Parse a matcher spec into a Matcher."""
    field_matchers = []
    for fm_spec in spec.get("matchers", []):
        predicate = parse_predicate(fm_spec["predicate"])
        on_match = parse_on_match(fm_spec["on_match"])
        field_matchers.append(FieldMatcher(predicate, on_match))

    on_no_match = None
    if "on_no_match" in spec:
        on_no_match = parse_on_match(spec["on_no_match"])

    return Matcher(tuple(field_matchers), on_no_match)


# ─── Fixture loading ────────────────────────────────────────────────────────


def load_core_fixtures() -> list[FixtureCase]:
    """Load all core conformance fixtures (01-04)."""
    cases: list[FixtureCase] = []
    for subdir in sorted(SPEC_DIR.iterdir()):
        if not subdir.is_dir():
            continue
        if not subdir.name.startswith(("01_", "02_", "03_", "04_")):
            continue
        for yaml_file in sorted(subdir.glob("*.yaml")):
            cases.extend(_load_core_file(yaml_file))
    return cases


def _load_core_file(path: Path) -> list[FixtureCase]:
    """Load a single core fixture YAML file (may contain multiple documents)."""
    cases: list[FixtureCase] = []
    with path.open() as f:
        for doc in yaml.safe_load_all(f):
            if doc is None:
                continue
            fixture_name = doc["name"]
            matcher = parse_matcher(doc["matcher"])
            for case in doc["cases"]:
                ctx = {str(k): str(v) for k, v in case["context"].items()}
                expect = case["expect"]
                cases.append(
                    FixtureCase(
                        fixture_name=fixture_name,
                        case_name=case["name"],
                        matcher=matcher,
                        context=ctx,
                        expect=expect,
                    )
                )
    return cases


# ─── HTTP fixture loading ──────────────────────────────────────────────────


def load_http_fixtures() -> list[HttpFixtureCase]:
    """Load all HTTP conformance fixtures (05)."""
    cases: list[HttpFixtureCase] = []
    http_dir = SPEC_DIR / "05_http"
    if not http_dir.exists():
        return cases
    for yaml_file in sorted(http_dir.glob("*.yaml")):
        cases.extend(_load_http_file(yaml_file))
    return cases


def _load_http_file(path: Path) -> list[HttpFixtureCase]:
    """Load a single HTTP fixture YAML file (may contain multiple documents)."""
    cases: list[HttpFixtureCase] = []
    with path.open() as f:
        for doc in yaml.safe_load_all(f):
            if doc is None:
                continue
            fixture_name = doc["name"]
            action = doc["action"]
            on_no_match = doc.get("on_no_match")
            matcher = _compile_http_fixture(doc, action, on_no_match)

            for case in doc["cases"]:
                request = _parse_http_request(case["http_request"])
                expect = case["expect"]
                cases.append(
                    HttpFixtureCase(
                        fixture_name=fixture_name,
                        case_name=case["name"],
                        matcher=matcher,
                        request=request,
                        expect=expect,
                    )
                )
    return cases


def _compile_http_fixture(
    doc: dict[str, Any], action: str, on_no_match: str | None
) -> Matcher[Any, str]:
    """Compile an HTTP fixture document into a Matcher."""
    if "http_route_match" in doc:
        # Single route match
        route_match = _parse_route_match(doc["http_route_match"])
        if on_no_match is not None:
            return compile_route_matches([route_match], action, on_no_match)
        return route_match.compile(action)

    if "http_route_matches" in doc:
        # Multiple route matches (ORed)
        route_matches = [_parse_route_match(rm) for rm in doc["http_route_matches"]]
        return compile_route_matches(route_matches, action, on_no_match)

    msg = f"HTTP fixture must have 'http_route_match' or 'http_route_matches': {doc}"
    raise ValueError(msg)


def _parse_route_match(spec: dict[str, Any]) -> HttpRouteMatch:
    """Parse a YAML route match spec into an HttpRouteMatch."""
    path = None
    if "path" in spec:
        path = HttpPathMatch(
            type=spec["path"]["type"],
            value=spec["path"]["value"],
        )

    method = str(spec["method"]) if "method" in spec else None

    headers = []
    for h in spec.get("headers", []):
        headers.append(HttpHeaderMatch(type=h["type"], name=h["name"], value=str(h["value"])))

    query_params = []
    for q in spec.get("query_params", []):
        query_params.append(
            HttpQueryParamMatch(type=q["type"], name=q["name"], value=str(q["value"]))
        )

    return HttpRouteMatch(
        path=path,
        method=method,
        headers=headers,
        query_params=query_params,
    )


def _parse_http_request(spec: dict[str, Any]) -> HttpRequest:
    """Parse a YAML http_request spec into an HttpRequest."""
    headers = {}
    if "headers" in spec:
        headers = {str(k): str(v) for k, v in spec["headers"].items()}

    return HttpRequest(
        method=str(spec.get("method", "GET")),
        raw_path=str(spec.get("path", "/")),
        headers=headers,
    )
