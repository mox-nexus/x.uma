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
    # xDS StringMatcher.ignore_case. It belongs to the comparison rather than
    # to the pattern, which is why it sits here and not inside the value.
    ignore_case: bool = False


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


class ConfigParseError(Exception):
    """Error parsing a config dict into config types."""
