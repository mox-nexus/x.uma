"""Head-to-head config benchmarks: puma vs xuma-crust (PyO3).

Compares the config loading path across both implementations to isolate:
1. Config parsing overhead -- JSON -> config types
2. Registry loading -- type URL lookup + factory invocation
3. Evaluation parity -- config-loaded matcher evaluation speed

The configs are canonical protojson (DECISIONS.md D-026), fed through
`parse_protojson` on the puma side and `from_config` on the crust side --
both are the real production entry points, not synthetic ones.

Run:
  cd rumi/crusts/python
  maturin develop
  uv run pytest tests/test_bench_config.py --benchmark-only
"""

from __future__ import annotations

import json

import pytest

from xuma_crust import HttpMatcher, TestMatcher as CrustTestMatcher

# Pure Python for comparison
from xuma import RegistryBuilder
from xuma._protojson import parse_protojson
from xuma.testing import register

# ── Shared protojson configs ─────────────────────────────────────────────────


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

HTTP_SIMPLE_CONFIG = json.dumps(
    {
        "matcherList": {
            "matchers": [
                {
                    "predicate": {
                        "singlePredicate": {
                            "input": {
                                "name": "path",
                                "typedConfig": {"@type": "type.googleapis.com/xuma.http.v1.PathInput"},
                            },
                            "valueMatch": {"exact": "/api/v1/users"},
                        }
                    },
                    "onMatch": _action_match("users_api"),
                }
            ]
        },
        "onNoMatch": _action_match("not_found"),
    }
)


# ── Helpers ──────────────────────────────────────────────────────────────────


def _puma_registry():
    return register(RegistryBuilder()).build()


# ── Config load: test domain ─────────────────────────────────────────────────


def test_bench_crusty_config_load_simple(benchmark):
    """Crusty: TestMatcher.from_config(json)."""
    benchmark(CrustTestMatcher.from_config, SIMPLE_CONFIG)


def test_bench_puma_config_load_simple(benchmark):
    """Puma: parse_protojson -> registry.load_matcher."""
    registry = _puma_registry()

    def go():
        config = parse_protojson(json.loads(SIMPLE_CONFIG))
        return registry.load_matcher(config)

    benchmark(go)


def test_bench_crusty_config_load_compound(benchmark):
    """Crusty: TestMatcher.from_config(compound json)."""
    benchmark(CrustTestMatcher.from_config, COMPOUND_CONFIG)


def test_bench_puma_config_load_compound(benchmark):
    """Puma: compound config via registry."""
    registry = _puma_registry()

    def go():
        config = parse_protojson(json.loads(COMPOUND_CONFIG))
        return registry.load_matcher(config)

    benchmark(go)


# ── Config evaluate: test domain ─────────────────────────────────────────────


@pytest.fixture
def crusty_config_matcher():
    return CrustTestMatcher.from_config(SIMPLE_CONFIG)


@pytest.fixture
def puma_config_matcher():
    registry = _puma_registry()
    config = parse_protojson(json.loads(SIMPLE_CONFIG))
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
