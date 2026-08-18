"""Tests for puma registry (puma._registry).

Validates the builder -> frozen registry -> load_matcher pipeline.

These used to build configs through `parse_matcher_config`, the terse
dialect's reader. That dialect is retired (DECISIONS.md D-026); the IR types
it produced (`MatcherConfig` and friends) are still exactly what the registry
consumes, so the tests construct them directly instead. `single()` and
`field()` below are the only new things — small builders that keep test bodies
close to their previous shape, not a second config format.
"""

from __future__ import annotations

import pytest

from xuma import (
    ActionConfig,
    AndPredicateConfig,
    BuiltInMatch,
    CustomMatch,
    FieldMatcherConfig,
    InvalidConfigError,
    MatcherConfig,
    OrPredicateConfig,
    PatternTooLongError,
    Registry,
    RegistryBuilder,
    SinglePredicateConfig,
    TooManyFieldMatchersError,
    TooManyPredicatesError,
    TypedConfig,
    UnknownTypeUrlError,
)
from xuma._registry import (
    MAX_FIELD_MATCHERS,
    MAX_PATTERN_LENGTH,
    MAX_PREDICATES_PER_COMPOUND,
    MAX_REGEX_PATTERN_LENGTH,
)
from xuma.testing import DictInput, register


def mapinput(key: str) -> TypedConfig:
    """A `xuma.kv.v1.MapInput` reference, config-shaped as the registry expects."""
    return TypedConfig("xuma.kv.v1.MapInput", {"key": key})


def single(key: str, variant: str, value: str) -> SinglePredicateConfig:
    """A single predicate: read `key`, compare with `variant` (Exact/Prefix/...)."""
    return SinglePredicateConfig(mapinput(key), BuiltInMatch(variant, value))


def field(predicate, action: str) -> FieldMatcherConfig[str]:  # noqa: ANN001
    """A field matcher with a plain action on_match."""
    return FieldMatcherConfig(predicate, ActionConfig(action))


class TestRegistryBuilder:
    """Tests for RegistryBuilder."""

    def test_builder_registers_and_freezes(self) -> None:
        builder = RegistryBuilder()
        builder = builder.input("test.Input", lambda cfg: DictInput(cfg["key"]))
        registry = builder.build()
        assert isinstance(registry, Registry)

    def test_register_helper(self) -> None:
        builder = RegistryBuilder()
        builder = register(builder)
        registry = builder.build()
        assert registry.contains_input("xuma.kv.v1.MapInput")

    def test_introspection_type_urls(self) -> None:
        builder = RegistryBuilder()
        builder = register(builder)
        registry = builder.build()
        assert "xuma.kv.v1.MapInput" in registry.input_type_urls()

    def _make_registry(self) -> Registry[dict[str, str]]:
        builder = RegistryBuilder()
        builder = register(builder)
        return builder.build()

    def test_simple_exact_match(self) -> None:
        registry = self._make_registry()
        config = MatcherConfig(
            (field(single("name", "Exact", "alice"), "matched"),),
            ActionConfig("default"),
        )
        matcher = registry.load_matcher(config)

        assert matcher.evaluate({"name": "alice"}) == "matched"
        assert matcher.evaluate({"name": "bob"}) == "default"

    def test_and_predicate(self) -> None:
        registry = self._make_registry()
        config = MatcherConfig(
            (
                field(
                    AndPredicateConfig(
                        (
                            single("role", "Exact", "admin"),
                            single("org", "Prefix", "acme"),
                        )
                    ),
                    "admin_acme",
                ),
            )
        )
        matcher = registry.load_matcher(config)

        assert matcher.evaluate({"role": "admin", "org": "acme-corp"}) == "admin_acme"
        assert matcher.evaluate({"role": "admin", "org": "other"}) is None

    def test_nested_matcher(self) -> None:
        registry = self._make_registry()
        from xuma import MatcherOnMatchConfig

        inner = MatcherConfig((field(single("tier", "Exact", "premium"), "premium_route"),))
        outer_predicate = single("tier", "Prefix", "")
        outer_on_match = MatcherOnMatchConfig(inner)
        config = MatcherConfig(
            (FieldMatcherConfig(outer_predicate, outer_on_match),),
            ActionConfig("fallback"),
        )
        matcher = registry.load_matcher(config)

        assert matcher.evaluate({"tier": "premium"}) == "premium_route"
        assert matcher.evaluate({"tier": "basic"}) == "fallback"

    def test_all_string_match_types(self) -> None:
        """Verify all 5 string match types work end-to-end."""
        registry = self._make_registry()

        cases = [
            ("Exact", "hello", {"key": "hello"}, True),
            ("Prefix", "hel", {"key": "hello"}, True),
            ("Suffix", "llo", {"key": "hello"}, True),
            ("Contains", "ell", {"key": "hello"}, True),
            ("Regex", "^h.*o$", {"key": "hello"}, True),
            ("Exact", "hello", {"key": "world"}, False),
        ]

        for variant, pattern, ctx, should_match in cases:
            config = MatcherConfig((field(single("key", variant, pattern), "hit"),))
            matcher = registry.load_matcher(config)
            result = matcher.evaluate(ctx)
            expected = "hit" if should_match else None
            assert result == expected, (
                f"{variant}({pattern}) vs {ctx}: {result!r}, expected {expected!r}"
            )


class TestRegistryErrors:
    """Tests for registry error handling."""

    def test_unknown_input_type_url(self) -> None:
        registry = RegistryBuilder().build()
        config = MatcherConfig(
            (
                FieldMatcherConfig(
                    SinglePredicateConfig(
                        TypedConfig("unknown.Input"), BuiltInMatch("Exact", "x")
                    ),
                    ActionConfig("x"),
                ),
            )
        )
        with pytest.raises(UnknownTypeUrlError) as exc_info:
            registry.load_matcher(config)
        assert exc_info.value.type_url == "unknown.Input"
        assert exc_info.value.registry == "input"

    def test_unknown_input_lists_available(self) -> None:
        builder = RegistryBuilder()
        builder = register(builder)
        registry = builder.build()

        config = MatcherConfig(
            (
                FieldMatcherConfig(
                    SinglePredicateConfig(
                        TypedConfig("unknown.Input"), BuiltInMatch("Exact", "x")
                    ),
                    ActionConfig("x"),
                ),
            )
        )
        with pytest.raises(UnknownTypeUrlError) as exc_info:
            registry.load_matcher(config)
        assert "xuma.kv.v1.MapInput" in exc_info.value.available
        assert "xuma.kv.v1.MapInput" in str(exc_info.value)

    def test_unknown_matcher_type_url(self) -> None:
        builder = RegistryBuilder()
        builder = register(builder)
        registry = builder.build()

        config = MatcherConfig(
            (
                FieldMatcherConfig(
                    SinglePredicateConfig(
                        mapinput("x"), CustomMatch(TypedConfig("unknown.Matcher"))
                    ),
                    ActionConfig("x"),
                ),
            )
        )
        with pytest.raises(UnknownTypeUrlError) as exc_info:
            registry.load_matcher(config)
        assert exc_info.value.type_url == "unknown.Matcher"
        assert exc_info.value.registry == "matcher"

    def test_invalid_config(self) -> None:
        builder = RegistryBuilder()
        builder = register(builder)
        registry = builder.build()

        config = MatcherConfig(
            (
                field(
                    SinglePredicateConfig(
                        TypedConfig("xuma.kv.v1.MapInput", {"wrong_field": 42}),
                        BuiltInMatch("Exact", "x"),
                    ),
                    "x",
                ),
            )
        )
        with pytest.raises(InvalidConfigError):
            registry.load_matcher(config)


class TestWidthLimits:
    """Tests for width-based denial-of-service protection."""

    def _make_registry(self) -> Registry[dict[str, str]]:
        builder = RegistryBuilder()
        builder = register(builder)
        return builder.build()

    def test_too_many_field_matchers(self) -> None:
        registry = self._make_registry()
        fm = field(single("x", "Exact", "x"), "x")
        config = MatcherConfig(tuple([fm] * (MAX_FIELD_MATCHERS + 1)))
        with pytest.raises(TooManyFieldMatchersError) as exc_info:
            registry.load_matcher(config)
        assert exc_info.value.count == MAX_FIELD_MATCHERS + 1
        assert exc_info.value.max == MAX_FIELD_MATCHERS

    def test_too_many_predicates_and(self) -> None:
        registry = self._make_registry()
        one = single("x", "Exact", "x")
        config = MatcherConfig(
            (field(AndPredicateConfig(tuple([one] * (MAX_PREDICATES_PER_COMPOUND + 1))), "x"),)
        )
        with pytest.raises(TooManyPredicatesError):
            registry.load_matcher(config)

    def test_too_many_predicates_or(self) -> None:
        registry = self._make_registry()
        one = single("x", "Exact", "x")
        config = MatcherConfig(
            (field(OrPredicateConfig(tuple([one] * (MAX_PREDICATES_PER_COMPOUND + 1))), "x"),)
        )
        with pytest.raises(TooManyPredicatesError):
            registry.load_matcher(config)

    def test_pattern_too_long_exact(self) -> None:
        registry = self._make_registry()
        long_pattern = "x" * (MAX_PATTERN_LENGTH + 1)
        config = MatcherConfig((field(single("x", "Exact", long_pattern), "x"),))
        with pytest.raises(PatternTooLongError) as exc_info:
            registry.load_matcher(config)
        assert exc_info.value.length == MAX_PATTERN_LENGTH + 1
        assert exc_info.value.max == MAX_PATTERN_LENGTH

    def test_regex_pattern_too_long(self) -> None:
        registry = self._make_registry()
        long_regex = "a" * (MAX_REGEX_PATTERN_LENGTH + 1)
        config = MatcherConfig((field(single("x", "Regex", long_regex), "x"),))
        with pytest.raises(PatternTooLongError):
            registry.load_matcher(config)

    def test_pattern_at_limit_succeeds(self) -> None:
        registry = self._make_registry()
        pattern = "x" * MAX_PATTERN_LENGTH
        config = MatcherConfig((field(single("x", "Exact", pattern), "x"),))
        registry.load_matcher(config)  # should not raise

    def test_field_matchers_at_limit_succeeds(self) -> None:
        registry = self._make_registry()
        fm = field(single("x", "Exact", "x"), "x")
        config = MatcherConfig(tuple([fm] * MAX_FIELD_MATCHERS))
        registry.load_matcher(config)  # should not raise
