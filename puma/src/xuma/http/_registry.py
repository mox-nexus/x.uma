"""HTTP domain registration for the xuma registry.

Registers all HTTP-domain DataInput types so they can be constructed
from config via type_url lookup.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from xuma._registry import register_core_matchers
from xuma.http._inputs import (
    AuthorityInput,
    HeaderInput,
    MethodInput,
    PathInput,
    QueryParamInput,
    SchemeInput,
)

if TYPE_CHECKING:
    from xuma._registry import RegistryBuilder
    from xuma.http._request import HttpRequest


def register(
    builder: RegistryBuilder[HttpRequest],
) -> RegistryBuilder[HttpRequest]:
    """Register all HTTP-domain DataInput types.

    Type URLs follow the xuma namespace convention:
    - xuma.http.v1.PathInput
    - xuma.http.v1.MethodInput
    - xuma.http.v1.HeaderInput
    - xuma.http.v1.QueryParamInput
    """
    return (
        register_core_matchers(builder)
        .input("xuma.http.v1.PathInput", _path_factory)
        .input("xuma.http.v1.MethodInput", _method_factory)
        .input("xuma.http.v1.HeaderInput", _header_factory)
        .input("xuma.http.v1.QueryParamInput", _query_param_factory)
        .input("xuma.http.v1.AuthorityInput", _authority_factory)
        .input("xuma.http.v1.SchemeInput", _scheme_factory)
    )


def _authority_factory(_config: dict[str, Any]) -> AuthorityInput:
    return AuthorityInput()


def _scheme_factory(_config: dict[str, Any]) -> SchemeInput:
    return SchemeInput()


def _path_factory(_config: dict[str, Any]) -> PathInput:
    return PathInput()


def _method_factory(_config: dict[str, Any]) -> MethodInput:
    return MethodInput()


def _header_factory(config: dict[str, Any]) -> HeaderInput:
    name = config.get("name")
    if not isinstance(name, str) or not name:
        msg = "HeaderInput requires a non-empty 'name' field"
        raise ValueError(msg)
    return HeaderInput(name=name)


def _query_param_factory(config: dict[str, Any]) -> QueryParamInput:
    name = config.get("name")
    if not isinstance(name, str) or not name:
        msg = "QueryParamInput requires a non-empty 'name' field"
        raise ValueError(msg)
    return QueryParamInput(name=name)
