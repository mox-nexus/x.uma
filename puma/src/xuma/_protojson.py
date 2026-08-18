"""Canonical protojson — the config format users write.

protojson is protobuf's own JSON mapping, so a config file here is a
``xds.type.matcher.v3.Matcher`` written the way protobuf says to write one:
``lowerCamelCase`` field names, a ``oneof`` as a single key, and an ``Any``
carried as its payload's fields beside an ``@type`` URL.

This module reads it into the same :class:`~xuma._config.MatcherConfig` the
registry already loads, so only the *reader* is new — the matcher construction
path is unchanged.

Why hand-written
----------------

puma carries no protobuf runtime, and that is a decision rather than an
omission. Measured 2026-08-18: neither ``betterproto.from_dict`` nor ts-proto's
``fromJSON`` rejects an unknown field — given ``{"kye": "role"}`` for
``MapInput`` both return a message with an *empty key*, which is exactly the
fail-open x.uma rejects everywhere else. Generated code that is lenient is worse
than a hand-written reader, because nobody audits a file headed ``DO NOT EDIT``.

The cost is that the dependency on ``proto/xuma/**`` is no longer an arrow a
build can see. The conformance suite carries it instead — see the fixture
coverage check, which fails when a message or field has no fixture.

Strictness
----------

Unknown fields are errors at every level. That is the whole point: a typo in a
deny rule must not produce a rule that silently never fires. The xDS tree is
checked here against the frozen upstream schema; payload fields are checked by
the factory that consumes them, because that is where the schema knowledge
already lives.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from xuma._config import (
    ActionConfig,
    AndPredicateConfig,
    BuiltInMatch,
    ConfigParseError,
    CustomMatch,
    FieldMatcherConfig,
    MatcherConfig,
    MatcherOnMatchConfig,
    NotPredicateConfig,
    OrPredicateConfig,
    SinglePredicateConfig,
    TypedConfig,
)

if TYPE_CHECKING:
    from collections.abc import Callable

    from xuma._config import OnMatchConfig, PredicateConfig, ValueMatchConfig

__all__ = ["named_action", "parse_protojson"]

_TYPE_KEY = "@type"
_TYPE_PREFIX = "type.googleapis.com/"

# How deep a document may nest before it is refused. Matches the limit Python's
# and Rust's JSON parsers impose, and exists because this walk runs over
# untrusted input before any matcher exists for MAX_DEPTH to protect.
_MAX_JSON_DEPTH = 128

# protojson accepts both the proto field name and its lowerCamelCase form.
# Listing both is what makes a *third* spelling an error rather than a shrug.
_MATCHER_FIELDS = frozenset(
    {"matcher_list", "matcherList", "matcher_tree", "matcherTree", "on_no_match", "onNoMatch"}
)
_ON_MATCH_FIELDS = frozenset({"matcher", "action", "keep_matching", "keepMatching"})
_FIELD_MATCHER_FIELDS = frozenset({"predicate", "on_match", "onMatch"})
_PREDICATE_FIELDS = frozenset(
    {
        "single_predicate", "singlePredicate", "or_matcher", "orMatcher",
        "and_matcher", "andMatcher", "not_matcher", "notMatcher",
    }
)
_SINGLE_PREDICATE_FIELDS = frozenset(
    {"input", "value_match", "valueMatch", "custom_match", "customMatch"}
)
_TYPED_EXTENSION_FIELDS = frozenset({"name", "typed_config", "typedConfig"})

# StringMatcher's match_pattern oneof, mapped to the variant names the config
# types use. `safe_regex` carries a RegexMatcher message rather than a string.
_STRING_MATCH_PATTERNS = {
    "exact": "Exact",
    "prefix": "Prefix",
    "suffix": "Suffix",
    "contains": "Contains",
}
_STRING_MATCHER_FIELDS = frozenset(
    set(_STRING_MATCH_PATTERNS)
    | {"safe_regex", "safeRegex", "ignore_case", "ignoreCase", "custom"}
)


def named_action(config: TypedConfig) -> str:
    """Turn ``xuma.core.v1.NamedAction`` into the string the engine returns.

    In xDS an action is a ``TypedExtensionConfig`` like any other extension; in
    this engine the action type is a plain string. This is the adapter between
    the two, and it is the default because it is the only action type x.uma
    ships.

    An empty ``name`` is refused. Every other empty identifier in the schema
    makes a predicate *false* — no decision. This one would make the rule
    **fire** and return ``""``, leaving a host that discriminates on
    ``action == "deny"`` to decide the polarity by accident.

    Raises:
        ConfigParseError: if the payload is not a usable NamedAction.
    """
    if config.type_url != "xuma.core.v1.NamedAction":
        msg = (
            f"action type {config.type_url!r} is not registered; "
            f"this engine ships only 'xuma.core.v1.NamedAction'"
        )
        raise ConfigParseError(msg)

    unknown = sorted(set(config.config) - {"name", "metadata"})
    if unknown:
        plural = "s" if len(unknown) > 1 else ""
        msg = f"NamedAction: unknown field{plural} {', '.join(map(repr, unknown))}"
        raise ConfigParseError(msg)

    name = config.config.get("name")
    if not isinstance(name, str) or not name:
        msg = (
            "NamedAction.name must be a non-empty string; an empty action name "
            "makes the rule fire and return nothing"
        )
        raise ConfigParseError(msg)
    return name


def parse_protojson(
    document: dict[str, Any],
    action: Callable[[TypedConfig], str] = named_action,
) -> MatcherConfig[str]:
    """Read a canonical protojson matcher.

    Args:
        document: the parsed YAML or JSON document.
        action: turns an action's ``Any`` payload into the value the engine
            returns. Mirrors rumi's ``ActionRegistry``; the default handles the
            one action type x.uma ships.

    Returns:
        The same config shape :func:`~xuma._config.parse_matcher_config` returns,
        so the registry loads it unchanged.

    Raises:
        ConfigParseError: if the document is not a valid
            ``xds.type.matcher.v3.Matcher``. Unknown fields are errors.
    """
    return _matcher(document, "matcher", 0, action)


def _obj(value: Any, where: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        msg = f"{where}: expected an object, got {type(value).__name__}"
        raise ConfigParseError(msg)
    return value


def _check_depth(depth: int, where: str) -> None:
    if depth > _MAX_JSON_DEPTH:
        msg = f"{where}: config nests deeper than {_MAX_JSON_DEPTH} levels"
        raise ConfigParseError(msg)


def _reject_unknown(data: dict[str, Any], allowed: frozenset[str], where: str) -> None:
    """The reason this module exists.

    A field the schema does not define is a load error, never a shrug. The
    hand-written config types this replaces had no such check, so a misspelled
    key in a deny rule produced a rule that never fired.
    """
    unknown = sorted(set(data) - allowed)
    if unknown:
        msg = (
            f"{where}: unknown field{'s' if len(unknown) > 1 else ''} "
            f"{', '.join(repr(u) for u in unknown)}; expected one of "
            f"{', '.join(repr(a) for a in sorted(allowed))}"
        )
        raise ConfigParseError(msg)


def _one_of(data: dict[str, Any], names: tuple[str, ...], where: str) -> tuple[str, Any] | None:
    """Read a protobuf ``oneof``: at most one member may be present."""
    found = [(n, data[n]) for n in names if n in data]
    if len(found) > 1:
        msg = f"{where}: {', '.join(repr(n) for n, _ in found)} are alternatives; set only one"
        raise ConfigParseError(msg)
    return found[0] if found else None


def _matcher(
    data: Any, where: str, depth: int, action: Callable[[TypedConfig], str]
) -> MatcherConfig[str]:
    _check_depth(depth, where)
    data = _obj(data, where)
    _reject_unknown(data, _MATCHER_FIELDS, where)

    chosen = _one_of(data, ("matcher_list", "matcherList", "matcher_tree", "matcherTree"), where)
    if chosen is None:
        msg = f"{where}: one of 'matcherList' or 'matcherTree' is required"
        raise ConfigParseError(msg)

    key, value = chosen
    if key in ("matcher_tree", "matcherTree"):
        # Rejected rather than ignored: a tree that silently became an empty
        # list would answer every request with on_no_match.
        msg = f"{where}: matcherTree is not implemented; use matcherList"
        raise ConfigParseError(msg)

    listing = _obj(value, f"{where}.matcherList")
    _reject_unknown(listing, frozenset({"matchers"}), f"{where}.matcherList")
    raw = listing.get("matchers", [])
    if not isinstance(raw, list):
        msg = f"{where}.matcherList.matchers: expected a list"
        raise ConfigParseError(msg)

    matchers = tuple(
        _field_matcher(fm, f"{where}.matchers[{i}]", depth + 1, action)
        for i, fm in enumerate(raw)
    )

    on_no_match = None
    for key in ("on_no_match", "onNoMatch"):
        if key in data:
            on_no_match = _on_match(data[key], f"{where}.onNoMatch", depth + 1, action)

    return MatcherConfig(matchers=matchers, on_no_match=on_no_match)


def _field_matcher(
    data: Any, where: str, depth: int, action: Callable[[TypedConfig], str]
) -> FieldMatcherConfig[str]:
    _check_depth(depth, where)
    data = _obj(data, where)
    _reject_unknown(data, _FIELD_MATCHER_FIELDS, where)

    if "predicate" not in data:
        msg = f"{where}: missing required field 'predicate'"
        raise ConfigParseError(msg)

    on_match = _one_of(data, ("on_match", "onMatch"), where)
    if on_match is None:
        msg = f"{where}: missing required field 'onMatch'"
        raise ConfigParseError(msg)

    return FieldMatcherConfig(
        predicate=_predicate(data["predicate"], f"{where}.predicate", depth + 1),
        on_match=_on_match(on_match[1], f"{where}.onMatch", depth + 1, action),
    )


def _predicate(data: Any, where: str, depth: int) -> PredicateConfig:
    _check_depth(depth, where)
    data = _obj(data, where)
    _reject_unknown(data, _PREDICATE_FIELDS, where)

    chosen = _one_of(
        data,
        ("single_predicate", "singlePredicate", "or_matcher", "orMatcher",
         "and_matcher", "andMatcher", "not_matcher", "notMatcher"),
        where,
    )
    if chosen is None:
        msg = (
            f"{where}: a predicate must set one of singlePredicate, andMatcher, "
            f"orMatcher, notMatcher"
        )
        raise ConfigParseError(msg)

    key, value = chosen
    if key in ("single_predicate", "singlePredicate"):
        return _single_predicate(value, f"{where}.singlePredicate", depth + 1)
    if key in ("not_matcher", "notMatcher"):
        return NotPredicateConfig(predicate=_predicate(value, f"{where}.notMatcher", depth + 1))

    # and_matcher / or_matcher carry a PredicateList: { "predicate": [...] }
    listing = _obj(value, f"{where}.{key}")
    _reject_unknown(listing, frozenset({"predicate"}), f"{where}.{key}")
    raw = listing.get("predicate", [])
    if not isinstance(raw, list):
        msg = f"{where}.{key}.predicate: expected a list"
        raise ConfigParseError(msg)
    children = tuple(
        _predicate(p, f"{where}.{key}.predicate[{i}]", depth + 1) for i, p in enumerate(raw)
    )
    if key in ("and_matcher", "andMatcher"):
        return AndPredicateConfig(predicates=children)
    return OrPredicateConfig(predicates=children)


def _single_predicate(data: Any, where: str, depth: int) -> SinglePredicateConfig:
    _check_depth(depth, where)
    data = _obj(data, where)
    _reject_unknown(data, _SINGLE_PREDICATE_FIELDS, where)

    if "input" not in data:
        msg = f"{where}: missing required field 'input'"
        raise ConfigParseError(msg)
    input_config = _typed_extension(data["input"], f"{where}.input")

    chosen = _one_of(data, ("value_match", "valueMatch", "custom_match", "customMatch"), where)
    if chosen is None:
        msg = f"{where}: one of 'valueMatch' or 'customMatch' is required"
        raise ConfigParseError(msg)

    key, value = chosen
    matcher: ValueMatchConfig = (
        CustomMatch(typed_config=_typed_extension(value, f"{where}.customMatch"))
        if key in ("custom_match", "customMatch")
        else _string_matcher(value, f"{where}.valueMatch")
    )

    return SinglePredicateConfig(input=input_config, matcher=matcher)


def _string_matcher(data: Any, where: str) -> BuiltInMatch:
    data = _obj(data, where)
    _reject_unknown(data, _STRING_MATCHER_FIELDS, where)

    if "custom" in data:
        msg = f"{where}: custom StringMatcher extensions are not implemented"
        raise ConfigParseError(msg)

    ignore_case = bool(data.get("ignore_case", data.get("ignoreCase", False)))

    chosen = _one_of(data, (*_STRING_MATCH_PATTERNS, "safe_regex", "safeRegex"), where)
    if chosen is None:
        msg = (
            f"{where}: a StringMatcher must set one of exact, prefix, suffix, "
            f"contains, safeRegex"
        )
        raise ConfigParseError(msg)

    key, value = chosen
    if key in ("safe_regex", "safeRegex"):
        regex = _obj(value, f"{where}.safeRegex")
        _reject_unknown(
            regex, frozenset({"regex", "google_re2", "googleRe2"}), f"{where}.safeRegex"
        )
        pattern = regex.get("regex")
        if not isinstance(pattern, str):
            msg = f"{where}.safeRegex: missing required field 'regex'"
            raise ConfigParseError(msg)
        return BuiltInMatch(variant="Regex", value=pattern, ignore_case=ignore_case)

    if not isinstance(value, str):
        msg = f"{where}.{key}: expected a string, got {type(value).__name__}"
        raise ConfigParseError(msg)
    return BuiltInMatch(
        variant=_STRING_MATCH_PATTERNS[key], value=value, ignore_case=ignore_case
    )


def _on_match(
    data: Any, where: str, depth: int, action: Callable[[TypedConfig], str]
) -> OnMatchConfig[str]:
    _check_depth(depth, where)
    data = _obj(data, where)
    _reject_unknown(data, _ON_MATCH_FIELDS, where)

    # keep_matching records the action and keeps evaluating in xDS; this engine
    # returns the first match. Accepting it would answer a different question
    # than the config asked, so it is refused rather than ignored.
    if data.get("keep_matching", data.get("keepMatching", False)):
        msg = (
            f"{where}: keepMatching is not implemented. In xDS it records the action and "
            f"continues evaluating; this engine returns the first match. Remove it, or "
            f"restructure the rule."
        )
        raise ConfigParseError(msg)

    chosen = _one_of(data, ("matcher", "action"), where)
    if chosen is None:
        msg = f"{where}: one of 'matcher' or 'action' is required"
        raise ConfigParseError(msg)

    key, value = chosen
    if key == "matcher":
        return MatcherOnMatchConfig(matcher=_matcher(value, f"{where}.matcher", depth + 1, action))
    return ActionConfig(action=action(_typed_extension(value, f"{where}.action")))


def _typed_extension(data: Any, where: str) -> TypedConfig:
    """Read a ``TypedExtensionConfig`` — a name and an ``Any`` payload."""
    data = _obj(data, where)
    _reject_unknown(data, _TYPED_EXTENSION_FIELDS, where)

    payload = None
    for key in ("typed_config", "typedConfig"):
        if key in data:
            payload = _obj(data[key], f"{where}.typedConfig")
    if payload is None:
        msg = f"{where}: missing required field 'typedConfig'"
        raise ConfigParseError(msg)

    url = payload.get(_TYPE_KEY)
    if not isinstance(url, str):
        msg = f"{where}.typedConfig: missing required field '{_TYPE_KEY}'"
        raise ConfigParseError(msg)
    if not url.startswith(_TYPE_PREFIX):
        msg = (
            f"{where}.typedConfig: '{_TYPE_KEY}' must be a full type URL beginning "
            f"'{_TYPE_PREFIX}', got {url!r}"
        )
        raise ConfigParseError(msg)

    # The payload body is handed to its factory unwalked. Its fields belong to
    # the payload's own schema, and the factory is where that knowledge lives.
    body = {k: v for k, v in payload.items() if k != _TYPE_KEY}
    return TypedConfig(type_url=url.removeprefix(_TYPE_PREFIX), config=body)
