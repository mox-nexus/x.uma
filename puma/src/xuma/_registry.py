"""Type registry for config-driven matcher construction.

The registry enables generic config loading: JSON/YAML config → compiled
Matcher without domain-specific compile code.

Architecture mirrors rumi's registry.rs:
- RegistryBuilder[Ctx] → .build() → Registry[Ctx] (immutable)
- Factories are plain callables: (config: dict) → DataInput[Ctx] or InputMatcher
- load_matcher() walks the config tree and constructs runtime types

Example::

    builder = RegistryBuilder()
    builder.input("xuma.test.v1.StringInput", lambda cfg: DictInput(cfg["key"]))
    registry = builder.build()

    config = parse_matcher_config(json_data)
    matcher = registry.load_matcher(config)
"""

from __future__ import annotations

from dataclasses import dataclass, field
from types import MappingProxyType
from typing import TYPE_CHECKING, Any

from xuma._config import (
    ActionConfig,
    AndPredicateConfig,
    BuiltInMatch,
    CustomMatch,
    FieldMatcherConfig,
    MatcherConfig,
    MatcherOnMatchConfig,
    NotPredicateConfig,
    OrPredicateConfig,
    SinglePredicateConfig,
)
from xuma._matcher import (
    Action,
    FieldMatcher,
    Matcher,
    MatcherError,
    NestedMatcher,
)
from xuma._predicate import And, Not, Or, SinglePredicate
from xuma._string_matchers import (
    ContainsMatcher,
    ExactMatcher,
    PrefixMatcher,
    RegexMatcher,
    SuffixMatcher,
)

if TYPE_CHECKING:
    from collections.abc import Callable

    from xuma._config import OnMatchConfig, PredicateConfig, ValueMatchConfig
    from xuma._types import DataInput, InputMatcher

# ═══════════════════════════════════════════════════════════════════════════════
# Limits (matching rumi core constants)
# ═══════════════════════════════════════════════════════════════════════════════

MAX_FIELD_MATCHERS = 256
MAX_PREDICATES_PER_COMPOUND = 256
MAX_PATTERN_LENGTH = 8192
MAX_REGEX_PATTERN_LENGTH = 4096

# ═══════════════════════════════════════════════════════════════════════════════
# Error types
# ═══════════════════════════════════════════════════════════════════════════════


class UnknownTypeUrlError(MatcherError):
    """A type_url was not found in the registry."""

    def __init__(self, type_url: str, registry: str, available: list[str]) -> None:
        self.type_url = type_url
        self.registry = registry
        self.available = sorted(available)
        if self.available:
            registered = ", ".join(self.available)
            msg = (
                f"unknown {registry} type_url: {type_url!r} "
                f"(registered: {registered})"
            )
        else:
            msg = (
                f"unknown {registry} type_url: {type_url!r} "
                f"(no {registry} types are registered)"
            )
        super().__init__(msg)


class InvalidConfigError(MatcherError):
    """A config payload was malformed or semantically invalid."""

    def __init__(self, source: str) -> None:
        self.source = source
        super().__init__(f"invalid config: {source}")


class TooManyFieldMatchersError(MatcherError):
    """Config has too many field matchers (width-based limit)."""

    def __init__(self, count: int, max_: int) -> None:
        self.count = count
        self.max = max_
        super().__init__(
            f"too many field matchers: {count} exceeds maximum {max_}"
        )


class TooManyPredicatesError(MatcherError):
    """Compound predicate has too many children (width-based limit)."""

    def __init__(self, count: int, max_: int) -> None:
        self.count = count
        self.max = max_
        super().__init__(
            f"too many predicates in compound: {count} exceeds maximum {max_}"
        )


class PatternTooLongError(MatcherError):
    """A match pattern exceeds the length limit."""

    def __init__(self, length: int, max_: int) -> None:
        self.length = length
        self.max = max_
        super().__init__(
            f"pattern length {length} exceeds maximum {max_}"
        )


# ═══════════════════════════════════════════════════════════════════════════════
# Builder
# ═══════════════════════════════════════════════════════════════════════════════

# Factory type aliases
type InputFactory[Ctx] = Callable[[dict[str, Any]], DataInput[Ctx]]
type MatcherFactory = Callable[[dict[str, Any]], InputMatcher]


class RegistryBuilder[Ctx]:
    """Builder for constructing a Registry.

    Register DataInput and InputMatcher factories with type URLs, then call
    build() to produce an immutable Registry.

    Arch-guild constraint: immutability after build. No runtime registration.
    """

    def __init__(self) -> None:
        self._input_factories: dict[str, InputFactory[Ctx]] = {}
        self._matcher_factories: dict[str, MatcherFactory] = {}

    def input(
        self, type_url: str, factory: InputFactory[Ctx]
    ) -> RegistryBuilder[Ctx]:
        """Register a DataInput factory with a type URL."""
        self._input_factories[type_url] = factory
        return self

    def matcher(
        self, type_url: str, factory: MatcherFactory
    ) -> RegistryBuilder[Ctx]:
        """Register an InputMatcher factory with a type URL."""
        self._matcher_factories[type_url] = factory
        return self

    def build(self) -> Registry[Ctx]:
        """Freeze the registry. No further registration is possible."""
        return Registry(
            _input_factories=MappingProxyType(dict(self._input_factories)),
            _matcher_factories=MappingProxyType(dict(self._matcher_factories)),
        )


def register_core_matchers[Ctx](builder: RegistryBuilder[Ctx]) -> RegistryBuilder[Ctx]:
    """Register core built-in matchers (BoolMatcher, StringMatcher).

    Call this in domain register() functions to avoid duplicating
    core matcher registrations.
    """
    # StringMatcher handles all built-in string match types.
    # BoolMatcher is not used in config loading (built-in value_match covers strings),
    # but register it for custom_match usage.
    return builder


# ═══════════════════════════════════════════════════════════════════════════════
# Registry
# ═══════════════════════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class Registry[Ctx]:
    """Immutable registry of DataInput and InputMatcher factories.

    Constructed via RegistryBuilder. Use load_matcher() to compile
    config into a runtime Matcher.
    """

    _input_factories: MappingProxyType[str, InputFactory[Ctx]] = field(
        default_factory=lambda: MappingProxyType({})
    )
    _matcher_factories: MappingProxyType[str, MatcherFactory] = field(
        default_factory=lambda: MappingProxyType({})
    )

    def load_matcher(self, config: MatcherConfig[str]) -> Matcher[Ctx, str]:
        """Load a Matcher from configuration.

        Walks the config tree, constructs DataInputs and InputMatchers via
        registered factories, builds predicates and field matchers, and
        validates depth constraints.

        Raises:
            UnknownTypeUrlError: input or matcher type_url not registered
            InvalidConfigError: config payload malformed
            TooManyFieldMatchersError: too many field matchers
            TooManyPredicatesError: too many compound predicate children
            PatternTooLongError: pattern exceeds length limit
            MatcherError: depth exceeded
        """
        if len(config.matchers) > MAX_FIELD_MATCHERS:
            raise TooManyFieldMatchersError(len(config.matchers), MAX_FIELD_MATCHERS)

        matchers = tuple(
            self._load_field_matcher(fm) for fm in config.matchers
        )

        on_no_match = None
        if config.on_no_match is not None:
            on_no_match = self._load_on_match(config.on_no_match)

        return Matcher(matcher_list=matchers, on_no_match=on_no_match)

    @property
    def input_count(self) -> int:
        """Number of registered input types."""
        return len(self._input_factories)

    @property
    def matcher_count(self) -> int:
        """Number of registered matcher types."""
        return len(self._matcher_factories)

    def contains_input(self, type_url: str) -> bool:
        """Check if an input type URL is registered."""
        return type_url in self._input_factories

    def contains_matcher(self, type_url: str) -> bool:
        """Check if a matcher type URL is registered."""
        return type_url in self._matcher_factories

    def input_type_urls(self) -> list[str]:
        """Return all registered input type URLs (sorted)."""
        return sorted(self._input_factories.keys())

    def matcher_type_urls(self) -> list[str]:
        """Return all registered matcher type URLs (sorted)."""
        return sorted(self._matcher_factories.keys())

    # ── Private loading methods ────────────────────────────────────────────

    def _load_field_matcher(
        self, config: FieldMatcherConfig[str]
    ) -> FieldMatcher[Ctx, str]:
        predicate = self._load_predicate(config.predicate)
        on_match = self._load_on_match(config.on_match)
        return FieldMatcher(predicate=predicate, on_match=on_match)

    def _load_predicate(self, config: PredicateConfig) -> Any:
        match config:
            case SinglePredicateConfig():
                return self._load_single(config)
            case AndPredicateConfig(predicates=children):
                if len(children) > MAX_PREDICATES_PER_COMPOUND:
                    raise TooManyPredicatesError(
                        len(children), MAX_PREDICATES_PER_COMPOUND
                    )
                return And(
                    predicates=tuple(
                        self._load_predicate(p) for p in children
                    )
                )
            case OrPredicateConfig(predicates=children):
                if len(children) > MAX_PREDICATES_PER_COMPOUND:
                    raise TooManyPredicatesError(
                        len(children), MAX_PREDICATES_PER_COMPOUND
                    )
                return Or(
                    predicates=tuple(
                        self._load_predicate(p) for p in children
                    )
                )
            case NotPredicateConfig(predicate=inner):
                return Not(predicate=self._load_predicate(inner))
            case _:  # pragma: no cover
                msg = f"unknown predicate config type: {type(config).__name__}"
                raise InvalidConfigError(msg)

    def _load_single(self, config: SinglePredicateConfig) -> SinglePredicate[Ctx]:
        # Resolve input via factory
        factory = self._input_factories.get(config.input.type_url)
        if factory is None:
            raise UnknownTypeUrlError(
                config.input.type_url,
                "input",
                list(self._input_factories.keys()),
            )

        try:
            data_input = factory(config.input.config)
        except Exception as e:
            raise InvalidConfigError(str(e)) from e

        # Resolve matcher: built-in or custom
        matcher = self._load_value_match(config.matcher)
        return SinglePredicate(input=data_input, matcher=matcher)

    def _load_value_match(self, config: ValueMatchConfig) -> InputMatcher:
        match config:
            case BuiltInMatch(variant=variant, value=value):
                return _compile_built_in(variant, value)
            case CustomMatch(typed_config=tc):
                factory = self._matcher_factories.get(tc.type_url)
                if factory is None:
                    raise UnknownTypeUrlError(
                        tc.type_url,
                        "matcher",
                        list(self._matcher_factories.keys()),
                    )
                try:
                    return factory(tc.config)
                except Exception as e:
                    raise InvalidConfigError(str(e)) from e
            case _:  # pragma: no cover
                msg = f"unknown value_match config type: {type(config).__name__}"
                raise InvalidConfigError(msg)

    def _load_on_match(
        self, config: OnMatchConfig[str]
    ) -> Action[str] | NestedMatcher[Ctx, str]:
        match config:
            case ActionConfig(action=action):
                return Action(value=action)
            case MatcherOnMatchConfig(matcher=nested_config):
                nested = self.load_matcher(nested_config)
                return NestedMatcher(matcher=nested)
            case _:  # pragma: no cover
                msg = f"unknown on_match config type: {type(config).__name__}"
                raise InvalidConfigError(msg)


# ═══════════════════════════════════════════════════════════════════════════════
# Built-in matcher compilation
# ═══════════════════════════════════════════════════════════════════════════════


def _check_pattern_length(variant: str, value: str) -> None:
    """Enforce pattern length limits on built-in string match specs."""
    if variant == "Regex":
        if len(value) > MAX_REGEX_PATTERN_LENGTH:
            raise PatternTooLongError(len(value), MAX_REGEX_PATTERN_LENGTH)
    elif len(value) > MAX_PATTERN_LENGTH:
        raise PatternTooLongError(len(value), MAX_PATTERN_LENGTH)


def _compile_built_in(variant: str, value: str) -> InputMatcher:
    """Compile a built-in string match variant into an InputMatcher."""
    _check_pattern_length(variant, value)

    match variant:
        case "Exact":
            return ExactMatcher(value=value)
        case "Prefix":
            return PrefixMatcher(prefix=value)
        case "Suffix":
            return SuffixMatcher(suffix=value)
        case "Contains":
            return ContainsMatcher(substring=value)
        case "Regex":
            try:
                return RegexMatcher(pattern=value)
            except Exception as e:
                msg = f"invalid regex pattern: {e}"
                raise InvalidConfigError(msg) from e
        case _:
            msg = f"unknown built-in match variant: {variant!r}"
            raise InvalidConfigError(msg)
