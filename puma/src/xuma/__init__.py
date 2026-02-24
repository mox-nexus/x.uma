"""xuma — Pure Python implementation of xDS Unified Matcher API.

All public types are exported from this module for flat imports:

    from xuma import Matcher, FieldMatcher, SinglePredicate, ExactMatcher
"""

__version__ = "0.0.2"

# Protocols
# Matcher tree
# Config types — see xuma._config for details
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
    OnMatchConfig,
    OrPredicateConfig,
    PredicateConfig,
    SinglePredicateConfig,
    TypedConfig,
    ValueMatchConfig,
    parse_matcher_config,
)
from xuma._matcher import (
    MAX_DEPTH,
    Action,
    FieldMatcher,
    Matcher,
    MatcherError,
    NestedMatcher,
    OnMatch,
    matcher_from_predicate,
)

# Predicates
from xuma._predicate import (
    And,
    Not,
    Or,
    Predicate,
    SinglePredicate,
    and_predicate,
    or_predicate,
    predicate_depth,
)

# Registry — see xuma._registry for details
from xuma._registry import (
    MAX_FIELD_MATCHERS,
    MAX_PATTERN_LENGTH,
    MAX_PREDICATES_PER_COMPOUND,
    MAX_REGEX_PATTERN_LENGTH,
    InvalidConfigError,
    PatternTooLongError,
    Registry,
    RegistryBuilder,
    TooManyFieldMatchersError,
    TooManyPredicatesError,
    UnknownTypeUrlError,
    register_core_matchers,
)

# Concrete matchers
from xuma._string_matchers import (
    ContainsMatcher,
    ExactMatcher,
    PrefixMatcher,
    RegexMatcher,
    SuffixMatcher,
)
from xuma._types import DataInput, InputMatcher, MatchingData

__all__ = [
    # Protocols
    "DataInput",
    "InputMatcher",
    "MatchingData",
    # Predicates
    "SinglePredicate",
    "And",
    "Or",
    "Not",
    "Predicate",
    "and_predicate",
    "or_predicate",
    "predicate_depth",
    # Matcher
    "Action",
    "NestedMatcher",
    "OnMatch",
    "FieldMatcher",
    "Matcher",
    "MatcherError",
    "matcher_from_predicate",
    "MAX_DEPTH",
    # Concrete matchers
    "ExactMatcher",
    "PrefixMatcher",
    "SuffixMatcher",
    "ContainsMatcher",
    "RegexMatcher",
    # Config types
    "TypedConfig",
    "BuiltInMatch",
    "CustomMatch",
    "ValueMatchConfig",
    "SinglePredicateConfig",
    "AndPredicateConfig",
    "OrPredicateConfig",
    "NotPredicateConfig",
    "PredicateConfig",
    "ActionConfig",
    "MatcherOnMatchConfig",
    "OnMatchConfig",
    "FieldMatcherConfig",
    "MatcherConfig",
    "ConfigParseError",
    "parse_matcher_config",
    # Registry
    "RegistryBuilder",
    "Registry",
    "register_core_matchers",
    "UnknownTypeUrlError",
    "InvalidConfigError",
    "TooManyFieldMatchersError",
    "TooManyPredicatesError",
    "PatternTooLongError",
    "MAX_FIELD_MATCHERS",
    "MAX_PREDICATES_PER_COMPOUND",
    "MAX_PATTERN_LENGTH",
    "MAX_REGEX_PATTERN_LENGTH",
]
