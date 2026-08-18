"""Concrete string matchers implementing the InputMatcher protocol.

Each matcher is a frozen dataclass — immutable after construction.
All matchers return False for non-string or None input values.

Regex uses ``google-re2`` for guaranteed linear-time matching. RE2 does not
support backreferences or lookahead/lookbehind because they require
backtracking — patterns using them are rejected at compile time.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

import re2

from xuma._limits import MAX_PATTERN_LENGTH, MAX_REGEX_PATTERN_LENGTH
from xuma._matcher import MatcherError

if TYPE_CHECKING:
    from xuma._types import MatchingData


def _check_literal_length(pattern: str) -> None:
    """Enforce the literal pattern limit at construction.

    The limit belongs to the type that holds the pattern, not to the config
    loader -- until 2026-08-17 only the loader checked, so the HTTP gateway and
    direct construction were unguarded.
    """
    if not isinstance(pattern, str):
        msg = f"pattern must be str, got {type(pattern).__name__}"
        raise MatcherError(msg)
    if len(pattern) > MAX_PATTERN_LENGTH:
        msg = f"pattern length {len(pattern)} exceeds maximum {MAX_PATTERN_LENGTH}"
        raise MatcherError(msg)


@dataclass(frozen=True, slots=True)
class ExactMatcher:
    """Exact string equality match.

    When ignore_case is True, comparison is case-insensitive.
    The comparison value is pre-lowercased at construction time.
    """

    value: str
    ignore_case: bool = False
    _cmp_value: str = field(init=False, repr=False)

    def __post_init__(self) -> None:
        _check_literal_length(self.value)
        object.__setattr__(
            self, "_cmp_value", self.value.casefold() if self.ignore_case else self.value
        )

    def matches(self, value: MatchingData, /) -> bool:
        if not isinstance(value, str):
            return False
        input_val = value.casefold() if self.ignore_case else value
        return input_val == self._cmp_value


@dataclass(frozen=True, slots=True)
class PrefixMatcher:
    """String prefix match (startswith).

    When ignore_case is True, comparison is case-insensitive.
    The prefix is pre-lowercased at construction time.
    """

    prefix: str
    ignore_case: bool = False
    _cmp_prefix: str = field(init=False, repr=False)

    def __post_init__(self) -> None:
        _check_literal_length(self.prefix)
        object.__setattr__(
            self, "_cmp_prefix", self.prefix.casefold() if self.ignore_case else self.prefix
        )

    def matches(self, value: MatchingData, /) -> bool:
        if not isinstance(value, str):
            return False
        input_val = value.casefold() if self.ignore_case else value
        return input_val.startswith(self._cmp_prefix)


@dataclass(frozen=True, slots=True)
class SuffixMatcher:
    """String suffix match (endswith).

    When ignore_case is True, comparison is case-insensitive.
    The suffix is pre-lowercased at construction time.
    """

    suffix: str
    ignore_case: bool = False
    _cmp_suffix: str = field(init=False, repr=False)

    def __post_init__(self) -> None:
        _check_literal_length(self.suffix)
        object.__setattr__(
            self, "_cmp_suffix", self.suffix.casefold() if self.ignore_case else self.suffix
        )

    def matches(self, value: MatchingData, /) -> bool:
        if not isinstance(value, str):
            return False
        input_val = value.casefold() if self.ignore_case else value
        return input_val.endswith(self._cmp_suffix)


@dataclass(frozen=True, slots=True)
class ContainsMatcher:
    """Substring search match.

    When ignore_case is True, comparison is case-insensitive.
    The substring pattern is pre-lowercased at construction time
    (Knuth optimization: avoid repeated lowercasing of the pattern).
    """

    substring: str
    ignore_case: bool = False
    _cmp_substring: str = field(init=False, repr=False)

    def __post_init__(self) -> None:
        _check_literal_length(self.substring)
        object.__setattr__(
            self,
            "_cmp_substring",
            self.substring.casefold() if self.ignore_case else self.substring,
        )

    def matches(self, value: MatchingData, /) -> bool:
        if not isinstance(value, str):
            return False
        input_val = value.casefold() if self.ignore_case else value
        return self._cmp_substring in input_val


@dataclass(frozen=True, slots=True)
class RegexMatcher:
    """Regular expression match.

    The pattern is compiled at construction time via ``google-re2``, providing
    guaranteed linear-time matching. Uses search (not fullmatch) to match
    anywhere in the string, consistent with rumi's behavior.

    RE2 does not support backreferences or lookahead/lookbehind because they
    require backtracking. Patterns using them are rejected at compile time.

    Raises:
        MatcherError: If the pattern is not valid RE2 syntax.
    """

    pattern: str
    _compiled: re2.Pattern[str] = field(init=False, repr=False)

    def __post_init__(self) -> None:
        # The limit is enforced here, in the constructor that owns the compiled
        # program, not in the config loader. Every caller inherits it -- the
        # registry, the HTTP gateway, and direct construction. Until 2026-08-17
        # the loader was the only guarded path, so the gateway accepted a
        # 40,960-byte regex against this 4,096 limit.
        if not isinstance(self.pattern, str):
            # google-re2 raises a bare TypeError for a non-str pattern, which
            # escapes the MatcherError contract callers rely on (review L-4).
            msg = f"regex pattern must be str, got {type(self.pattern).__name__}"
            raise MatcherError(msg)
        if len(self.pattern) > MAX_REGEX_PATTERN_LENGTH:
            msg = (
                f"pattern length {len(self.pattern)} exceeds maximum "
                f"{MAX_REGEX_PATTERN_LENGTH}"
            )
            raise MatcherError(msg)
        try:
            compiled = re2.compile(self.pattern)
        except re2.error as e:
            msg = f'invalid regex pattern "{self.pattern}": {e}'
            raise MatcherError(msg) from e
        except TypeError as e:
            # google-re2 raises TypeError for a non-str pattern, which escapes
            # the MatcherError contract callers rely on (review L-4).
            msg = f"invalid regex pattern {self.pattern!r}: {e}"
            raise MatcherError(msg) from e
        object.__setattr__(self, "_compiled", compiled)

    def matches(self, value: MatchingData, /) -> bool:
        if not isinstance(value, str):
            return False
        return self._compiled.search(value) is not None


@dataclass(frozen=True, slots=True)
class BoolMatcher:
    """Matches a boolean value.

    Reachable through ``custom_match``, which is the seam for comparisons xDS's
    own ``StringMatcher`` cannot express — a boolean is one, since
    ``value_match`` only compares strings.

    No input x.uma ships produces a boolean; this exists for a custom domain
    whose ``DataInput`` returns one. It is registered by
    :func:`~xuma._registry.register_core_matchers` so that
    ``xuma.core.v1.BoolMatcher`` resolves in all implementations rather than in
    some — that function previously registered nothing at all while its
    docstring said it did.
    """

    expected: bool

    def matches(self, value: MatchingData, /) -> bool:
        return isinstance(value, bool) and value == self.expected
