"""DataInput implementations for HttpRequest.

Each input extracts a specific field from an HttpRequest context
and returns it as a MatchingData for predicate evaluation.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from xuma._types import MatchingData
    from xuma.http._request import HttpRequest


@dataclass(frozen=True, slots=True)
class PathInput:
    """Extracts the request path (without query string)."""

    def get(self, ctx: HttpRequest, /) -> MatchingData:
        return ctx.path


@dataclass(frozen=True, slots=True)
class MethodInput:
    """Extracts the HTTP method (case-sensitive)."""

    def get(self, ctx: HttpRequest, /) -> MatchingData:
        return ctx.method


@dataclass(frozen=True, slots=True)
class HeaderInput:
    """Extracts a header value by name (case-insensitive lookup)."""

    name: str

    def get(self, ctx: HttpRequest, /) -> MatchingData:
        return ctx.header(self.name)


@dataclass(frozen=True, slots=True)
class QueryParamInput:
    """Extracts a query parameter value by name."""

    name: str

    def get(self, ctx: HttpRequest, /) -> MatchingData:
        return ctx.query_param(self.name)


@dataclass(frozen=True, slots=True)
class AuthorityInput:
    """Extracts the ``:authority`` pseudo-header (Host in HTTP/1).

    Read from the headers, which is where rumi's `HttpMessage` reads it from
    too. Without this and `SchemeInput`, four of the six ``xuma.http.v1.*``
    type URLs resolved here and six resolved in rumi — a type URL that loads in
    one implementation and not another.
    """

    def get(self, ctx: HttpRequest, /) -> MatchingData:
        return ctx.header(":authority")


@dataclass(frozen=True, slots=True)
class SchemeInput:
    """Extracts the ``:scheme`` pseudo-header (http/https)."""

    def get(self, ctx: HttpRequest, /) -> MatchingData:
        return ctx.header(":scheme")
