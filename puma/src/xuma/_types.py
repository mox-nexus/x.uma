"""Core protocols and type aliases for xuma.

The type system mirrors rumi's architecture:
- MatchingData is the type-erased data union (same name across all implementations)
- DataInput is the domain-specific extraction port
- InputMatcher is the domain-agnostic matching port
"""

from __future__ import annotations

from typing import Protocol, TypeVar, runtime_checkable

# The erased data type — Python's union replaces Rust's MatchingData enum.
# None maps to MatchingData::None (triggers the None -> false invariant).
MatchingData = str | int | bool | bytes | None

Ctx = TypeVar("Ctx", contravariant=True)


@runtime_checkable
class DataInput(Protocol[Ctx]):
    """Extract a value from a domain-specific context.

    Implementations are domain-specific (HTTP, Claude, test) but return
    the domain-agnostic MatchingData type.

    Returning None signals "data not available" and causes the predicate
    to evaluate to False (the None -> false invariant).
    """

    def get(self, ctx: Ctx, /) -> MatchingData: ...


@runtime_checkable
class InputMatcher(Protocol):
    """Match against a type-erased value.

    InputMatcher is intentionally non-generic — the same ExactMatcher works
    for HTTP, Claude, test contexts, etc. This is the key design insight
    from Envoy's matcher architecture.
    """

    def matches(self, value: MatchingData, /) -> bool: ...
