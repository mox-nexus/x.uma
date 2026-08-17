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
