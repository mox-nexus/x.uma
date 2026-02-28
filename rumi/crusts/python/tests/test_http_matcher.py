"""Tests for xuma-crust HttpMatcher.

Covers:
- Config-driven matcher loading from JSON
- HTTP request evaluation (method, path, headers, query params)
- Compound predicates (AND, OR, NOT)
- Nested matchers
- Fallback actions
- Error handling (invalid JSON, unknown type URLs, invalid regex)
- Trace debugging
"""

import json

import pytest

from xuma_crust import HttpMatcher


# ═══════════════════════════════════════════════════════════════════════════════
# Basic: single field matchers
# ═══════════════════════════════════════════════════════════════════════════════


class TestBasicMatching:
    """Single-field config-driven HTTP matching."""

    def test_path_exact_match(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                    "value_match": {"Exact": "/api/users"},
                },
                "on_match": {"type": "action", "action": "users"},
            }],
        })
        matcher = HttpMatcher.from_config(config)
        assert matcher.evaluate("GET", "/api/users") == "users"
        assert matcher.evaluate("GET", "/api/orders") is None

    def test_path_prefix_match(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                    "value_match": {"Prefix": "/api"},
                },
                "on_match": {"type": "action", "action": "api_backend"},
            }],
            "on_no_match": {"type": "action", "action": "default"},
        })
        matcher = HttpMatcher.from_config(config)
        assert matcher.evaluate("GET", "/api/users") == "api_backend"
        assert matcher.evaluate("GET", "/api/orders") == "api_backend"
        assert matcher.evaluate("GET", "/health") == "default"

    def test_method_exact_match(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {"type_url": "xuma.http.v1.MethodInput", "config": {}},
                    "value_match": {"Exact": "POST"},
                },
                "on_match": {"type": "action", "action": "write"},
            }],
        })
        matcher = HttpMatcher.from_config(config)
        assert matcher.evaluate("POST", "/anything") == "write"
        assert matcher.evaluate("GET", "/anything") is None

    def test_header_match(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.http.v1.HeaderInput",
                        "config": {"name": "content-type"},
                    },
                    "value_match": {"Exact": "application/json"},
                },
                "on_match": {"type": "action", "action": "json_handler"},
            }],
        })
        matcher = HttpMatcher.from_config(config)
        assert matcher.evaluate(
            "POST", "/api", headers={"content-type": "application/json"},
        ) == "json_handler"
        assert matcher.evaluate(
            "POST", "/api", headers={"content-type": "text/html"},
        ) is None
        # Missing header
        assert matcher.evaluate("POST", "/api") is None

    def test_query_param_match(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.http.v1.QueryParamInput",
                        "config": {"name": "format"},
                    },
                    "value_match": {"Exact": "json"},
                },
                "on_match": {"type": "action", "action": "json_response"},
            }],
        })
        matcher = HttpMatcher.from_config(config)
        assert matcher.evaluate(
            "GET", "/api", query_params={"format": "json"},
        ) == "json_response"
        assert matcher.evaluate(
            "GET", "/api", query_params={"format": "xml"},
        ) is None


# ═══════════════════════════════════════════════════════════════════════════════
# Compound predicates
# ═══════════════════════════════════════════════════════════════════════════════


class TestCompoundPredicates:
    """AND, OR, NOT predicate compositions."""

    def test_and_path_and_method(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "and",
                    "predicates": [
                        {
                            "type": "single",
                            "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                            "value_match": {"Prefix": "/api"},
                        },
                        {
                            "type": "single",
                            "input": {"type_url": "xuma.http.v1.MethodInput", "config": {}},
                            "value_match": {"Exact": "POST"},
                        },
                    ],
                },
                "on_match": {"type": "action", "action": "api_write"},
            }],
        })
        matcher = HttpMatcher.from_config(config)
        assert matcher.evaluate("POST", "/api/users") == "api_write"
        assert matcher.evaluate("GET", "/api/users") is None
        assert matcher.evaluate("POST", "/health") is None

    def test_or_predicate(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "or",
                    "predicates": [
                        {
                            "type": "single",
                            "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                            "value_match": {"Exact": "/health"},
                        },
                        {
                            "type": "single",
                            "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                            "value_match": {"Exact": "/ready"},
                        },
                    ],
                },
                "on_match": {"type": "action", "action": "probe"},
            }],
        })
        matcher = HttpMatcher.from_config(config)
        assert matcher.evaluate("GET", "/health") == "probe"
        assert matcher.evaluate("GET", "/ready") == "probe"
        assert matcher.evaluate("GET", "/api") is None

    def test_not_predicate(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "not",
                    "predicate": {
                        "type": "single",
                        "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                        "value_match": {"Prefix": "/internal"},
                    },
                },
                "on_match": {"type": "action", "action": "public"},
            }],
        })
        matcher = HttpMatcher.from_config(config)
        assert matcher.evaluate("GET", "/api/users") == "public"
        assert matcher.evaluate("GET", "/internal/debug") is None


# ═══════════════════════════════════════════════════════════════════════════════
# Nested matchers and fallback
# ═══════════════════════════════════════════════════════════════════════════════


class TestNesting:
    """Nested matchers and on_no_match fallback."""

    def test_nested_matcher(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                    "value_match": {"Prefix": "/api"},
                },
                "on_match": {
                    "type": "matcher",
                    "matcher": {
                        "matchers": [{
                            "predicate": {
                                "type": "single",
                                "input": {"type_url": "xuma.http.v1.MethodInput", "config": {}},
                                "value_match": {"Exact": "POST"},
                            },
                            "on_match": {"type": "action", "action": "api_write"},
                        }],
                        "on_no_match": {"type": "action", "action": "api_read"},
                    },
                },
            }],
            "on_no_match": {"type": "action", "action": "default"},
        })
        matcher = HttpMatcher.from_config(config)
        assert matcher.evaluate("POST", "/api/users") == "api_write"
        assert matcher.evaluate("GET", "/api/users") == "api_read"
        assert matcher.evaluate("GET", "/health") == "default"

    def test_multiple_field_matchers_first_wins(self):
        config = json.dumps({
            "matchers": [
                {
                    "predicate": {
                        "type": "single",
                        "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                        "value_match": {"Exact": "/health"},
                    },
                    "on_match": {"type": "action", "action": "health"},
                },
                {
                    "predicate": {
                        "type": "single",
                        "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                        "value_match": {"Prefix": "/"},
                    },
                    "on_match": {"type": "action", "action": "catch_all"},
                },
            ],
        })
        matcher = HttpMatcher.from_config(config)
        # First match wins
        assert matcher.evaluate("GET", "/health") == "health"
        assert matcher.evaluate("GET", "/anything") == "catch_all"


# ═══════════════════════════════════════════════════════════════════════════════
# String match types
# ═══════════════════════════════════════════════════════════════════════════════


class TestStringMatchTypes:
    """All built-in string match types."""

    def _make_path_matcher(self, value_match):
        return HttpMatcher.from_config(json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                    "value_match": value_match,
                },
                "on_match": {"type": "action", "action": "matched"},
            }],
        }))

    def test_exact(self):
        m = self._make_path_matcher({"Exact": "/api"})
        assert m.evaluate("GET", "/api") == "matched"
        assert m.evaluate("GET", "/api/extra") is None

    def test_prefix(self):
        m = self._make_path_matcher({"Prefix": "/api"})
        assert m.evaluate("GET", "/api") == "matched"
        assert m.evaluate("GET", "/api/users") == "matched"
        assert m.evaluate("GET", "/health") is None

    def test_suffix(self):
        m = self._make_path_matcher({"Suffix": ".json"})
        assert m.evaluate("GET", "/data.json") == "matched"
        assert m.evaluate("GET", "/data.xml") is None

    def test_contains(self):
        m = self._make_path_matcher({"Contains": "users"})
        assert m.evaluate("GET", "/api/users/123") == "matched"
        assert m.evaluate("GET", "/api/orders") is None

    def test_regex(self):
        m = self._make_path_matcher({"Regex": r"^/api/v\d+/"})
        assert m.evaluate("GET", "/api/v1/users") == "matched"
        assert m.evaluate("GET", "/api/v2/orders") == "matched"
        assert m.evaluate("GET", "/api/users") is None


# ═══════════════════════════════════════════════════════════════════════════════
# Error handling
# ═══════════════════════════════════════════════════════════════════════════════


class TestErrors:
    """Error cases produce clear ValueError messages."""

    def test_invalid_json(self):
        with pytest.raises(ValueError, match="invalid config JSON"):
            HttpMatcher.from_config("not json")

    def test_unknown_type_url(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {"type_url": "xuma.fake.v1.FakeInput", "config": {}},
                    "value_match": {"Exact": "x"},
                },
                "on_match": {"type": "action", "action": "x"},
            }],
        })
        with pytest.raises(ValueError, match="xuma.fake.v1.FakeInput"):
            HttpMatcher.from_config(config)

    def test_invalid_regex(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                    "value_match": {"Regex": "[invalid"},
                },
                "on_match": {"type": "action", "action": "x"},
            }],
        })
        with pytest.raises(ValueError):
            HttpMatcher.from_config(config)

    def test_missing_on_match(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                    "value_match": {"Exact": "/"},
                },
            }],
        })
        with pytest.raises(ValueError, match="invalid config JSON"):
            HttpMatcher.from_config(config)


# ═══════════════════════════════════════════════════════════════════════════════
# Trace debugging
# ═══════════════════════════════════════════════════════════════════════════════


class TestTrace:
    """Trace evaluation for debugging."""

    def test_trace_result_matches_evaluate(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                    "value_match": {"Prefix": "/api"},
                },
                "on_match": {"type": "action", "action": "api"},
            }],
            "on_no_match": {"type": "action", "action": "default"},
        })
        matcher = HttpMatcher.from_config(config)

        eval_result = matcher.evaluate("GET", "/api/users")
        trace = matcher.trace("GET", "/api/users")
        assert trace.result == eval_result

    def test_trace_has_steps(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                    "value_match": {"Prefix": "/api"},
                },
                "on_match": {"type": "action", "action": "api"},
            }],
        })
        matcher = HttpMatcher.from_config(config)
        trace = matcher.trace("GET", "/api/users")
        assert len(trace.steps) > 0
        assert trace.steps[0].matched is True

    def test_trace_fallback(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                    "value_match": {"Exact": "/api"},
                },
                "on_match": {"type": "action", "action": "api"},
            }],
            "on_no_match": {"type": "action", "action": "fallback"},
        })
        matcher = HttpMatcher.from_config(config)
        trace = matcher.trace("GET", "/other")
        assert trace.result == "fallback"
        assert trace.used_fallback is True


# ═══════════════════════════════════════════════════════════════════════════════
# DX: repr and ergonomics
# ═══════════════════════════════════════════════════════════════════════════════


class TestDeveloperExperience:
    """Repr and usability checks."""

    def test_repr(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {"type_url": "xuma.http.v1.PathInput", "config": {}},
                    "value_match": {"Exact": "/"},
                },
                "on_match": {"type": "action", "action": "root"},
            }],
        })
        matcher = HttpMatcher.from_config(config)
        assert "HttpMatcher" in repr(matcher)
