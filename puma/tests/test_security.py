"""Security regression tests.

Each test here corresponds to a finding in
``reference/security-review-2026-08-16.md``. They exist so that a fix cannot be
undone silently -- the review itself was nearly lost, and prose findings do not
fail a build.
"""

from __future__ import annotations

import pytest

from xuma._matcher import MatcherError

# ── Regression: SEC2 / review F-02, and L-4 ──────────────────────────────────
#
# Limits used to live only in the config loader, so the HTTP gateway and direct
# construction were unguarded. The review measured the gateway accepting a
# 40,960-byte regex against the 4,096 limit.


class TestLimitsLiveInTheConstructor:
    def test_oversized_regex_is_rejected_by_the_constructor(self) -> None:
        from xuma._limits import MAX_REGEX_PATTERN_LENGTH
        from xuma._string_matchers import RegexMatcher

        with pytest.raises(MatcherError, match="exceeds maximum"):
            RegexMatcher("a" * (MAX_REGEX_PATTERN_LENGTH + 1))

    def test_non_str_pattern_stays_inside_the_error_contract(self) -> None:
        """L-4: google-re2 raises a bare TypeError, outside MatcherError."""
        from xuma._string_matchers import RegexMatcher

        with pytest.raises(MatcherError):
            RegexMatcher(123)  # type: ignore[arg-type]

    def test_the_guard_is_not_inert(self) -> None:
        """A constructor that rejected everything would pass the tests above."""
        from xuma._limits import MAX_REGEX_PATTERN_LENGTH
        from xuma._string_matchers import RegexMatcher

        assert RegexMatcher("^user-[0-9]+$").matches("user-42")
        assert RegexMatcher("a" * MAX_REGEX_PATTERN_LENGTH) is not None

    def test_google_re2_rejects_the_compile_bomb(self) -> None:
        """Why puma is immune to SEC1 / review F-01.

        Documented as a test so that swapping google-re2 for a pure-Python
        engine fails loudly rather than silently removing the protection.
        """
        from xuma._string_matchers import RegexMatcher

        with pytest.raises(MatcherError):
            RegexMatcher("(a{100}){100}")


class TestCompilerWidthLimits:
    """The gateway compiler enforces the widths, not just the loader.

    rumi moved these onto ``Matcher::validate`` in #32 so that every
    construction path inherited them. puma was not carried across: until
    2026-08-23 ``compile_route_matches`` accepted a compound predicate of any
    width, because ``validate()`` checked depth only and the width limits lived
    in ``_registry``. A 257-route config compiled without complaint.
    """

    def test_compiler_rejects_more_routes_than_the_limit(self) -> None:
        from xuma._limits import MAX_PREDICATES_PER_COMPOUND
        from xuma._registry import TooManyPredicatesError
        from xuma.http import HttpPathMatch, HttpRouteMatch, compile_route_matches

        routes = [
            HttpRouteMatch(path=HttpPathMatch(type="Exact", value=f"/r{i}"))
            for i in range(MAX_PREDICATES_PER_COMPOUND + 1)
        ]
        with pytest.raises(TooManyPredicatesError):
            compile_route_matches(routes, "hit")

    def test_the_width_guard_is_not_inert(self) -> None:
        """A compiler that rejected everything would pass the test above."""
        from xuma._limits import MAX_PREDICATES_PER_COMPOUND
        from xuma.http import (
            HttpPathMatch,
            HttpRequest,
            HttpRouteMatch,
            compile_route_matches,
        )

        routes = [
            HttpRouteMatch(path=HttpPathMatch(type="Exact", value=f"/r{i}"))
            for i in range(MAX_PREDICATES_PER_COMPOUND - 1)
        ]
        matcher = compile_route_matches(routes, "hit")
        assert matcher.evaluate(HttpRequest(raw_path="/r0")) == "hit"

    def test_matcher_list_width_is_enforced_at_construction(self) -> None:
        from xuma import Action, ExactMatcher, FieldMatcher, Matcher, SinglePredicate
        from xuma._limits import MAX_FIELD_MATCHERS
        from xuma._registry import TooManyFieldMatchersError
        from xuma.testing import DictInput

        def fm(i: int) -> FieldMatcher[dict[str, str], str]:
            return FieldMatcher(
                SinglePredicate(DictInput("k"), ExactMatcher(f"v{i}")),
                Action("hit"),
            )

        with pytest.raises(TooManyFieldMatchersError):
            Matcher(tuple(fm(i) for i in range(MAX_FIELD_MATCHERS + 1)))
