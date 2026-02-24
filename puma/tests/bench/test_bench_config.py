"""Config-path benchmarks for puma (Pure Python).

Measures the cost of JSON config → Registry → Matcher construction,
and compares config-loaded evaluation against compiler-built evaluation.

Run: cd puma && uv run pytest tests/bench/test_bench_config.py --benchmark-only
"""

from __future__ import annotations

import json

from xuma import (
    Action,
    ExactMatcher,
    FieldMatcher,
    Matcher,
    RegistryBuilder,
    SinglePredicate,
    parse_matcher_config,
)
from xuma.testing import DictInput, register

# ── Shared JSON configs (identical across all implementations) ────────────────

SIMPLE_CONFIG = json.dumps(
    {
        "matchers": [
            {
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.test.v1.StringInput",
                        "config": {"key": "role"},
                    },
                    "value_match": {"Exact": "admin"},
                },
                "on_match": {"type": "action", "action": "matched"},
            }
        ],
        "on_no_match": {"type": "action", "action": "default"},
    }
)

COMPOUND_CONFIG = json.dumps(
    {
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
)

NESTED_CONFIG = json.dumps(
    {
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
                    "type": "matcher",
                    "matcher": {
                        "matchers": [
                            {
                                "predicate": {
                                    "type": "single",
                                    "input": {
                                        "type_url": "xuma.test.v1.StringInput",
                                        "config": {"key": "region"},
                                    },
                                    "value_match": {"Exact": "us"},
                                },
                                "on_match": {
                                    "type": "action",
                                    "action": "premium_us",
                                },
                            }
                        ],
                        "on_no_match": {
                            "type": "action",
                            "action": "premium_other",
                        },
                    },
                },
            }
        ],
        "on_no_match": {"type": "action", "action": "default"},
    }
)


# ── Registry construction ────────────────────────────────────────────────────


def _build_registry():
    return register(RegistryBuilder()).build()


def test_bench_config_registry_build(benchmark):
    """One-time registry construction cost."""
    benchmark(_build_registry)


# ── Config loading: JSON → parse → Registry → Matcher ────────────────────────


def test_bench_config_load_simple(benchmark):
    """Config path: single exact match."""
    registry = _build_registry()

    def go():
        config = parse_matcher_config(json.loads(SIMPLE_CONFIG))
        return registry.load_matcher(config)

    benchmark(go)


def test_bench_config_load_compound(benchmark):
    """Config path: AND predicate."""
    registry = _build_registry()

    def go():
        config = parse_matcher_config(json.loads(COMPOUND_CONFIG))
        return registry.load_matcher(config)

    benchmark(go)


def test_bench_config_load_nested(benchmark):
    """Config path: nested matcher-in-matcher."""
    registry = _build_registry()

    def go():
        config = parse_matcher_config(json.loads(NESTED_CONFIG))
        return registry.load_matcher(config)

    benchmark(go)


# ── Evaluation parity ────────────────────────────────────────────────────────


def test_bench_config_evaluate_simple(benchmark):
    """Evaluate a config-loaded matcher (should match compiler path speed)."""
    registry = _build_registry()
    config = parse_matcher_config(json.loads(SIMPLE_CONFIG))
    matcher = registry.load_matcher(config)
    ctx = {"role": "admin"}

    benchmark(matcher.evaluate, ctx)


def test_bench_compiler_evaluate_simple(benchmark):
    """Evaluate a manually-constructed matcher (compiler path baseline)."""
    matcher = Matcher(
        matcher_list=(
            FieldMatcher(
                predicate=SinglePredicate(
                    input=DictInput("role"),
                    matcher=ExactMatcher("admin"),
                ),
                on_match=Action("matched"),
            ),
        ),
        on_no_match=Action("default"),
    )
    ctx = {"role": "admin"}

    benchmark(matcher.evaluate, ctx)


# ── Head-to-head: config load vs manual construction ─────────────────────────
# NOTE: config_construct_simple duplicates config_load_simple intentionally —
# both appear in the same pytest-benchmark group to compare config vs compiler
# construction side-by-side in benchmark output.


def test_bench_config_construct_simple(benchmark):
    """Config path: full JSON → Matcher pipeline."""
    registry = _build_registry()

    def go():
        config = parse_matcher_config(json.loads(SIMPLE_CONFIG))
        return registry.load_matcher(config)

    benchmark(go)


def test_bench_compiler_construct_simple(benchmark):
    """Compiler path: manual Matcher construction."""

    def go():
        return Matcher(
            matcher_list=(
                FieldMatcher(
                    predicate=SinglePredicate(
                        input=DictInput("role"),
                        matcher=ExactMatcher("admin"),
                    ),
                    on_match=Action("matched"),
                ),
            ),
            on_no_match=Action("default"),
        )

    benchmark(go)
