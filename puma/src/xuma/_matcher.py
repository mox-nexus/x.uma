"""Matcher — Top-level matcher with first-match-wins semantics.

Mirrors rumi's Matcher<Ctx, A> with the same xDS evaluation semantics:
- Field matchers evaluated in order (first-match-wins)
- OnMatch is exclusive: Action XOR NestedMatcher (per xDS proto)
- Nested matcher failure propagates (no fallback to sibling)
- on_no_match is the Matcher-level fallback
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from xuma._predicate import Predicate, predicate_depth

MAX_DEPTH = 32


class MatcherError(Exception):
    """Errors from matcher validation."""


@dataclass(frozen=True, slots=True)
class Action[A]:
    """Execute this action when matched.

    Wraps the action value in an OnMatch variant.
    Per xDS, OnMatch is exclusive — Action XOR NestedMatcher.
    """

    value: A


@dataclass(frozen=True, slots=True)
class NestedMatcher[Ctx, A]:
    """Continue evaluation into a nested matcher.

    If the nested matcher returns None, this OnMatch also returns None
    (xDS semantics: nested matcher failure propagates).
    """

    matcher: Matcher[Ctx, A]


# xDS OnMatch exclusivity — Action XOR NestedMatcher, never both.
type OnMatch[Ctx, A] = Action[A] | NestedMatcher[Ctx, A]


@dataclass(frozen=True, slots=True)
class FieldMatcher[Ctx, A]:
    """Pairs a predicate with an OnMatch outcome.

    If the predicate evaluates to True, the OnMatch is consulted.
    """

    predicate: Predicate[Ctx]
    on_match: OnMatch[Ctx, A]


@dataclass(frozen=True, slots=True)
class Matcher[Ctx, A]:
    """Top-level matcher with first-match-wins semantics.

    Evaluates field matchers in order and returns the action from
    the first matching predicate. If no predicate matches, returns
    the on_no_match fallback (if present).

    Depth validation runs automatically at construction time.
    If the matcher tree exceeds MAX_DEPTH (32), MatcherError is raised.

    INV (Dijkstra): First-match-wins — later matches are never consulted.
    """

    matcher_list: tuple[FieldMatcher[Ctx, A], ...]
    on_no_match: OnMatch[Ctx, A] | None = None

    def __post_init__(self) -> None:
        self.validate()

    def evaluate(self, ctx: Any) -> A | None:
        """Evaluate this matcher against a context.

        Returns the matched action, or None if nothing matches and
        there is no on_no_match fallback.
        """
        for fm in self.matcher_list:
            if fm.predicate.evaluate(ctx):
                result = _evaluate_on_match(fm.on_match, ctx)
                if result is not None:
                    return result
                # xDS: nested matcher failure -> continue to next field_matcher
        if self.on_no_match is not None:
            return _evaluate_on_match(self.on_no_match, ctx)
        return None

    def validate(self) -> None:
        """Validate matcher depth does not exceed MAX_DEPTH.

        Should be called at config load time, not evaluation time.

        Raises:
            MatcherError: If depth exceeds MAX_DEPTH.
        """
        d = self.depth()
        if d > MAX_DEPTH:
            msg = f"matcher depth {d} exceeds maximum allowed depth {MAX_DEPTH}"
            raise MatcherError(msg)

    def depth(self) -> int:
        """Calculate the total nesting depth of this matcher tree."""
        max_predicate = max(
            (predicate_depth(fm.predicate) for fm in self.matcher_list),
            default=0,
        )
        max_nested = max(
            (_on_match_depth(fm.on_match) for fm in self.matcher_list),
            default=0,
        )
        no_match_depth = _on_match_depth(self.on_no_match) if self.on_no_match else 0
        return 1 + max(max_predicate, max_nested, no_match_depth)


def matcher_from_predicate[Ctx, A](
    predicate: Predicate[Ctx],
    action: A,
    on_no_match: A | None = None,
) -> Matcher[Ctx, A]:
    """Create a Matcher from a single predicate, action, and optional fallback.

    This is the standard way to wrap a predicate tree into a ready-to-evaluate
    Matcher. Eliminates repeated Matcher(matcher_list=(...), on_no_match=...) boilerplate.
    """
    on_no_match_om = Action(on_no_match) if on_no_match is not None else None
    return Matcher(
        matcher_list=(FieldMatcher(predicate, Action(action)),),
        on_no_match=on_no_match_om,
    )


def _evaluate_on_match[A](on_match: OnMatch[Any, A], ctx: Any) -> A | None:
    """Evaluate an OnMatch variant."""
    match on_match:
        case Action(value=v):
            return v
        case NestedMatcher(matcher=m):
            return m.evaluate(ctx)
    return None  # pragma: no cover


def _on_match_depth(on_match: OnMatch[Any, Any]) -> int:
    """Calculate depth contribution of an OnMatch."""
    match on_match:
        case Action():
            return 0
        case NestedMatcher(matcher=m):
            return m.depth()
    return 0  # pragma: no cover
