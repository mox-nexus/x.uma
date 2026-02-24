"""Tests for concrete string matchers."""

import pytest

from xuma import (
    ContainsMatcher,
    ExactMatcher,
    MatcherError,
    PrefixMatcher,
    RegexMatcher,
    SuffixMatcher,
)


class TestExactMatcher:
    def test_exact_match(self) -> None:
        m = ExactMatcher("hello")
        assert m.matches("hello") is True

    def test_no_match(self) -> None:
        m = ExactMatcher("hello")
        assert m.matches("world") is False

    def test_case_sensitive_by_default(self) -> None:
        m = ExactMatcher("hello")
        assert m.matches("Hello") is False

    def test_ignore_case(self) -> None:
        m = ExactMatcher("hello", ignore_case=True)
        assert m.matches("HELLO") is True
        assert m.matches("Hello") is True
        assert m.matches("hello") is True

    def test_none_returns_false(self) -> None:
        m = ExactMatcher("hello")
        assert m.matches(None) is False

    def test_non_string_returns_false(self) -> None:
        m = ExactMatcher("42")
        assert m.matches(42) is False

    def test_empty_string(self) -> None:
        m = ExactMatcher("")
        assert m.matches("") is True
        assert m.matches("a") is False

    def test_partial_no_match(self) -> None:
        m = ExactMatcher("hello")
        assert m.matches("hello world") is False


class TestPrefixMatcher:
    def test_prefix_match(self) -> None:
        m = PrefixMatcher("/api")
        assert m.matches("/api/users") is True

    def test_exact_is_prefix(self) -> None:
        m = PrefixMatcher("/api")
        assert m.matches("/api") is True

    def test_no_match(self) -> None:
        m = PrefixMatcher("/api")
        assert m.matches("/other") is False

    def test_ignore_case(self) -> None:
        m = PrefixMatcher("/API", ignore_case=True)
        assert m.matches("/api/users") is True

    def test_none_returns_false(self) -> None:
        m = PrefixMatcher("/api")
        assert m.matches(None) is False

    def test_empty_prefix(self) -> None:
        m = PrefixMatcher("")
        assert m.matches("anything") is True


class TestSuffixMatcher:
    def test_suffix_match(self) -> None:
        m = SuffixMatcher(".json")
        assert m.matches("data.json") is True

    def test_no_match(self) -> None:
        m = SuffixMatcher(".json")
        assert m.matches("data.xml") is False

    def test_ignore_case(self) -> None:
        m = SuffixMatcher(".JSON", ignore_case=True)
        assert m.matches("data.json") is True

    def test_none_returns_false(self) -> None:
        m = SuffixMatcher(".json")
        assert m.matches(None) is False


class TestContainsMatcher:
    def test_contains_match(self) -> None:
        m = ContainsMatcher("world")
        assert m.matches("hello world") is True

    def test_no_match(self) -> None:
        m = ContainsMatcher("xyz")
        assert m.matches("hello world") is False

    def test_ignore_case(self) -> None:
        m = ContainsMatcher("WORLD", ignore_case=True)
        assert m.matches("hello world") is True

    def test_none_returns_false(self) -> None:
        m = ContainsMatcher("x")
        assert m.matches(None) is False

    def test_empty_substring_always_matches(self) -> None:
        m = ContainsMatcher("")
        assert m.matches("anything") is True


class TestRegexMatcher:
    def test_regex_match(self) -> None:
        m = RegexMatcher(r"^\d+$")
        assert m.matches("12345") is True

    def test_no_match(self) -> None:
        m = RegexMatcher(r"^\d+$")
        assert m.matches("abc") is False

    def test_search_not_fullmatch(self) -> None:
        m = RegexMatcher(r"\d+")
        assert m.matches("abc123def") is True

    def test_none_returns_false(self) -> None:
        m = RegexMatcher(r"\d+")
        assert m.matches(None) is False

    def test_invalid_regex_raises(self) -> None:
        with pytest.raises(MatcherError):
            RegexMatcher(r"[invalid")

    def test_backreference_rejected_by_re2(self) -> None:
        """RE2 rejects backreferences — this is what the migration prevents."""
        with pytest.raises(MatcherError):
            RegexMatcher(r"(a)\1")
