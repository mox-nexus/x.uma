"""Tests for xuma-crust TestMatcher.

Covers:
- Config-driven matcher loading from JSON
- Key-value context evaluation
- Conformance fixture execution
- Compound predicates, nesting, fallback
- Error handling
- Trace debugging
"""

import json
from pathlib import Path

import pytest

from xuma_crust import TestMatcher


FIXTURES_DIR = Path(__file__).resolve().parents[4] / "spec" / "tests" / "06_config"


# ═══════════════════════════════════════════════════════════════════════════════
# Basic: single field matchers
# ═══════════════════════════════════════════════════════════════════════════════


class TestBasicMatching:
    """Single-field config-driven test matching."""

    def test_exact_match_hit(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.test.v1.StringInput",
                        "config": {"key": "role"},
                    },
                    "value_match": {"Exact": "admin"},
                },
                "on_match": {"type": "action", "action": "allow"},
            }],
            "on_no_match": {"type": "action", "action": "deny"},
        })
        matcher = TestMatcher.from_config(config)
        assert matcher.evaluate({"role": "admin"}) == "allow"
        assert matcher.evaluate({"role": "viewer"}) == "deny"
        assert matcher.evaluate({"other": "admin"}) == "deny"

    def test_prefix_match(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.test.v1.StringInput",
                        "config": {"key": "email"},
                    },
                    "value_match": {"Suffix": "@acme.com"},
                },
                "on_match": {"type": "action", "action": "internal"},
            }],
        })
        matcher = TestMatcher.from_config(config)
        assert matcher.evaluate({"email": "alice@acme.com"}) == "internal"
        assert matcher.evaluate({"email": "bob@other.com"}) is None

    def test_regex_match(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.test.v1.StringInput",
                        "config": {"key": "version"},
                    },
                    "value_match": {"Regex": r"^v\d+\.\d+"},
                },
                "on_match": {"type": "action", "action": "valid"},
            }],
        })
        matcher = TestMatcher.from_config(config)
        assert matcher.evaluate({"version": "v1.0"}) == "valid"
        assert matcher.evaluate({"version": "latest"}) is None


# ═══════════════════════════════════════════════════════════════════════════════
# Compound predicates
# ═══════════════════════════════════════════════════════════════════════════════


class TestCompoundPredicates:
    """AND, OR, NOT compositions."""

    def test_and_predicate(self):
        config = json.dumps({
            "matchers": [{
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
            }],
        })
        matcher = TestMatcher.from_config(config)
        assert matcher.evaluate({"role": "admin", "org": "acme-corp"}) == "admin_acme"
        assert matcher.evaluate({"role": "admin", "org": "other"}) is None
        assert matcher.evaluate({"role": "viewer", "org": "acme-corp"}) is None

    def test_or_predicate(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "or",
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
                                "config": {"key": "role"},
                            },
                            "value_match": {"Exact": "superadmin"},
                        },
                    ],
                },
                "on_match": {"type": "action", "action": "privileged"},
            }],
        })
        matcher = TestMatcher.from_config(config)
        assert matcher.evaluate({"role": "admin"}) == "privileged"
        assert matcher.evaluate({"role": "superadmin"}) == "privileged"
        assert matcher.evaluate({"role": "viewer"}) is None

    def test_not_predicate(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "not",
                    "predicate": {
                        "type": "single",
                        "input": {
                            "type_url": "xuma.test.v1.StringInput",
                            "config": {"key": "env"},
                        },
                        "value_match": {"Exact": "prod"},
                    },
                },
                "on_match": {"type": "action", "action": "non_prod"},
            }],
        })
        matcher = TestMatcher.from_config(config)
        assert matcher.evaluate({"env": "staging"}) == "non_prod"
        assert matcher.evaluate({"env": "prod"}) is None


# ═══════════════════════════════════════════════════════════════════════════════
# Nesting and fallback
# ═══════════════════════════════════════════════════════════════════════════════


class TestNesting:
    """Nested matchers and on_no_match fallback."""

    def test_nested_matcher(self):
        """Mirrors spec/tests/06_config/03_nested_matcher.yaml."""
        config = json.dumps({
            "matchers": [{
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
                                    "value_match": {"Exact": "us-east"},
                                },
                                "on_match": {"type": "action", "action": "premium_us_east"},
                            },
                            {
                                "predicate": {
                                    "type": "single",
                                    "input": {
                                        "type_url": "xuma.test.v1.StringInput",
                                        "config": {"key": "region"},
                                    },
                                    "value_match": {"Exact": "eu-west"},
                                },
                                "on_match": {"type": "action", "action": "premium_eu_west"},
                            },
                        ],
                        "on_no_match": {"type": "action", "action": "premium_default"},
                    },
                },
            }],
            "on_no_match": {"type": "action", "action": "free_tier"},
        })
        matcher = TestMatcher.from_config(config)
        assert matcher.evaluate({"tier": "premium", "region": "us-east"}) == "premium_us_east"
        assert matcher.evaluate({"tier": "premium", "region": "eu-west"}) == "premium_eu_west"
        assert matcher.evaluate({"tier": "premium", "region": "ap-south"}) == "premium_default"
        assert matcher.evaluate({"tier": "free", "region": "us-east"}) == "free_tier"
        assert matcher.evaluate({"region": "us-east"}) == "free_tier"


# ═══════════════════════════════════════════════════════════════════════════════
# Conformance fixtures
# ═══════════════════════════════════════════════════════════════════════════════


class TestConformance:
    """Run the conformance fixtures from spec/tests/06_config/."""

    @pytest.fixture(params=sorted(FIXTURES_DIR.glob("*.yaml")), ids=lambda p: p.stem)
    def fixture_file(self, request):
        return request.param

    def test_fixture(self, fixture_file):
        """Run all test cases from a single fixture file."""
        yaml_content = fixture_file.read_text()
        results = TestMatcher.run_fixtures(yaml_content)
        failures = [(name, case, detail) for name, case, passed, detail in results if not passed]
        assert not failures, f"Failed cases: {failures}"


# ═══════════════════════════════════════════════════════════════════════════════
# Error handling
# ═══════════════════════════════════════════════════════════════════════════════


class TestErrors:
    """Error cases produce clear ValueError messages."""

    def test_invalid_json(self):
        with pytest.raises(ValueError, match="invalid config JSON"):
            TestMatcher.from_config("{bad json}")

    def test_unknown_type_url(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {"type_url": "xuma.fake.v1.Unknown", "config": {}},
                    "value_match": {"Exact": "x"},
                },
                "on_match": {"type": "action", "action": "x"},
            }],
        })
        with pytest.raises(ValueError, match="xuma.fake.v1.Unknown"):
            TestMatcher.from_config(config)

    def test_invalid_regex(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.test.v1.StringInput",
                        "config": {"key": "name"},
                    },
                    "value_match": {"Regex": "[invalid"},
                },
                "on_match": {"type": "action", "action": "x"},
            }],
        })
        with pytest.raises(ValueError):
            TestMatcher.from_config(config)


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
                    "input": {
                        "type_url": "xuma.test.v1.StringInput",
                        "config": {"key": "role"},
                    },
                    "value_match": {"Exact": "admin"},
                },
                "on_match": {"type": "action", "action": "allow"},
            }],
            "on_no_match": {"type": "action", "action": "deny"},
        })
        matcher = TestMatcher.from_config(config)
        eval_result = matcher.evaluate({"role": "admin"})
        trace = matcher.trace({"role": "admin"})
        assert trace.result == eval_result

    def test_trace_miss_uses_fallback(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.test.v1.StringInput",
                        "config": {"key": "role"},
                    },
                    "value_match": {"Exact": "admin"},
                },
                "on_match": {"type": "action", "action": "allow"},
            }],
            "on_no_match": {"type": "action", "action": "deny"},
        })
        matcher = TestMatcher.from_config(config)
        trace = matcher.trace({"role": "viewer"})
        assert trace.result == "deny"
        assert trace.used_fallback is True


# ═══════════════════════════════════════════════════════════════════════════════
# DX
# ═══════════════════════════════════════════════════════════════════════════════


class TestDeveloperExperience:
    """Repr and usability checks."""

    def test_repr(self):
        config = json.dumps({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.test.v1.StringInput",
                        "config": {"key": "x"},
                    },
                    "value_match": {"Exact": "y"},
                },
                "on_match": {"type": "action", "action": "z"},
            }],
        })
        matcher = TestMatcher.from_config(config)
        assert "TestMatcher" in repr(matcher)
