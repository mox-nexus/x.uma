"""Conformance over the protojson fixtures.

`spec/tests/07_protojson/` holds the format x.uma actually implements —
protobuf's canonical JSON mapping of `xds.type.matcher.v3.Matcher`. The four
older dialects are transitional.

Each fixture names the implementations expected to run it, and this runner
holds that ledger in **both** directions: if `python` is listed the fixture must
run, and if it is not listed the fixture must *fail* to run. A skip that quietly
starts working means the ledger is reporting on work somebody already finished,
and a suite that lies about its own coverage is worse than one that is red.

Run with: cd puma && uv run pytest tests/test_proto_conformance.py -v
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pytest
import yaml

from xuma import (
    ConfigParseError,
    MatcherError,
    RegistryBuilder,
    parse_protojson,
)
from xuma.http import HttpRequest
from xuma.http import register as register_http
from xuma.testing import register

ME = "python"
SPEC_DIR = Path(__file__).resolve().parent.parent.parent / "spec" / "tests"
PROTO_DIR = SPEC_DIR / "07_protojson"


def _load() -> list[dict[str, Any]]:
    fixtures: list[dict[str, Any]] = []
    if not PROTO_DIR.exists():
        return fixtures
    for yaml_file in sorted(PROTO_DIR.glob("*.yaml")):
        with yaml_file.open() as f:
            for doc in yaml.safe_load_all(f):
                if doc and "proto_matcher" in doc:
                    fixtures.append(doc)
    return fixtures


FIXTURES = _load()


def _build(fixture: dict[str, Any]):  # noqa: ANN202
    """Load the fixture's matcher, or raise saying why it could not be built.

    The matcher config is domain-agnostic; only the registry differs.
    """
    domain = fixture.get("domain", "kv")
    builder = register_http(RegistryBuilder()) if domain == "http" else register(RegistryBuilder())
    return builder.build().load_matcher(parse_protojson(fixture["proto_matcher"]))


def _context(fixture: dict[str, Any], case: dict[str, Any]):  # noqa: ANN202
    """Build the context a case evaluates against, for its fixture's domain."""
    if fixture.get("domain", "kv") == "http":
        spec = case.get("http_request")
        if spec is None:
            msg = f"case {case.get('name')!r} is in the http domain but has no http_request"
            raise AssertionError(msg)
        return HttpRequest(
            method=spec.get("method", ""),
            raw_path=spec.get("path", ""),
            headers=dict(spec.get("headers", {})),
        )
    return dict(case.get("context", {}))


def _ids() -> list[str]:
    return [f.get("name", f"fixture-{i}") for i, f in enumerate(FIXTURES)]


def test_the_corpus_is_not_empty() -> None:
    """A runner over zero fixtures passes and proves nothing."""
    assert FIXTURES, f"no protojson fixtures found under {PROTO_DIR}"


@pytest.mark.parametrize("fixture", FIXTURES, ids=_ids())
def test_protojson_fixture(fixture: dict[str, Any]) -> None:
    expected = fixture.get("implementations", ["rust", "python", "typescript"])
    name = fixture.get("name", "<unnamed>")

    if ME not in expected:
        # Not listed, so it must not work. See the module docstring.
        with pytest.raises((ConfigParseError, MatcherError, Exception)):
            _build(fixture)
            pytest.fail(
                f"fixture {name!r} does not list {ME}, but {ME} loads it. Add {ME} to "
                f"`implementations` — a stale exception hides a finished migration."
            )
        return

    if fixture.get("expect_error"):
        with pytest.raises((ConfigParseError, MatcherError)) as excinfo:
            _build(fixture)
        needle = fixture.get("error_contains")
        if needle:
            assert needle in str(excinfo.value), (
                f"fixture {name!r} failed for the wrong reason.\n"
                f"  wanted: {needle}\n  got:    {excinfo.value}"
            )
        return

    matcher = _build(fixture)
    for case in fixture.get("cases", []):
        actual = matcher.evaluate(_context(fixture, case))
        assert actual == case.get("expect"), (
            f"fixture {name!r} case {case.get('name')!r}: "
            f"expected {case.get('expect')!r}, got {actual!r}"
        )
