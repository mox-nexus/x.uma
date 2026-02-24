"""Tests for Matcher, FieldMatcher, OnMatch, and depth validation."""

from __future__ import annotations

import pytest

from xuma import (
    MAX_DEPTH,
    Action,
    ExactMatcher,
    FieldMatcher,
    Matcher,
    MatcherError,
    NestedMatcher,
    SinglePredicate,
)
from xuma.testing import DictInput


class TestMatcher:
    def test_first_match_wins(self) -> None:
        m = Matcher(
            matcher_list=(
                FieldMatcher(
                    SinglePredicate(DictInput("x"), ExactMatcher("a")),
                    Action("first"),
                ),
                FieldMatcher(
                    SinglePredicate(DictInput("x"), ExactMatcher("a")),
                    Action("second"),
                ),
            ),
        )
        assert m.evaluate({"x": "a"}) == "first"

    def test_no_match_returns_none(self) -> None:
        m = Matcher(
            matcher_list=(
                FieldMatcher(
                    SinglePredicate(DictInput("x"), ExactMatcher("a")),
                    Action("hit"),
                ),
            ),
        )
        assert m.evaluate({"x": "b"}) is None

    def test_on_no_match_fallback(self) -> None:
        m = Matcher(
            matcher_list=(
                FieldMatcher(
                    SinglePredicate(DictInput("x"), ExactMatcher("a")),
                    Action("hit"),
                ),
            ),
            on_no_match=Action("default"),
        )
        assert m.evaluate({"x": "b"}) == "default"

    def test_empty_matcher(self) -> None:
        m: Matcher[dict[str, str], str] = Matcher(matcher_list=())
        assert m.evaluate({"x": "a"}) is None

    def test_empty_matcher_with_fallback(self) -> None:
        m: Matcher[dict[str, str], str] = Matcher(
            matcher_list=(),
            on_no_match=Action("default"),
        )
        assert m.evaluate({}) == "default"


class TestNestedMatcher:
    def test_nested_match(self) -> None:
        inner = Matcher(
            matcher_list=(
                FieldMatcher(
                    SinglePredicate(DictInput("y"), ExactMatcher("b")),
                    Action("nested_hit"),
                ),
            ),
        )
        outer = Matcher(
            matcher_list=(
                FieldMatcher(
                    SinglePredicate(DictInput("x"), ExactMatcher("a")),
                    NestedMatcher(inner),
                ),
            ),
        )
        assert outer.evaluate({"x": "a", "y": "b"}) == "nested_hit"

    def test_nested_failure_propagates(self) -> None:
        """When nested matcher returns None, outer continues to next rule."""
        inner = Matcher(
            matcher_list=(
                FieldMatcher(
                    SinglePredicate(DictInput("y"), ExactMatcher("b")),
                    Action("nested_hit"),
                ),
            ),
        )
        outer = Matcher(
            matcher_list=(
                FieldMatcher(
                    SinglePredicate(DictInput("x"), ExactMatcher("a")),
                    NestedMatcher(inner),
                ),
                FieldMatcher(
                    SinglePredicate(DictInput("x"), ExactMatcher("a")),
                    Action("fallthrough"),
                ),
            ),
        )
        # x=a matches, but nested fails (y != b), so continues to second rule
        assert outer.evaluate({"x": "a", "y": "nope"}) == "fallthrough"


class TestDepthValidation:
    def test_shallow_passes(self) -> None:
        """Shallow matchers auto-validate at construction without error."""
        Matcher(
            matcher_list=(
                FieldMatcher(
                    SinglePredicate(DictInput("x"), ExactMatcher("a")),
                    Action("hit"),
                ),
            ),
        )

    def test_at_max_depth_passes(self) -> None:
        """Matcher at exactly MAX_DEPTH auto-validates without error."""
        inner: Matcher[dict[str, str], str] = Matcher(
            matcher_list=(
                FieldMatcher(
                    SinglePredicate(DictInput("x"), ExactMatcher("a")),
                    Action("deep"),
                ),
            ),
        )
        current = inner
        while current.depth() < MAX_DEPTH:
            current = Matcher(
                matcher_list=(
                    FieldMatcher(
                        SinglePredicate(DictInput("x"), ExactMatcher("a")),
                        NestedMatcher(current),
                    ),
                ),
            )
        assert current.depth() == MAX_DEPTH

    def test_exceeds_max_depth_raises_at_construction(self) -> None:
        """Auto-validation rejects depth > MAX_DEPTH at construction time."""
        inner: Matcher[dict[str, str], str] = Matcher(
            matcher_list=(
                FieldMatcher(
                    SinglePredicate(DictInput("x"), ExactMatcher("a")),
                    Action("deep"),
                ),
            ),
        )
        current = inner
        while current.depth() < MAX_DEPTH:
            current = Matcher(
                matcher_list=(
                    FieldMatcher(
                        SinglePredicate(DictInput("x"), ExactMatcher("a")),
                        NestedMatcher(current),
                    ),
                ),
            )
        assert current.depth() == MAX_DEPTH
        # One more nesting level raises MatcherError at construction
        with pytest.raises(MatcherError, match="exceeds"):
            Matcher(
                matcher_list=(
                    FieldMatcher(
                        SinglePredicate(DictInput("x"), ExactMatcher("a")),
                        NestedMatcher(current),
                    ),
                ),
            )
