"""Tests for puma config parsing (puma._config).

Validates the dict → config type conversion that mirrors rumi's serde.
"""

import pytest

from puma import (
    ActionConfig,
    AndPredicateConfig,
    BuiltInMatch,
    ConfigParseError,
    CustomMatch,
    MatcherOnMatchConfig,
    NotPredicateConfig,
    OrPredicateConfig,
    SinglePredicateConfig,
    parse_matcher_config,
)


class TestParseMatcherConfig:
    """Tests for parse_matcher_config()."""

    def test_simple_exact(self) -> None:
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {"type_url": "test.Input", "config": {"key": "val"}},
                        "value_match": {"Exact": "hello"},
                    },
                    "on_match": {"type": "action", "action": "hit"},
                }
            ],
            "on_no_match": {"type": "action", "action": "miss"},
        }
        config = parse_matcher_config(data)
        assert len(config.matchers) == 1
        assert isinstance(config.on_no_match, ActionConfig)
        assert config.on_no_match.action == "miss"

    def test_and_predicate(self) -> None:
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "and",
                        "predicates": [
                            {
                                "type": "single",
                                "input": {"type_url": "a"},
                                "value_match": {"Exact": "x"},
                            },
                            {
                                "type": "single",
                                "input": {"type_url": "b"},
                                "value_match": {"Prefix": "y"},
                            },
                        ],
                    },
                    "on_match": {"type": "action", "action": "ok"},
                }
            ]
        }
        config = parse_matcher_config(data)
        pred = config.matchers[0].predicate
        assert isinstance(pred, AndPredicateConfig)
        assert len(pred.predicates) == 2

    def test_or_predicate(self) -> None:
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "or",
                        "predicates": [
                            {
                                "type": "single",
                                "input": {"type_url": "a"},
                                "value_match": {"Exact": "x"},
                            },
                        ],
                    },
                    "on_match": {"type": "action", "action": "ok"},
                }
            ]
        }
        config = parse_matcher_config(data)
        pred = config.matchers[0].predicate
        assert isinstance(pred, OrPredicateConfig)

    def test_not_predicate(self) -> None:
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "not",
                        "predicate": {
                            "type": "single",
                            "input": {"type_url": "a"},
                            "value_match": {"Exact": "x"},
                        },
                    },
                    "on_match": {"type": "action", "action": "ok"},
                }
            ]
        }
        config = parse_matcher_config(data)
        pred = config.matchers[0].predicate
        assert isinstance(pred, NotPredicateConfig)

    def test_nested_matcher(self) -> None:
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {"type_url": "a"},
                        "value_match": {"Prefix": ""},
                    },
                    "on_match": {
                        "type": "matcher",
                        "matcher": {
                            "matchers": [
                                {
                                    "predicate": {
                                        "type": "single",
                                        "input": {"type_url": "a"},
                                        "value_match": {"Exact": "deep"},
                                    },
                                    "on_match": {"type": "action", "action": "nested"},
                                }
                            ]
                        },
                    },
                }
            ]
        }
        config = parse_matcher_config(data)
        on_match = config.matchers[0].on_match
        assert isinstance(on_match, MatcherOnMatchConfig)
        assert len(on_match.matcher.matchers) == 1

    def test_typed_config_defaults_to_empty_object(self) -> None:
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {"type_url": "test.Input"},
                        "value_match": {"Exact": "x"},
                    },
                    "on_match": {"type": "action", "action": "ok"},
                }
            ]
        }
        config = parse_matcher_config(data)
        pred = config.matchers[0].predicate
        assert isinstance(pred, SinglePredicateConfig)
        assert pred.input.config == {}

    def test_no_on_no_match_is_none(self) -> None:
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {"type_url": "a"},
                        "value_match": {"Exact": "x"},
                    },
                    "on_match": {"type": "action", "action": "ok"},
                }
            ]
        }
        config = parse_matcher_config(data)
        assert config.on_no_match is None

    def test_custom_match(self) -> None:
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {"type_url": "a"},
                        "custom_match": {
                            "type_url": "custom.Matcher",
                            "config": {"foo": "bar"},
                        },
                    },
                    "on_match": {"type": "action", "action": "ok"},
                }
            ]
        }
        config = parse_matcher_config(data)
        pred = config.matchers[0].predicate
        assert isinstance(pred, SinglePredicateConfig)
        assert isinstance(pred.matcher, CustomMatch)
        assert pred.matcher.typed_config.type_url == "custom.Matcher"

    def test_all_string_match_variants(self) -> None:
        """Verify all 5 built-in string match variants parse."""
        for variant in ("Exact", "Prefix", "Suffix", "Contains", "Regex"):
            data = {
                "matchers": [
                    {
                        "predicate": {
                            "type": "single",
                            "input": {"type_url": "a"},
                            "value_match": {variant: "test"},
                        },
                        "on_match": {"type": "action", "action": "ok"},
                    }
                ]
            }
            config = parse_matcher_config(data)
            pred = config.matchers[0].predicate
            assert isinstance(pred, SinglePredicateConfig)
            assert isinstance(pred.matcher, BuiltInMatch)
            assert pred.matcher.variant == variant
            assert pred.matcher.value == "test"


class TestParseErrors:
    """Tests for parse error cases."""

    def test_missing_matchers(self) -> None:
        with pytest.raises(ConfigParseError, match="missing required field 'matchers'"):
            parse_matcher_config({})

    def test_not_a_dict(self) -> None:
        with pytest.raises(ConfigParseError, match="expected dict"):
            parse_matcher_config("not a dict")  # type: ignore[arg-type]

    def test_both_value_and_custom_match(self) -> None:
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {"type_url": "a", "config": {"key": "x"}},
                        "value_match": {"Exact": "a"},
                        "custom_match": {"type_url": "b", "config": {}},
                    },
                    "on_match": {"type": "action", "action": "x"},
                }
            ]
        }
        with pytest.raises(ConfigParseError, match="exactly one"):
            parse_matcher_config(data)

    def test_neither_value_nor_custom_match(self) -> None:
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {"type_url": "a", "config": {"key": "x"}},
                    },
                    "on_match": {"type": "action", "action": "x"},
                }
            ]
        }
        with pytest.raises(ConfigParseError, match="required"):
            parse_matcher_config(data)

    def test_unknown_predicate_type(self) -> None:
        data = {
            "matchers": [
                {
                    "predicate": {"type": "xor", "predicates": []},
                    "on_match": {"type": "action", "action": "x"},
                }
            ]
        }
        with pytest.raises(ConfigParseError, match="unknown predicate type"):
            parse_matcher_config(data)

    def test_unknown_on_match_type(self) -> None:
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {"type_url": "a"},
                        "value_match": {"Exact": "x"},
                    },
                    "on_match": {"type": "unknown"},
                }
            ]
        }
        with pytest.raises(ConfigParseError, match="unknown on_match type"):
            parse_matcher_config(data)

    def test_unknown_value_match_variant(self) -> None:
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {"type_url": "a"},
                        "value_match": {"StartsWith": "x"},
                    },
                    "on_match": {"type": "action", "action": "x"},
                }
            ]
        }
        with pytest.raises(ConfigParseError, match="value_match must contain one of"):
            parse_matcher_config(data)

    def test_missing_type_url(self) -> None:
        data = {
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {"config": {}},
                        "value_match": {"Exact": "x"},
                    },
                    "on_match": {"type": "action", "action": "x"},
                }
            ]
        }
        with pytest.raises(ConfigParseError, match="type_url"):
            parse_matcher_config(data)
