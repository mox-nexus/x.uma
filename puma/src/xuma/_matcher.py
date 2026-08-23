"""Matcher — Top-level matcher with first-match-wins semantics.

Mirrors rumi's Matcher<Ctx, A> with the same xDS evaluation semantics:
- Field matchers evaluated in order (first-match-wins)
- OnMatch is exclusive: Action XOR NestedMatcher (per xDS proto)
- Nested matcher failure propagates (no fallback to sibling)
- on_no_match is the Matcher-level fallback
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

from xuma._limits import MAX_FIELD_MATCHERS, MAX_PREDICATES_PER_COMPOUND
from xuma._predicate import And, Not, Or, Predicate, predicate_depth

if TYPE_CHECKING:
    from xuma._types import DataInput

MAX_DEPTH = 32


class MatcherError(Exception):
    """Errors from matcher validation."""



class TooManyFieldMatchersError(MatcherError):
    """A matcher list is wider than ``MAX_FIELD_MATCHERS``."""

    def __init__(self, count: int, max_: int) -> None:
        self.count = count
        self.max = max_
        super().__init__(
            f"too many field matchers: {count} exceeds maximum {max_}"
        )


class TooManyPredicatesError(MatcherError):
    """A compound predicate is wider than ``MAX_PREDICATES_PER_COMPOUND``."""

    def __init__(self, count: int, max_: int) -> None:
        self.count = count
        self.max = max_
        super().__init__(
            f"too many predicates in compound: {count} exceeds maximum {max_}"
        )


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
class MatcherTree[Ctx, A]:
    """Map-based matching — xDS ``Matcher.MatcherTree``.

    Extracts a key via a DataInput, then looks it up either exactly or by
    longest matching prefix. The prefix rule is the one behaviour a matcher
    list cannot express: a list is first-match-wins in written order, so it
    returns ``/api`` for ``/api/v2`` whenever ``/api`` is listed first.

    Carries no fallback — the enclosing Matcher owns it. See DECISIONS.md
    D-044.

    rumi backs the prefix rule with a radix tree, O(k) in the key length. This
    scans the entries instead, O(n·k). The conformance suite pins behaviour,
    not the data structure, and puma is the readable implementation; if prefix
    lookup ever shows up in a puma profile, that is when to build the trie.
    """

    input: DataInput[Ctx]
    rule: str  # "exact" or "prefix"
    entries: tuple[tuple[str, OnMatch[Ctx, A]], ...]

    def lookup(self, key: str) -> tuple[str, OnMatch[Ctx, A]] | None:
        """The entry a key selects, and which entry key won."""
        if self.rule == "exact":
            for k, om in self.entries:
                if k == key:
                    return (k, om)
            return None

        best: tuple[str, OnMatch[Ctx, A]] | None = None
        for k, om in self.entries:
            if key.startswith(k) and (best is None or len(k) > len(best[0])):
                best = (k, om)
        return best

    def key_for(self, ctx: Any) -> str | None:
        """The lookup key, or None if the input produced no usable string."""
        data = self.input.get(ctx)
        return data if isinstance(data, str) else None

    def evaluate(self, ctx: Any) -> A | None:
        """Look up and dispatch. A miss is None; the Matcher owns the fallback."""
        key = self.key_for(ctx)
        if key is None:
            return None
        hit = self.lookup(key)
        if hit is None:
            return None
        return _evaluate_on_match(hit[1], ctx)

    def depth(self) -> int:
        """Deepest nesting reachable through this tree's entries.

        Entries hold OnMatch, which can hold a Matcher, which can hold another
        tree. Not walking this is what let such a config report depth 1 and
        pass validation — see DECISIONS.md D-045.
        """
        return max((_on_match_depth(om) for _, om in self.entries), default=0)


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

    matcher_list: tuple[FieldMatcher[Ctx, A], ...] = ()
    on_no_match: OnMatch[Ctx, A] | None = None
    # xDS models this as `oneof matcher_type`: a list or a tree, never both.
    tree: MatcherTree[Ctx, A] | None = None

    def __post_init__(self) -> None:
        self.validate()

    def evaluate(self, ctx: Any) -> A | None:
        """Evaluate this matcher against a context.

        Returns the matched action, or None if nothing matches and
        there is no on_no_match fallback.
        """
        if self.tree is not None:
            # A tree miss and a tree hit whose nested matcher returned None
            # both arrive as None, and both then reach on_no_match — the same
            # rule the list follows when it falls off the end.
            result = self.tree.evaluate(ctx)
            if result is not None:
                return result
        else:
            for fm in self.matcher_list:
                if fm.predicate.evaluate(ctx):
                    result = _evaluate_on_match(fm.on_match, ctx)
                    if result is not None:
                        return result
                    # xDS: nested matcher failure -> continue to the next one.
        if self.on_no_match is not None:
            return _evaluate_on_match(self.on_no_match, ctx)
        return None

    def validate(self) -> None:
        """Validate matcher depth and width against the declared limits.

        Should be called at config load time, not evaluation time.

        Raises:
            MatcherError: If depth or any width exceeds its limit.
        """
        d = self.depth()
        if d > MAX_DEPTH:
            msg = f"matcher depth {d} exceeds maximum allowed depth {MAX_DEPTH}"
            raise MatcherError(msg)
        self._validate_widths()

    def _validate_widths(self) -> None:
        """Reject a list or compound predicate wider than its limit.

        The widths lived only in the registry, so every path that did not go
        through ``load_matcher`` — the gateway compiler above all — accepted a
        matcher of any width. rumi closed this in #32 by moving the checks onto
        ``Matcher::validate``; puma and bumi were not carried across, and a
        257-child compound compiled here without complaint until 2026-08-23.
        The rule the security review named: the type that holds the resource
        owns the limit on that resource.
        """
        if self.tree is None and len(self.matcher_list) > MAX_FIELD_MATCHERS:
            raise TooManyFieldMatchersError(
                len(self.matcher_list), MAX_FIELD_MATCHERS
            )
        for fm in self.matcher_list:
            _validate_predicate_width(fm.predicate)
            if isinstance(fm.on_match, NestedMatcher):
                fm.on_match.matcher.validate()
        if isinstance(self.on_no_match, NestedMatcher):
            self.on_no_match.matcher.validate()
        # A tree's entries are bounded by MAX_TREE_ENTRIES at load, for a
        # different reason — see _limits. Its nested matchers still validate
        # themselves at construction.

    def depth(self) -> int:
        """Calculate the total nesting depth of this matcher tree."""
        if self.tree is not None:
            body_depth = self.tree.depth()
        else:
            max_predicate = max(
                (predicate_depth(fm.predicate) for fm in self.matcher_list),
                default=0,
            )
            max_nested = max(
                (_on_match_depth(fm.on_match) for fm in self.matcher_list),
                default=0,
            )
            body_depth = max(max_predicate, max_nested)
        no_match_depth = _on_match_depth(self.on_no_match) if self.on_no_match else 0
        return 1 + max(body_depth, no_match_depth)


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


def _validate_predicate_width(p: Predicate[Any]) -> None:
    """Recursively bound compound-predicate width. See ``Matcher.validate``."""
    match p:
        case And(predicates=ps) | Or(predicates=ps):
            if len(ps) > MAX_PREDICATES_PER_COMPOUND:
                raise TooManyPredicatesError(len(ps), MAX_PREDICATES_PER_COMPOUND)
            for sub in ps:
                _validate_predicate_width(sub)
        case Not(predicate=inner):
            _validate_predicate_width(inner)
        case _:
            pass
