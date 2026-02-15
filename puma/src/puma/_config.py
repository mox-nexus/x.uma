"""Config types for generic matcher construction.

These types mirror rumi's config.rs — the same JSON/YAML shape works across
all implementations. Config-driven matcher construction path:
  dict → parse_matcher_config() → MatcherConfig → Registry.load_matcher() → Matcher

Relationship to runtime types:

| Config type            | Runtime type      |
|------------------------|-------------------|
| MatcherConfig          | Matcher           |
| FieldMatcherConfig     | FieldMatcher      |
| PredicateConfig        | Predicate         |
| SinglePredicateConfig  | SinglePredicate   |
| ValueMatchConfig       | InputMatcher      |
| OnMatchConfig          | OnMatch           |
| TypedConfig            | DataInput/matcher |
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

# ═══════════════════════════════════════════════════════════════════════════════
# Config types (frozen dataclasses, mirroring rumi/core/src/config.rs)
# ═══════════════════════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class TypedConfig:
    """Reference to a registered type with its configuration.

    Maps to xDS TypedExtensionConfig:
    - type_url identifies the registered type (input, matcher, or action)
    - config carries the type-specific configuration payload
    """

    type_url: str
    config: dict[str, Any] = field(default_factory=dict)


@dataclass(frozen=True, slots=True)
class BuiltInMatch:
    """Built-in string matching (Exact, Prefix, Suffix, Contains, Regex).

    The variant name follows rumi's serde format:
    { "Exact": "hello" }, { "Prefix": "/api" }, { "Regex": "^foo" }
    """

    variant: str
    value: str


@dataclass(frozen=True, slots=True)
class CustomMatch:
    """Custom matcher resolved via the registry's matcher factories."""

    typed_config: TypedConfig


# Mirrors Envoy's oneof matcher in SinglePredicate.
type ValueMatchConfig = BuiltInMatch | CustomMatch


@dataclass(frozen=True, slots=True)
class SinglePredicateConfig:
    """Config for a SinglePredicate: input + value match.

    Exactly one of value_match or custom_match must be set (oneof).
    """

    input: TypedConfig
    matcher: ValueMatchConfig


@dataclass(frozen=True, slots=True)
class AndPredicateConfig:
    """All child predicates must match (logical AND)."""

    predicates: tuple[PredicateConfig, ...]


@dataclass(frozen=True, slots=True)
class OrPredicateConfig:
    """Any child predicate must match (logical OR)."""

    predicates: tuple[PredicateConfig, ...]


@dataclass(frozen=True, slots=True)
class NotPredicateConfig:
    """Inverts the inner predicate (logical NOT)."""

    predicate: PredicateConfig


type PredicateConfig = (
    SinglePredicateConfig | AndPredicateConfig | OrPredicateConfig | NotPredicateConfig
)


@dataclass(frozen=True, slots=True)
class ActionConfig[A]:
    """Return this action when the predicate matches."""

    action: A


@dataclass(frozen=True, slots=True)
class MatcherOnMatchConfig[A]:
    """Continue evaluation into a nested matcher."""

    matcher: MatcherConfig[A]


type OnMatchConfig[A] = ActionConfig[A] | MatcherOnMatchConfig[A]


@dataclass(frozen=True, slots=True)
class FieldMatcherConfig[A]:
    """Pairs a predicate config with an on_match config."""

    predicate: PredicateConfig
    on_match: OnMatchConfig[A]


@dataclass(frozen=True, slots=True)
class MatcherConfig[A]:
    """Configuration for a Matcher.

    Deserializes from JSON/YAML dicts and can be loaded into a runtime
    Matcher via Registry.load_matcher().
    """

    matchers: tuple[FieldMatcherConfig[A], ...]
    on_no_match: OnMatchConfig[A] | None = None


# ═══════════════════════════════════════════════════════════════════════════════
# Parsing (dict → config types)
# Same JSON shape as rumi's serde deserialization.
# ═══════════════════════════════════════════════════════════════════════════════

# String match variant names (matching rumi's serde format)
_STRING_MATCH_VARIANTS = frozenset({"Exact", "Prefix", "Suffix", "Contains", "Regex"})


class ConfigParseError(Exception):
    """Error parsing a config dict into config types."""


def parse_matcher_config(data: dict[str, Any]) -> MatcherConfig[str]:
    """Parse a dict into a MatcherConfig[str].

    This is the main entry point for config loading. Accepts the same
    JSON shape as rumi's serde deserialization.

    Raises:
        ConfigParseError: If the dict is malformed.
    """
    if not isinstance(data, dict):
        msg = f"expected dict, got {type(data).__name__}"
        raise ConfigParseError(msg)

    raw_matchers = data.get("matchers")
    if raw_matchers is None:
        msg = "missing required field 'matchers'"
        raise ConfigParseError(msg)
    if not isinstance(raw_matchers, list):
        msg = f"'matchers' must be a list, got {type(raw_matchers).__name__}"
        raise ConfigParseError(msg)

    matchers = tuple(_parse_field_matcher(fm) for fm in raw_matchers)

    on_no_match = None
    if "on_no_match" in data:
        on_no_match = _parse_on_match(data["on_no_match"])

    return MatcherConfig(matchers=matchers, on_no_match=on_no_match)


def _parse_field_matcher(data: dict[str, Any]) -> FieldMatcherConfig[str]:
    """Parse a field matcher config dict."""
    if not isinstance(data, dict):
        msg = f"field_matcher must be a dict, got {type(data).__name__}"
        raise ConfigParseError(msg)

    if "predicate" not in data:
        msg = "field_matcher missing required field 'predicate'"
        raise ConfigParseError(msg)
    if "on_match" not in data:
        msg = "field_matcher missing required field 'on_match'"
        raise ConfigParseError(msg)

    predicate = _parse_predicate(data["predicate"])
    on_match = _parse_on_match(data["on_match"])
    return FieldMatcherConfig(predicate=predicate, on_match=on_match)


def _parse_predicate(data: dict[str, Any]) -> PredicateConfig:
    """Parse a predicate config dict.

    Uses 'type' discriminant: single, and, or, not.
    """
    if not isinstance(data, dict):
        msg = f"predicate must be a dict, got {type(data).__name__}"
        raise ConfigParseError(msg)

    pred_type = data.get("type")
    if pred_type is None:
        msg = "predicate missing required field 'type'"
        raise ConfigParseError(msg)

    if pred_type == "single":
        return _parse_single_predicate(data)
    if pred_type == "and":
        children = data.get("predicates", [])
        return AndPredicateConfig(
            predicates=tuple(_parse_predicate(p) for p in children)
        )
    if pred_type == "or":
        children = data.get("predicates", [])
        return OrPredicateConfig(
            predicates=tuple(_parse_predicate(p) for p in children)
        )
    if pred_type == "not":
        if "predicate" not in data:
            msg = "not predicate missing required field 'predicate'"
            raise ConfigParseError(msg)
        return NotPredicateConfig(predicate=_parse_predicate(data["predicate"]))

    msg = f"unknown predicate type: {pred_type!r}"
    raise ConfigParseError(msg)


def _parse_single_predicate(data: dict[str, Any]) -> SinglePredicateConfig:
    """Parse a single predicate config dict.

    Enforces oneof: exactly one of value_match or custom_match.
    """
    if "input" not in data:
        msg = "single predicate missing required field 'input'"
        raise ConfigParseError(msg)

    input_cfg = _parse_typed_config(data["input"])
    has_value_match = "value_match" in data
    has_custom_match = "custom_match" in data

    if has_value_match and has_custom_match:
        msg = "exactly one of 'value_match' or 'custom_match' must be set, got both"
        raise ConfigParseError(msg)
    if not has_value_match and not has_custom_match:
        msg = "one of 'value_match' or 'custom_match' is required"
        raise ConfigParseError(msg)

    matcher: ValueMatchConfig
    if has_value_match:
        matcher = _parse_value_match(data["value_match"])
    else:
        matcher = CustomMatch(typed_config=_parse_typed_config(data["custom_match"]))

    return SinglePredicateConfig(input=input_cfg, matcher=matcher)


def _parse_value_match(data: dict[str, Any]) -> BuiltInMatch:
    """Parse a value_match dict into a BuiltInMatch.

    Expected format: { "Exact": "hello" } or { "Prefix": "/api" } etc.
    """
    if not isinstance(data, dict):
        msg = f"value_match must be a dict, got {type(data).__name__}"
        raise ConfigParseError(msg)

    for variant in _STRING_MATCH_VARIANTS:
        if variant in data:
            value = data[variant]
            if not isinstance(value, str):
                msg = f"value_match {variant} value must be a string, got {type(value).__name__}"
                raise ConfigParseError(msg)
            return BuiltInMatch(variant=variant, value=value)

    expected = sorted(_STRING_MATCH_VARIANTS)
    msg = f"value_match must contain one of {expected}, got keys: {sorted(data.keys())}"
    raise ConfigParseError(msg)


def _parse_on_match(data: dict[str, Any]) -> OnMatchConfig[str]:
    """Parse an on_match config dict.

    Uses 'type' discriminant: action or matcher.
    """
    if not isinstance(data, dict):
        msg = f"on_match must be a dict, got {type(data).__name__}"
        raise ConfigParseError(msg)

    om_type = data.get("type")
    if om_type is None:
        msg = "on_match missing required field 'type'"
        raise ConfigParseError(msg)

    if om_type == "action":
        if "action" not in data:
            msg = "action on_match missing required field 'action'"
            raise ConfigParseError(msg)
        return ActionConfig(action=data["action"])

    if om_type == "matcher":
        if "matcher" not in data:
            msg = "matcher on_match missing required field 'matcher'"
            raise ConfigParseError(msg)
        return MatcherOnMatchConfig(matcher=parse_matcher_config(data["matcher"]))

    msg = f"unknown on_match type: {om_type!r}"
    raise ConfigParseError(msg)


def _parse_typed_config(data: dict[str, Any]) -> TypedConfig:
    """Parse a typed config dict."""
    if not isinstance(data, dict):
        msg = f"typed_config must be a dict, got {type(data).__name__}"
        raise ConfigParseError(msg)

    if "type_url" not in data:
        msg = "typed_config missing required field 'type_url'"
        raise ConfigParseError(msg)

    type_url = data["type_url"]
    if not isinstance(type_url, str):
        msg = f"type_url must be a string, got {type(type_url).__name__}"
        raise ConfigParseError(msg)

    config = data.get("config", {})
    if not isinstance(config, dict):
        msg = f"config must be a dict, got {type(config).__name__}"
        raise ConfigParseError(msg)

    return TypedConfig(type_url=type_url, config=config)
