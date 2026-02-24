"""Head-to-head config benchmarks: puma (pure Python) vs puma-crusty (PyO3).

Compares the config loading path across both implementations to isolate:
1. Config parsing overhead — JSON → config types
2. Registry loading — type URL lookup + factory invocation
3. Evaluation parity — config-loaded matcher evaluation speed

Run:
  cd rumi/crusts/python
  maturin develop
  uv run pytest tests/test_bench_config.py --benchmark-only
"""

from __future__ import annotations

import json

import pytest

from puma_crusty import HttpMatcher, TestMatcher

# Pure Python for comparison
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

# ── Shared JSON configs ──────────────────────────────────────────────────────

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

HTTP_SIMPLE_CONFIG = json.dumps(
    {
        "matchers": [
            {
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.http.v1.PathInput",
                        "config": {},
                    },
                    "value_match": {"Exact": "/api/v1/users"},
                },
                "on_match": {"type": "action", "action": "users_api"},
            }
        ],
        "on_no_match": {"type": "action", "action": "not_found"},
    }
)


# ── Helpers ──────────────────────────────────────────────────────────────────


def _puma_registry():
    return register(RegistryBuilder()).build()


# ── Config load: test domain ─────────────────────────────────────────────────


def test_bench_crusty_config_load_simple(benchmark):
    """Crusty: TestMatcher.from_config(json)."""
    benchmark(TestMatcher.from_config, SIMPLE_CONFIG)


def test_bench_puma_config_load_simple(benchmark):
    """Puma: parse_matcher_config → registry.load_matcher."""
    registry = _puma_registry()

    def go():
        config = parse_matcher_config(json.loads(SIMPLE_CONFIG))
        return registry.load_matcher(config)

    benchmark(go)


def test_bench_crusty_config_load_compound(benchmark):
    """Crusty: TestMatcher.from_config(compound json)."""
    benchmark(TestMatcher.from_config, COMPOUND_CONFIG)


def test_bench_puma_config_load_compound(benchmark):
    """Puma: compound config via registry."""
    registry = _puma_registry()

    def go():
        config = parse_matcher_config(json.loads(COMPOUND_CONFIG))
        return registry.load_matcher(config)

    benchmark(go)


# ── Config evaluate: test domain ─────────────────────────────────────────────


@pytest.fixture
def crusty_config_matcher():
    return TestMatcher.from_config(SIMPLE_CONFIG)


@pytest.fixture
def puma_config_matcher():
    registry = _puma_registry()
    config = parse_matcher_config(json.loads(SIMPLE_CONFIG))
    return registry.load_matcher(config)


def test_bench_crusty_config_evaluate_simple(benchmark, crusty_config_matcher):
    """Crusty: evaluate config-loaded test matcher."""
    benchmark(crusty_config_matcher.evaluate, {"role": "admin"})


def test_bench_puma_config_evaluate_simple(benchmark, puma_config_matcher):
    """Puma: evaluate config-loaded test matcher."""
    ctx = {"role": "admin"}
    benchmark(puma_config_matcher.evaluate, ctx)


# ── HTTP domain (crusty only) ────────────────────────────────────────────────


def test_bench_crusty_http_config_load(benchmark):
    """Crusty: HttpMatcher.from_config(json)."""
    benchmark(HttpMatcher.from_config, HTTP_SIMPLE_CONFIG)


@pytest.fixture
def crusty_http_matcher():
    return HttpMatcher.from_config(HTTP_SIMPLE_CONFIG)


def test_bench_crusty_http_config_evaluate(benchmark, crusty_http_matcher):
    """Crusty: evaluate config-loaded HTTP matcher."""
    benchmark(crusty_http_matcher.evaluate, "GET", "/api/v1/users")
