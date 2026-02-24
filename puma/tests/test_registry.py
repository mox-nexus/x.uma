"""Tests for puma registry (puma._registry).

Validates the builder → frozen registry → load_matcher pipeline.
"""

import pytest

from xuma import (
    InvalidConfigError,
    PatternTooLongError,
    Registry,
    RegistryBuilder,
    TooManyFieldMatchersError,
    TooManyPredicatesError,
    UnknownTypeUrlError,
    parse_matcher_config,
)
from xuma._registry import (
    MAX_FIELD_MATCHERS,
    MAX_PATTERN_LENGTH,
    MAX_PREDICATES_PER_COMPOUND,
    MAX_REGEX_PATTERN_LENGTH,
)
from xuma.testing import DictInput, register


class TestRegistryBuilder:
    """Tests for RegistryBuilder."""

    def test_builder_registers_and_freezes(self) -> None:
        builder = RegistryBuilder()
        builder.input("test.DictInput", lambda cfg: DictInput(cfg["key"]))
        registry = builder.build()

        assert registry.input_count == 1
        assert registry.contains_input("test.DictInput")
        assert not registry.contains_input("test.Unknown")

    def test_register_helper(self) -> None:
        builder = RegistryBuilder()
        builder = register(builder)
        registry = builder.build()

        assert registry.contains_input("xuma.test.v1.StringInput")

    def test_introspection_type_urls(self) -> None:
        builder = RegistryBuilder()
        builder.input("b.Input", lambda cfg: DictInput(cfg["key"]))
        builder.input("a.Input", lambda cfg: DictInput(cfg["key"]))
        registry = builder.build()

        # Sorted alphabetically
        assert registry.input_type_urls() == ["a.Input", "b.Input"]


class TestLoadMatcher:
    """Tests for Registry.load_matcher()."""

    def _make_registry(self) -> Registry[dict[str, str]]:
        builder = RegistryBuilder()
        builder = register(builder)
        return builder.build()

    def test_simple_exact_match(self) -> None:
        registry = self._make_registry()
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {
                            "type_url": "xuma.test.v1.StringInput",
                            "config": {"key": "name"},
                        },
                        "value_match": {"Exact": "alice"},
                    },
                    "on_match": {"type": "action", "action": "matched"},
                }
            ],
            "on_no_match": {"type": "action", "action": "default"},
        }
        config = parse_matcher_config(data)
        matcher = registry.load_matcher(config)

        assert matcher.evaluate({"name": "alice"}) == "matched"
        assert matcher.evaluate({"name": "bob"}) == "default"

    def test_and_predicate(self) -> None:
        registry = self._make_registry()
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "and",
                        "predicates": [
                            {
                                "type": "single",
                                "input": {
                                    "type_url": "xuma.test.v1.StringInput",
                                    "config": {"key": "role"},
                                },
                                "value_match": {"Exact": "admin"},
                            },
                            {
                                "type": "single",
                                "input": {
                                    "type_url": "xuma.test.v1.StringInput",
                                    "config": {"key": "org"},
                                },
                                "value_match": {"Prefix": "acme"},
                            },
                        ],
                    },
                    "on_match": {"type": "action", "action": "admin_acme"},
                }
            ]
        }
        config = parse_matcher_config(data)
        matcher = registry.load_matcher(config)

        assert matcher.evaluate({"role": "admin", "org": "acme-corp"}) == "admin_acme"
        assert matcher.evaluate({"role": "admin", "org": "other"}) is None

    def test_nested_matcher(self) -> None:
        registry = self._make_registry()
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {
                            "type_url": "xuma.test.v1.StringInput",
                            "config": {"key": "tier"},
                        },
                        "value_match": {"Prefix": ""},
                    },
                    "on_match": {
                        "type": "matcher",
                        "matcher": {
                            "matchers": [
                                {
                                    "predicate": {
                                        "type": "single",
                                        "input": {
                                            "type_url": "xuma.test.v1.StringInput",
                                            "config": {"key": "tier"},
                                        },
                                        "value_match": {"Exact": "premium"},
                                    },
                                    "on_match": {
                                        "type": "action",
                                        "action": "premium_route",
                                    },
                                }
                            ]
                        },
                    },
                }
            ],
            "on_no_match": {"type": "action", "action": "fallback"},
        }
        config = parse_matcher_config(data)
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
            data = {
                "matchers": [
                    {
                        "predicate": {
                            "type": "single",
                            "input": {
                                "type_url": "xuma.test.v1.StringInput",
                                "config": {"key": "key"},
                            },
                            "value_match": {variant: pattern},
                        },
                        "on_match": {"type": "action", "action": "hit"},
                    }
                ]
            }
            config = parse_matcher_config(data)
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
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {"type_url": "unknown.Input", "config": {}},
                        "value_match": {"Exact": "x"},
                    },
                    "on_match": {"type": "action", "action": "x"},
                }
            ]
        }
        config = parse_matcher_config(data)
        with pytest.raises(UnknownTypeUrlError) as exc_info:
            registry.load_matcher(config)
        assert exc_info.value.type_url == "unknown.Input"
        assert exc_info.value.registry == "input"

    def test_unknown_input_lists_available(self) -> None:
        builder = RegistryBuilder()
        builder = register(builder)
        registry = builder.build()

        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {"type_url": "unknown.Input", "config": {}},
                        "value_match": {"Exact": "x"},
                    },
                    "on_match": {"type": "action", "action": "x"},
                }
            ]
        }
        config = parse_matcher_config(data)
        with pytest.raises(UnknownTypeUrlError) as exc_info:
            registry.load_matcher(config)
        assert "xuma.test.v1.StringInput" in exc_info.value.available
        assert "xuma.test.v1.StringInput" in str(exc_info.value)

    def test_unknown_matcher_type_url(self) -> None:
        builder = RegistryBuilder()
        builder = register(builder)
        registry = builder.build()

        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {
                            "type_url": "xuma.test.v1.StringInput",
                            "config": {"key": "x"},
                        },
                        "custom_match": {"type_url": "unknown.Matcher", "config": {}},
                    },
                    "on_match": {"type": "action", "action": "x"},
                }
            ]
        }
        config = parse_matcher_config(data)
        with pytest.raises(UnknownTypeUrlError) as exc_info:
            registry.load_matcher(config)
        assert exc_info.value.type_url == "unknown.Matcher"
        assert exc_info.value.registry == "matcher"

    def test_invalid_config(self) -> None:
        builder = RegistryBuilder()
        builder = register(builder)
        registry = builder.build()

        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {
                            "type_url": "xuma.test.v1.StringInput",
                            "config": {"wrong_field": 42},
                        },
                        "value_match": {"Exact": "x"},
                    },
                    "on_match": {"type": "action", "action": "x"},
                }
            ]
        }
        config = parse_matcher_config(data)
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
        fm = {
            "predicate": {
                "type": "single",
                "input": {
                    "type_url": "xuma.test.v1.StringInput",
                    "config": {"key": "x"},
                },
                "value_match": {"Exact": "x"},
            },
            "on_match": {"type": "action", "action": "x"},
        }
        data = {"matchers": [fm] * (MAX_FIELD_MATCHERS + 1)}
        config = parse_matcher_config(data)
        with pytest.raises(TooManyFieldMatchersError) as exc_info:
            registry.load_matcher(config)
        assert exc_info.value.count == MAX_FIELD_MATCHERS + 1
        assert exc_info.value.max == MAX_FIELD_MATCHERS

    def test_too_many_predicates_and(self) -> None:
        registry = self._make_registry()
        single = {
            "type": "single",
            "input": {
                "type_url": "xuma.test.v1.StringInput",
                "config": {"key": "x"},
            },
            "value_match": {"Exact": "x"},
        }
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "and",
                        "predicates": [single] * (MAX_PREDICATES_PER_COMPOUND + 1),
                    },
                    "on_match": {"type": "action", "action": "x"},
                }
            ]
        }
        config = parse_matcher_config(data)
        with pytest.raises(TooManyPredicatesError):
            registry.load_matcher(config)

    def test_too_many_predicates_or(self) -> None:
        registry = self._make_registry()
        single = {
            "type": "single",
            "input": {
                "type_url": "xuma.test.v1.StringInput",
                "config": {"key": "x"},
            },
            "value_match": {"Exact": "x"},
        }
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "or",
                        "predicates": [single] * (MAX_PREDICATES_PER_COMPOUND + 1),
                    },
                    "on_match": {"type": "action", "action": "x"},
                }
            ]
        }
        config = parse_matcher_config(data)
        with pytest.raises(TooManyPredicatesError):
            registry.load_matcher(config)

    def test_pattern_too_long_exact(self) -> None:
        registry = self._make_registry()
        long_pattern = "x" * (MAX_PATTERN_LENGTH + 1)
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {
                            "type_url": "xuma.test.v1.StringInput",
                            "config": {"key": "x"},
                        },
                        "value_match": {"Exact": long_pattern},
                    },
                    "on_match": {"type": "action", "action": "x"},
                }
            ]
        }
        config = parse_matcher_config(data)
        with pytest.raises(PatternTooLongError) as exc_info:
            registry.load_matcher(config)
        assert exc_info.value.length == MAX_PATTERN_LENGTH + 1
        assert exc_info.value.max == MAX_PATTERN_LENGTH

    def test_regex_pattern_too_long(self) -> None:
        registry = self._make_registry()
        long_regex = "a" * (MAX_REGEX_PATTERN_LENGTH + 1)
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {
                            "type_url": "xuma.test.v1.StringInput",
                            "config": {"key": "x"},
                        },
                        "value_match": {"Regex": long_regex},
                    },
                    "on_match": {"type": "action", "action": "x"},
                }
            ]
        }
        config = parse_matcher_config(data)
        with pytest.raises(PatternTooLongError):
            registry.load_matcher(config)

    def test_pattern_at_limit_succeeds(self) -> None:
        registry = self._make_registry()
        pattern = "x" * MAX_PATTERN_LENGTH
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {
                            "type_url": "xuma.test.v1.StringInput",
                            "config": {"key": "x"},
                        },
                        "value_match": {"Exact": pattern},
                    },
                    "on_match": {"type": "action", "action": "x"},
                }
            ]
        }
        config = parse_matcher_config(data)
        # Should not raise
        registry.load_matcher(config)

    def test_field_matchers_at_limit_succeeds(self) -> None:
        registry = self._make_registry()
        fm = {
            "predicate": {
                "type": "single",
                "input": {
                    "type_url": "xuma.test.v1.StringInput",
                    "config": {"key": "x"},
                },
                "value_match": {"Exact": "x"},
            },
            "on_match": {"type": "action", "action": "x"},
        }
        data = {"matchers": [fm] * MAX_FIELD_MATCHERS}
        config = parse_matcher_config(data)
        # Should not raise
        registry.load_matcher(config)
