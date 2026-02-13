"""puma — Pure Python implementation of xDS Unified Matcher API.

All public types are exported from this module for flat imports:

    from puma import Matcher, FieldMatcher, SinglePredicate, ExactMatcher
"""

__version__ = "0.1.0"

# Protocols
# Matcher tree
from puma._matcher import (
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
from puma._predicate import (
    And,
    Not,
    Or,
    Predicate,
    SinglePredicate,
    and_predicate,
    or_predicate,
    predicate_depth,
)

# Concrete matchers
from puma._string_matchers import (
    ContainsMatcher,
    ExactMatcher,
    PrefixMatcher,
    RegexMatcher,
    SuffixMatcher,
)
from puma._types import DataInput, InputMatcher, MatchingData

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
]
