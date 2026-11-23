"""xuma-crust: Python bindings for rumi (Rust matcher engine).

Linear-time regex, zero-copy evaluation, type-safe config.
"""

from xuma_crust.xuma_crust import (
    Decision,
    HookMatcher,
    HttpMatcher,
    PyHookMatch as HookMatch,
    PyStringMatch as StringMatch,
    KvMatcher,
)

__all__ = ["Decision", "HookMatcher", "HookMatch", "HttpMatcher", "StringMatch", "KvMatcher"]
