"""Test utilities for xuma.

Provides convenience DataInput implementations for use in tests and examples.
These are NOT domain adapters — they exist to reduce boilerplate when
exploring xuma with dict-shaped contexts.

For real domains, implement DataInput for your own context type.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from xuma._registry import RegistryBuilder
    from xuma._types import MatchingData


@dataclass(frozen=True, slots=True)
class DictInput:
    """Extract a value by key from a dict context.

    The simplest possible DataInput — useful for tests, examples, and
    quick exploration without defining a custom context type.

    >>> from xuma import SinglePredicate, ExactMatcher
    >>> from xuma.testing import DictInput
    >>> p = SinglePredicate(DictInput("name"), ExactMatcher("alice"))
    >>> p.evaluate({"name": "alice"})
    True
    """

    key: str

    def get(self, ctx: dict[str, str], /) -> MatchingData:
        return ctx.get(self.key)


def register(
    builder: RegistryBuilder[dict[str, str]],
) -> RegistryBuilder[dict[str, str]]:
    """Register the test-domain DictInput type.

    Type URL: xuma.test.v1.StringInput (matches rumi-test convention).
    Config field: { "key": "field_name" }
    """
    return builder.input("xuma.test.v1.StringInput", _dict_input_factory)


def _dict_input_factory(config: dict[str, Any]) -> DictInput:
    key = config.get("key")
    if not isinstance(key, str):
        msg = "DictInput requires a 'key' field (string)"
        raise ValueError(msg)
    return DictInput(key=key)
