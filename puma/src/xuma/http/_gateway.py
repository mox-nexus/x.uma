"""Gateway API compiler — HttpRouteMatch -> Matcher[HttpRequest, A].

Translates Gateway API-style route configuration into xuma Matcher trees.
Pure Python types mirroring the Gateway API spec (no k8s dependency).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Literal

from xuma._matcher import Matcher, MatcherError, matcher_from_predicate
from xuma._predicate import Predicate, SinglePredicate, and_predicate, or_predicate
from xuma._string_matchers import ExactMatcher, PrefixMatcher, RegexMatcher
from xuma.http._inputs import HeaderInput, MethodInput, PathInput, QueryParamInput

if TYPE_CHECKING:
    from xuma.http._request import HttpRequest


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
        predicates: list[Predicate[HttpRequest]] = []

        if self.path is not None:
            predicates.append(_compile_path_match(self.path))

        if self.method is not None:
            predicates.append(SinglePredicate(MethodInput(), ExactMatcher(self.method)))

        for header_match in self.headers:
            predicates.append(_compile_header_match(header_match))

        for query_match in self.query_params:
            predicates.append(_compile_query_param_match(query_match))

        # An empty conjunction is vacuously true, so and_predicate would hand
        # back _catch_all(). Reaching this is rarely deliberate: every field is
        # optional and nothing rejects an unknown one, so a config saying
        # `pathPrefix:` where it meant `path:` arrives here with nothing set and
        # no signal that anything went wrong.
        #
        # This is also the only moment the mistake is visible. After
        # substitution the predicate is PrefixMatcher("") on the path, which is
        # indistinguishable from a deliberate catch-all — which is why
        # Matcher.validate() cannot catch it and never could.
        if not predicates:
            msg = (
                "HttpRouteMatch has no conditions, so it matches every request "
                "— check for a misspelled field. Use compile_catch_all() if a "
                "catch-all is intended."
            )
            raise MatcherError(msg)

        return and_predicate(predicates, _catch_all())


def compile_route_matches[A](
    matches: list[HttpRouteMatch],
    action: A,
    on_no_match: A | None = None,
) -> Matcher[HttpRequest, A]:
    """Compile multiple HttpRouteMatch entries into a single Matcher.

    Multiple matches are ORed together per Gateway API semantics.

    Raises:
        MatcherError: If the list is empty, or if any entry has no conditions.
            Both are catch-alls.
    """
    # Substituting a catch-all here was never spec behaviour. xDS is explicit:
    # "if no matcher above matched and this field is not populated, the match
    # will be considered unsuccessful" — an empty list is a *no-match*. The
    # config path already gets this right; only this convenience layer
    # disagreed with the engine underneath it.
    #
    # Not fixed by copying the loader, because there on_no_match is config the
    # operator wrote, while here it is an argument: ([], "allow", "deny") and
    # ([], "deny", "allow") are opposite outcomes from the same empty input.
    # An empty list is also almost never written on purpose — it is a config
    # that failed to load, or a filter that removed every rule.
    if not matches:
        msg = (
            "no route matches, which would match every request. Use "
            "compile_catch_all() if that is intended, or on_no_match for a "
            "default route."
        )
        raise MatcherError(msg)

    predicates = [m.to_predicate() for m in matches]
    return matcher_from_predicate(
        or_predicate(predicates, _catch_all()),
        action,
        on_no_match,
    )


def compile_catch_all[A](action: A) -> Matcher[HttpRequest, A]:
    """Build a matcher that matches every request.

    The explicit form of what compile_route_matches() now refuses to do by
    accident. A catch-all is a legitimate route; it just has to be asked for,
    and it is greppable when someone later asks why a gate admits everything.
    """
    return matcher_from_predicate(_catch_all(), action)


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
