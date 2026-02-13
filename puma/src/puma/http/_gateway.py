"""Gateway API compiler — HttpRouteMatch -> Matcher[HttpRequest, A].

Translates Gateway API-style route configuration into puma Matcher trees.
Pure Python types mirroring the Gateway API spec (no k8s dependency).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Literal

from puma._matcher import Matcher, matcher_from_predicate
from puma._predicate import Predicate, SinglePredicate, and_predicate, or_predicate
from puma._string_matchers import ExactMatcher, PrefixMatcher, RegexMatcher
from puma.http._inputs import HeaderInput, MethodInput, PathInput, QueryParamInput

if TYPE_CHECKING:
    from puma.http._request import HttpRequest


def _catch_all() -> Predicate[HttpRequest]:
    """A catch-all predicate that matches any HTTP request."""
    return SinglePredicate(PathInput(), PrefixMatcher(""))


@dataclass(frozen=True, slots=True)
class HttpPathMatch:
    """Gateway API path match specification."""

    type: Literal["Exact", "PathPrefix", "RegularExpression"]
    value: str


@dataclass(frozen=True, slots=True)
class HttpHeaderMatch:
    """Gateway API header match specification."""

    type: Literal["Exact", "RegularExpression"]
    name: str
    value: str


@dataclass(frozen=True, slots=True)
class HttpQueryParamMatch:
    """Gateway API query parameter match specification."""

    type: Literal["Exact", "RegularExpression"]
    name: str
    value: str


@dataclass(frozen=True, slots=True)
class HttpRouteMatch:
    """Gateway API HttpRouteMatch — configuration for route matching.

    All conditions within a single HttpRouteMatch are ANDed together.
    Multiple HttpRouteMatch entries are ORed (via compile_route_matches).
    """

    path: HttpPathMatch | None = None
    method: str | None = None
    headers: list[HttpHeaderMatch] = field(default_factory=list)
    query_params: list[HttpQueryParamMatch] = field(default_factory=list)

    def compile[A](self, action: A) -> Matcher[HttpRequest, A]:
        """Compile this route match into a Matcher with the given action."""
        return matcher_from_predicate(self.to_predicate(), action)

    def to_predicate(self) -> Predicate[HttpRequest]:
        """Convert this route match to a predicate tree."""
        predicates: list[SinglePredicate[HttpRequest]] = []

        if self.path is not None:
            predicates.append(_compile_path_match(self.path))

        if self.method is not None:
            predicates.append(SinglePredicate(MethodInput(), ExactMatcher(self.method)))

        for header_match in self.headers:
            predicates.append(_compile_header_match(header_match))

        for query_match in self.query_params:
            predicates.append(_compile_query_param_match(query_match))

        return and_predicate(predicates, _catch_all())


def compile_route_matches[A](
    matches: list[HttpRouteMatch],
    action: A,
    on_no_match: A | None = None,
) -> Matcher[HttpRequest, A]:
    """Compile multiple HttpRouteMatch entries into a single Matcher.

    Multiple matches are ORed together per Gateway API semantics.
    """
    predicates = [m.to_predicate() for m in matches]
    return matcher_from_predicate(
        or_predicate(predicates, _catch_all()),
        action,
        on_no_match,
    )


def _compile_path_match(path_match: HttpPathMatch) -> SinglePredicate[HttpRequest]:
    """Compile a path match to a predicate."""
    match path_match.type:
        case "Exact":
            return SinglePredicate(PathInput(), ExactMatcher(path_match.value))
        case "PathPrefix":
            return SinglePredicate(PathInput(), PrefixMatcher(path_match.value))
        case "RegularExpression":
            return SinglePredicate(PathInput(), RegexMatcher(path_match.value))
        case _:
            msg = f"Unknown path match type: {path_match.type}"
            raise ValueError(msg)


def _compile_header_match(header_match: HttpHeaderMatch) -> SinglePredicate[HttpRequest]:
    """Compile a header match to a predicate."""
    input_ = HeaderInput(header_match.name)
    match header_match.type:
        case "Exact":
            return SinglePredicate(input_, ExactMatcher(header_match.value))
        case "RegularExpression":
            return SinglePredicate(input_, RegexMatcher(header_match.value))
        case _:
            msg = f"Unknown header match type: {header_match.type}"
            raise ValueError(msg)


def _compile_query_param_match(query_match: HttpQueryParamMatch) -> SinglePredicate[HttpRequest]:
    """Compile a query param match to a predicate."""
    input_ = QueryParamInput(query_match.name)
    match query_match.type:
        case "Exact":
            return SinglePredicate(input_, ExactMatcher(query_match.value))
        case "RegularExpression":
            return SinglePredicate(input_, RegexMatcher(query_match.value))
        case _:
            msg = f"Unknown query param match type: {query_match.type}"
            raise ValueError(msg)
