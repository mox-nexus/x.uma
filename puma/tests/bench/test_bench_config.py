"""Config-path benchmarks for puma.

Measures the cost of JSON config -> Registry -> Matcher construction, and
compares config-loaded evaluation against compiler-built evaluation.

The configs are canonical protojson (DECISIONS.md D-026) fed through
`parse_protojson`, the same reader `Registry.load_matcher` consumes from in
production -- this measures the real config path, not a synthetic one.

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
)
from xuma._protojson import parse_protojson
from xuma.testing import DictInput, register

# ── Shared JSON configs (identical across all implementations) ────────────────

def _map_input(key: str) -> dict:
    return {"@type": "type.googleapis.com/xuma.kv.v1.MapInput", "key": key}


def _named_action(name: str) -> dict:
    return {"@type": "type.googleapis.com/xuma.core.v1.NamedAction", "name": name}


def _single(input_name: str, key: str, variant: str, value: str) -> dict:
    return {
        "singlePredicate": {
            "input": {"name": input_name, "typedConfig": _map_input(key)},
            "valueMatch": {variant: value},
        }
    }


def _action_match(name: str) -> dict:
    return {"action": {"name": name, "typedConfig": _named_action(name)}}


SIMPLE_CONFIG = json.dumps(
    {
        "matcherList": {
            "matchers": [
                {
                    "predicate": _single("role", "role", "exact", "admin"),
                    "onMatch": _action_match("matched"),
                }
            ]
        },
        "onNoMatch": _action_match("default"),
    }
)

COMPOUND_CONFIG = json.dumps(
    {
        "matcherList": {
            "matchers": [
                {
                    "predicate": {
                        "andMatcher": {
                            "predicate": [
                                _single("role", "role", "exact", "admin"),
                                _single("org", "org", "prefix", "acme"),
                            ]
                        }
                    },
                    "onMatch": _action_match("admin_acme"),
                }
            ]
        }
    }
)

NESTED_CONFIG = json.dumps(
    {
        "matcherList": {
            "matchers": [
                {
                    "predicate": _single("tier", "tier", "exact", "premium"),
                    "onMatch": {
                        "matcher": {
                            "matcherList": {
                                "matchers": [
                                    {
                                        "predicate": _single("region", "region", "exact", "us"),
                                        "onMatch": _action_match("premium_us"),
                                    }
                                ]
                            },
                            "onNoMatch": _action_match("premium_other"),
                        }
                    },
                }
            ]
        },
        "onNoMatch": _action_match("default"),
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
        config = parse_protojson(json.loads(SIMPLE_CONFIG))
        return registry.load_matcher(config)

    benchmark(go)


def test_bench_config_load_compound(benchmark):
    """Config path: AND predicate."""
    registry = _build_registry()

    def go():
        config = parse_protojson(json.loads(COMPOUND_CONFIG))
        return registry.load_matcher(config)

    benchmark(go)


def test_bench_config_load_nested(benchmark):
    """Config path: nested matcher-in-matcher."""
    registry = _build_registry()

    def go():
        config = parse_protojson(json.loads(NESTED_CONFIG))
        return registry.load_matcher(config)

    benchmark(go)


# ── Evaluation parity ────────────────────────────────────────────────────────


def test_bench_config_evaluate_simple(benchmark):
    """Evaluate a config-loaded matcher (should match compiler path speed)."""
    registry = _build_registry()
    config = parse_protojson(json.loads(SIMPLE_CONFIG))
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
        config = parse_protojson(json.loads(SIMPLE_CONFIG))
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
