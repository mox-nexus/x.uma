"""Boundary cost of the PyO3 binding on the config path.

What the numbers isolate:
1. Config load — protojson text in, a compiled matcher out; this is the
   parse, the registry lookups and the factory calls, all in Rust
2. Evaluate — a dict marshalled into a KvContext, one walk, a string back

The configs are canonical protojson (DECISIONS.md D-026) fed through
`from_config`, the real entry point, not a synthetic one.

Run:
  cd rumi/crusts/python
  maturin develop --release   # a debug build inverts every verdict drawn from this
  uv run pytest tests/test_bench_config.py --benchmark-only
"""

from __future__ import annotations

import json

import pytest

from xuma_crust import HttpMatcher, TestMatcher

# ── Configs ──────────────────────────────────────────────────────────────────


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


# ── Load: key-value domain ───────────────────────────────────────────────────


def test_bench_crusty_config_load_simple(benchmark):
    benchmark(TestMatcher.from_config, SIMPLE_CONFIG)


def test_bench_crusty_config_load_compound(benchmark):
    benchmark(TestMatcher.from_config, COMPOUND_CONFIG)


# ── Evaluate: key-value domain ───────────────────────────────────────────────


@pytest.fixture
def crusty_config_matcher():
    return TestMatcher.from_config(SIMPLE_CONFIG)


def test_bench_crusty_config_evaluate_simple(benchmark, crusty_config_matcher):
    benchmark(crusty_config_matcher.evaluate, {"role": "admin"})


# ── HTTP domain ──────────────────────────────────────────────────────────────


def test_bench_crusty_http_config_load(benchmark):
    benchmark(HttpMatcher.from_config, HTTP_SIMPLE_CONFIG)


@pytest.fixture
def crusty_http_matcher():
    return HttpMatcher.from_config(HTTP_SIMPLE_CONFIG)


def test_bench_crusty_http_config_evaluate(benchmark, crusty_http_matcher):
    benchmark(crusty_http_matcher.evaluate, "GET", "/api/v1/users")
