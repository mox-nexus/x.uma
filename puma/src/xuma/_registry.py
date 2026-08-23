"""Type registry for config-driven matcher construction.

The registry enables generic config loading: JSON/YAML config → compiled
Matcher without domain-specific compile code.

Architecture mirrors rumi's registry.rs:
- RegistryBuilder[Ctx] → .build() → Registry[Ctx] (immutable)
- Factories are plain callables: (config: dict) → DataInput[Ctx] or InputMatcher
- load_matcher() walks the config tree and constructs runtime types

Example::

    builder = RegistryBuilder()
    builder.input("xuma.kv.v1.MapInput", lambda cfg: DictInput(cfg["key"]))
    registry = builder.build()

    config = parse_protojson(json_data)  # from xuma._protojson
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
    MatcherTreeConfig,
    NotPredicateConfig,
    OrPredicateConfig,
    SinglePredicateConfig,
)
from xuma._matcher import (
    Action,
    FieldMatcher,
    Matcher,
    MatcherError,
    MatcherTree,
    NestedMatcher,
    OnMatch,
)

# These moved to _matcher so Matcher.validate() can raise them without importing
# this module, which imports it. Re-exported explicitly (`X as X`) because every
# existing import expects them here and mypy treats a bare re-import as private.
from xuma._matcher import TooManyFieldMatchersError as TooManyFieldMatchersError
from xuma._matcher import TooManyPredicatesError as TooManyPredicatesError
from xuma._predicate import And, Not, Or, SinglePredicate
from xuma._string_matchers import (
    BoolMatcher,
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

# Defined in _limits so the matcher constructors can enforce them without a
# circular import. Re-exported here so existing call sites are unaffected.
# The `X as X` form is an explicit re-export, which mypy strict requires.
from xuma._limits import MAX_FIELD_MATCHERS as MAX_FIELD_MATCHERS
from xuma._limits import MAX_PATTERN_LENGTH as MAX_PATTERN_LENGTH
from xuma._limits import MAX_PREDICATES_PER_COMPOUND as MAX_PREDICATES_PER_COMPOUND
from xuma._limits import MAX_REGEX_PATTERN_LENGTH as MAX_REGEX_PATTERN_LENGTH
from xuma._limits import MAX_TREE_ENTRIES as MAX_TREE_ENTRIES

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


class DuplicateTreeKeyError(MatcherError):
    """Two entries of a matcher tree share a key.

    A map keeps the last writer, so one of the two rules would vanish without
    a word — a rule the author believes is in force and is not.
    """

    def __init__(self, key: str) -> None:
        self.key = key
        super().__init__(
            f"matcher tree has more than one entry for key {key!r}; "
            f"one of them would never be reachable"
        )


class TooManyTreeEntriesError(MatcherError):
    """A matcher tree has too many entries (width-based limit)."""

    def __init__(self, count: int, max_: int) -> None:
        self.count = count
        self.max = max_
        super().__init__(
            f"matcher tree has {count} entries, maximum is {max_}"
        )


class IncompatibleTypesError(MatcherError):
    """An input's data type is not one the matcher can compare.

    Caught at load time rather than at evaluation, where the mismatch would
    look like a rule that simply never fires.
    """

    def __init__(self, input_type: str, matcher_types: tuple[str, ...]) -> None:
        self.input_type = input_type
        self.matcher_types = matcher_types
        super().__init__(
            f'input produces "{input_type}" data but matcher supports '
            f"{list(matcher_types)}"
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
    """Register core built-in matchers.

    Call this in domain register() functions to avoid duplicating core matcher
    registrations.

    This registered *nothing* until 2026-08-18 while claiming otherwise, so
    `xuma.core.v1.BoolMatcher` resolved in rumi and not here — a cross-language
    divergence in what a config may name.

    `xuma.core.v1.StringMatcher` is deliberately absent: it was a second way to
    say what `valueMatch` already says, and `customMatch` exists for
    comparisons that oneof cannot express, not for duplicating it.
    """
    return builder.matcher(
        "xuma.core.v1.BoolMatcher",
        lambda cfg: BoolMatcher(expected=bool(cfg["expected"])),
    )


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
        on_no_match = None
        if config.on_no_match is not None:
            on_no_match = self._load_on_match(config.on_no_match)

        if config.tree is not None:
            return Matcher(
                on_no_match=on_no_match,
                tree=self._load_tree(config.tree),
            )

        if len(config.matchers) > MAX_FIELD_MATCHERS:
            raise TooManyFieldMatchersError(len(config.matchers), MAX_FIELD_MATCHERS)

        matchers = tuple(
            self._load_field_matcher(fm) for fm in config.matchers
        )

        return Matcher(matcher_list=matchers, on_no_match=on_no_match)

    def _load_tree(self, config: MatcherTreeConfig[str]) -> MatcherTree[Ctx, str]:
        """Build a MatcherTree, resolving its input and entries."""
        # Checked before building, so a config that would blow memory is
        # rejected rather than materialised and then measured.
        if len(config.entries) > MAX_TREE_ENTRIES:
            raise TooManyTreeEntriesError(len(config.entries), MAX_TREE_ENTRIES)

        factory = self._input_factories.get(config.input.type_url)
        if factory is None:
            raise UnknownTypeUrlError(
                config.input.type_url,
                "input",
                list(self._input_factories.keys()),
            )
        try:
            tree_input = factory(config.input.config)
        except Exception as e:
            raise InvalidConfigError(str(e)) from e

        # A tree looks its key up as a string, so an input that declares any
        # other type can never match. Rejected here rather than silently never
        # firing. Bounded by the fact that data_type defaults to "string".
        declared = getattr(tree_input, "data_type", None)
        if callable(declared) and declared() != "string":
            raise IncompatibleTypesError(declared(), ("string",))

        seen: set[str] = set()
        entries: list[tuple[str, OnMatch[Ctx, str]]] = []
        for key, om in config.entries:
            if key in seen:
                raise DuplicateTreeKeyError(key)
            seen.add(key)
            entries.append((key, self._load_on_match(om)))

        return MatcherTree(input=tree_input, rule=config.rule, entries=tuple(entries))

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

        # Type compatibility, checked at load rather than discovered at
        # evaluation. rumi has done this since the beginning; puma and bumi did
        # not, so a config pairing a string input with a boolean matcher was a
        # load error in one implementation and a rule that silently never fired
        # in the other two -- DECISIONS.md D-040.
        data_type = getattr(data_input, "data_type", lambda: "string")()
        supported = getattr(matcher, "supported_types", lambda: ("string",))()
        if data_type not in supported:
            raise IncompatibleTypesError(data_type, tuple(supported))
        return SinglePredicate(input=data_input, matcher=matcher)

    def _load_value_match(self, config: ValueMatchConfig) -> InputMatcher:
        match config:
            case BuiltInMatch(variant=variant, value=value, ignore_case=ignore_case):
                return _compile_built_in(variant, value, ignore_case)
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


def _disables_case_insensitivity(pattern: str) -> bool:
    r"""Does this pattern clear the ``i`` flag with an inline group?

    ignore_case asks the engine for a case-insensitive match, and an inline
    ``(?-i)`` overrides that — measured in Rust both ways on 2026-08-18, and it
    is correct regex semantics rather than an engine quirk, so no choice of
    construction fixes it. The combination is refused instead.

    Scans flag groups: ``(?`` followed by flag letters, ended by ``)`` or ``:``.
    A group clears ``i`` only if an ``i`` follows a ``-`` inside it, so
    ``(?i-s)`` is fine and ``(?-si)`` is not. An escaped ``\(`` is not a group.
    """
    i = 0
    while i + 1 < len(pattern):
        if pattern[i] == "\\":
            i += 2
            continue
        if pattern[i] != "(" or pattern[i + 1] != "?":
            i += 1
            continue
        j = i + 2
        clearing = False
        while j < len(pattern):
            c = pattern[j]
            if c == "-":
                clearing = True
            elif c == "i" and clearing:
                return True
            elif c not in "imsuxU":
                break
            j += 1
        i = max(j, i + 2)
    return False


def _compile_built_in(
    variant: str, value: str, ignore_case: bool = False
) -> InputMatcher:
    """Compile a built-in string match variant into an InputMatcher."""
    _check_pattern_length(variant, value)

    match variant:
        case "Exact":
            return ExactMatcher(value=value, ignore_case=ignore_case)
        case "Prefix":
            return PrefixMatcher(prefix=value, ignore_case=ignore_case)
        case "Suffix":
            return SuffixMatcher(suffix=value, ignore_case=ignore_case)
        case "Contains":
            return ContainsMatcher(substring=value, ignore_case=ignore_case)
        case "Regex":
            if ignore_case and _disables_case_insensitivity(value):
                msg = (
                    f"ignore_case is set, but the pattern {value!r} turns "
                    f"case-insensitivity off inline with a (?-i) flag. An inline flag "
                    f"wins, so this rule would read case-insensitive and not be. "
                    f"Remove one of the two."
                )
                raise InvalidConfigError(msg)
            try:
                pattern = f"(?i){value}" if ignore_case else value
                return RegexMatcher(pattern=pattern)
            except Exception as e:
                msg = f"invalid regex pattern: {e}"
                raise InvalidConfigError(msg) from e
        case _:
            msg = f"unknown built-in match variant: {variant!r}"
            raise InvalidConfigError(msg)
