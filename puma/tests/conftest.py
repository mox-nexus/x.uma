"""HTTP conformance fixture loader for puma.

Loads `spec/tests/05_http/` — Gateway API route matches fed through the HTTP
compiler. That dialect is not a config format and D-026 does not change it, so
it stays while the others go.

The protojson fixtures have their own loader in `test_proto_conformance.py`.
This file used to load three more dialects; they were retired with the
hand-written config vocabulary.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Any

import yaml

from xuma.http import (
    HttpHeaderMatch,
    HttpPathMatch,
    HttpQueryParamMatch,
    HttpRequest,
    HttpRouteMatch,
    compile_route_matches,
)

if TYPE_CHECKING:
    from xuma import Matcher

SPEC_DIR = Path(__file__).resolve().parent.parent.parent / "spec" / "tests"


@dataclass
class HttpFixtureCase:
    """A single test case from an HTTP conformance fixture.

    Either an evaluation case (matcher + request + expect) or a compile-error
    case, in which case matcher and request are None and error_contains carries
    the substring the failure must mention.
    """

    fixture_name: str
    case_name: str
    matcher: Matcher[Any, str] | None
    request: HttpRequest | None
    expect: str | None
    error_contains: str | None = None
    doc: dict[str, Any] | None = None
    #: Set when the fixture does not list puma — the compile must then fail.
    unlisted: bool = False


# ─── YAML → puma type conversion ────────────────────────────────────────────


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

            # The migration ledger, same rule as the protojson runner: a
            # fixture that does not list us must not work here either. A skip
            # that quietly starts passing is as much a defect as one that
            # quietly starts failing — it means the list reports on work
            # already done. `05_http` had no ledger at all until 2026-08-31.
            expected = doc.get("implementations", ["rust", "python", "typescript"])
            if "python" not in expected:
                cases.append(
                    HttpFixtureCase(
                        fixture_name=fixture_name,
                        case_name="not_listed_for_python",
                        matcher=None,
                        request=None,
                        expect=None,
                        doc=doc,
                        unlisted=True,
                    )
                )
                continue

            # A fixture may assert that the config is *refused*. `error_contains`
            # is required rather than optional: without it the fixture passes on
            # any failure at all, so one that starts failing earlier — a typo in
            # the fixture itself — stays green while no longer testing what it
            # was written for. The protojson runner learned this the hard way.
            if doc.get("expect_error", False):
                cases.append(
                    HttpFixtureCase(
                        fixture_name=fixture_name,
                        case_name="compile_is_refused",
                        matcher=None,
                        request=None,
                        expect=None,
                        error_contains=doc["error_contains"],
                        doc=doc,
                    )
                )
                continue

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
