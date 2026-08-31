"""HTTP conformance tests — fixtures 05.

Runs all YAML fixtures from spec/tests/05_http against the puma.http
matching engine (HttpRequest + Gateway API compiler).
"""

from __future__ import annotations

import pytest

from tests.conftest import HttpFixtureCase, load_http_fixtures
from xuma._matcher import MatcherError
from xuma.http import compile_route_matches

_HTTP_FIXTURES = load_http_fixtures()


@pytest.mark.parametrize(
    "fixture",
    _HTTP_FIXTURES,
    ids=lambda f: f"{f.fixture_name}::{f.case_name}",
)
def test_http_conformance(fixture: HttpFixtureCase) -> None:
    """Each HTTP fixture case must produce the expected action (or None)."""
    if fixture.unlisted:
        assert fixture.doc is not None
        from tests.conftest import _compile_http_fixture

        with pytest.raises(Exception):  # noqa: B017,PT011 - any failure is the point
            _compile_http_fixture(
                fixture.doc, fixture.doc["action"], fixture.doc.get("on_no_match")
            )
            pytest.fail(
                f"fixture {fixture.fixture_name!r} does not list python, but python "
                f"compiles it. Add python to `implementations` — a stale exception "
                f"hides a finished migration."
            )
        return

    if fixture.error_contains is not None:
        assert fixture.doc is not None
        from tests.conftest import _compile_http_fixture

        with pytest.raises(MatcherError) as excinfo:
            _compile_http_fixture(
                fixture.doc, fixture.doc["action"], fixture.doc.get("on_no_match")
            )
        assert fixture.error_contains in str(excinfo.value), (
            f"Fixture '{fixture.fixture_name}': error did not mention "
            f"{fixture.error_contains!r}, got {excinfo.value}"
        )
        return

    assert fixture.matcher is not None
    assert fixture.request is not None
    result = fixture.matcher.evaluate(fixture.request)
    assert result == fixture.expect, (
        f"Fixture '{fixture.fixture_name}', case '{fixture.case_name}': "
        f"expected {fixture.expect!r}, got {result!r}"
    )


def test_the_error_path_is_not_inert() -> None:
    """A runner that never reached the error branch would pass everything above."""
    assert any(f.error_contains is not None for f in _HTTP_FIXTURES), (
        "no HTTP fixture exercises expect_error — the branch above is dead"
    )
    with pytest.raises(MatcherError):
        compile_route_matches([], "hit")
