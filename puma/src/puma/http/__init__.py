"""puma.http — HTTP matching domain.

Provides HttpRequest context, DataInput implementations,
a Gateway API compiler for route matching, and registry
registration for config-driven construction.
"""

from puma.http._gateway import (
    HttpHeaderMatch,
    HttpPathMatch,
    HttpQueryParamMatch,
    HttpRouteMatch,
    compile_route_matches,
)
from puma.http._inputs import HeaderInput, MethodInput, PathInput, QueryParamInput
from puma.http._registry import register
from puma.http._request import HttpRequest

__all__ = [
    # Context
    "HttpRequest",
    # DataInputs
    "PathInput",
    "MethodInput",
    "HeaderInput",
    "QueryParamInput",
    # Gateway API types
    "HttpPathMatch",
    "HttpHeaderMatch",
    "HttpQueryParamMatch",
    "HttpRouteMatch",
    "compile_route_matches",
    # Registry
    "register",
]
