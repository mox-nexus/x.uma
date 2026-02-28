"""xuma-crust: Python bindings for rumi (Rust matcher engine).

Linear-time regex, zero-copy evaluation, type-safe config.
"""

from xuma_crust.xuma_crust import (
    HookMatcher,
    HttpMatcher,
    PyHookMatch as HookMatch,
    PyStringMatch as StringMatch,
    TestMatcher,
)

__all__ = ["HookMatcher", "HookMatch", "HttpMatcher", "StringMatch", "TestMatcher"]
