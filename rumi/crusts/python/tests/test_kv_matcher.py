"""Tests for xuma-crust KvMatcher.

Covers:
- Config-driven matcher loading from JSON
- Key-value context evaluation
- Conformance fixture execution
- Compound predicates, nesting, fallback
- Error handling
- Trace debugging
"""

import json

from _terse_to_protojson import pj
from pathlib import Path

import pytest

from xuma_crust import KvMatcher as CrustTestMatcher


FIXTURES_DIR = Path(__file__).resolve().parents[4] / "spec" / "tests" / "07_protojson"


# ═══════════════════════════════════════════════════════════════════════════════
# Basic: single field matchers
# ═══════════════════════════════════════════════════════════════════════════════


class TestBasicMatching:
    """Single-field config-driven test matching."""

    def test_exact_match_hit(self):
        config = json.dumps(pj({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.core.v1.MapInput",
                        "config": {"key": "role"},
                    },
                    "value_match": {"Exact": "admin"},
                },
                "on_match": {"type": "action", "action": "allow"},
            }],
            "on_no_match": {"type": "action", "action": "deny"},
        }))
        matcher = CrustTestMatcher.from_config(config)
        assert matcher.evaluate({"role": "admin"}) == "allow"
        assert matcher.evaluate({"role": "viewer"}) == "deny"
        assert matcher.evaluate({"other": "admin"}) == "deny"

    def test_prefix_match(self):
        config = json.dumps(pj({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.core.v1.MapInput",
                        "config": {"key": "email"},
                    },
                    "value_match": {"Suffix": "@acme.com"},
                },
                "on_match": {"type": "action", "action": "internal"},
            }],
        }))
        matcher = CrustTestMatcher.from_config(config)
        assert matcher.evaluate({"email": "alice@acme.com"}) == "internal"
        assert matcher.evaluate({"email": "bob@other.com"}) is None

    def test_regex_match(self):
        config = json.dumps(pj({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.core.v1.MapInput",
                        "config": {"key": "version"},
                    },
                    "value_match": {"Regex": r"^v\d+\.\d+"},
                },
                "on_match": {"type": "action", "action": "valid"},
            }],
        }))
        matcher = CrustTestMatcher.from_config(config)
        assert matcher.evaluate({"version": "v1.0"}) == "valid"
        assert matcher.evaluate({"version": "latest"}) is None


# ═══════════════════════════════════════════════════════════════════════════════
# Compound predicates
# ═══════════════════════════════════════════════════════════════════════════════


class TestCompoundPredicates:
    """AND, OR, NOT compositions."""

    def test_and_predicate(self):
        config = json.dumps(pj({
            "matchers": [{
                "predicate": {
                    "type": "and",
                    "predicates": [
                        {
                            "type": "single",
                            "input": {
                                "type_url": "xuma.core.v1.MapInput",
                                "config": {"key": "role"},
                            },
                            "value_match": {"Exact": "admin"},
                        },
                        {
                            "type": "single",
                            "input": {
                                "type_url": "xuma.core.v1.MapInput",
                                "config": {"key": "org"},
                            },
                            "value_match": {"Prefix": "acme"},
                        },
                    ],
                },
                "on_match": {"type": "action", "action": "admin_acme"},
            }],
        }))
        matcher = CrustTestMatcher.from_config(config)
        assert matcher.evaluate({"role": "admin", "org": "acme-corp"}) == "admin_acme"
        assert matcher.evaluate({"role": "admin", "org": "other"}) is None
        assert matcher.evaluate({"role": "viewer", "org": "acme-corp"}) is None

    def test_or_predicate(self):
        config = json.dumps(pj({
            "matchers": [{
                "predicate": {
                    "type": "or",
                    "predicates": [
                        {
                            "type": "single",
                            "input": {
                                "type_url": "xuma.core.v1.MapInput",
                                "config": {"key": "role"},
                            },
                            "value_match": {"Exact": "admin"},
                        },
                        {
                            "type": "single",
                            "input": {
                                "type_url": "xuma.core.v1.MapInput",
                                "config": {"key": "role"},
                            },
                            "value_match": {"Exact": "superadmin"},
                        },
                    ],
                },
                "on_match": {"type": "action", "action": "privileged"},
            }],
        }))
        matcher = CrustTestMatcher.from_config(config)
        assert matcher.evaluate({"role": "admin"}) == "privileged"
        assert matcher.evaluate({"role": "superadmin"}) == "privileged"
        assert matcher.evaluate({"role": "viewer"}) is None

    def test_not_predicate(self):
        config = json.dumps(pj({
            "matchers": [{
                "predicate": {
                    "type": "not",
                    "predicate": {
                        "type": "single",
                        "input": {
                            "type_url": "xuma.core.v1.MapInput",
                            "config": {"key": "env"},
                        },
                        "value_match": {"Exact": "prod"},
                    },
                },
                "on_match": {"type": "action", "action": "non_prod"},
            }],
        }))
        matcher = CrustTestMatcher.from_config(config)
        assert matcher.evaluate({"env": "staging"}) == "non_prod"
        assert matcher.evaluate({"env": "prod"}) is None


# ═══════════════════════════════════════════════════════════════════════════════
# Nesting and fallback
# ═══════════════════════════════════════════════════════════════════════════════


class TestNesting:
    """Nested matchers and on_no_match fallback."""

    def test_nested_matcher(self):
        """Mirrors spec/tests/07_protojson/03_nested_matcher.yaml."""
        config = json.dumps(pj({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.core.v1.MapInput",
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
                                        "type_url": "xuma.core.v1.MapInput",
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
                                        "type_url": "xuma.core.v1.MapInput",
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
        }))
        matcher = CrustTestMatcher.from_config(config)
        assert matcher.evaluate({"tier": "premium", "region": "us-east"}) == "premium_us_east"
        assert matcher.evaluate({"tier": "premium", "region": "eu-west"}) == "premium_eu_west"
        assert matcher.evaluate({"tier": "premium", "region": "ap-south"}) == "premium_default"
        assert matcher.evaluate({"tier": "free", "region": "us-east"}) == "free_tier"
        assert matcher.evaluate({"region": "us-east"}) == "free_tier"


# ═══════════════════════════════════════════════════════════════════════════════
# Conformance fixtures
# ═══════════════════════════════════════════════════════════════════════════════


class TestConformance:
    """Run the conformance fixtures from spec/tests/07_protojson/.

    Same fixtures, same reader as rumi — see `crusts/*/src/fixtures.rs`.
    """

    @pytest.fixture(params=sorted(FIXTURES_DIR.glob("*.yaml")), ids=lambda p: p.stem)
    def fixture_file(self, request):
        return request.param

    def test_fixture(self, fixture_file):
        """Run all test cases from a single fixture file."""
        yaml_content = fixture_file.read_text()
        results = CrustTestMatcher.run_fixtures(yaml_content)
        failures = [(name, case, detail) for name, case, passed, detail in results if not passed]
        assert not failures, f"Failed cases: {failures}"

    def test_a_fixture_that_omits_this_binding_must_not_load_here(self):
        """The ledger holds in both directions: a fixture this binding is not
        listed for, but can load, is a stale exception and fails."""
        keyless_but_loadable = """
name: omits_python
implementations: [rust]
proto_matcher:
  matcherList:
    matchers:
      - predicate:
          singlePredicate:
            input:
              name: k
              typedConfig: {"@type": "type.googleapis.com/xuma.core.v1.MapInput", key: k}
            valueMatch: {exact: v}
        onMatch:
          action:
            name: hit
            typedConfig: {"@type": "type.googleapis.com/xuma.core.v1.NamedAction", name: hit}
cases: []
"""
        results = CrustTestMatcher.run_fixtures(keyless_but_loadable)
        assert results == [
            ("omits_python", "ledger", False,
             "not listed in `implementations`, but it loads here — add this binding to the ledger"),
        ]


# ═══════════════════════════════════════════════════════════════════════════════
# Error handling
# ═══════════════════════════════════════════════════════════════════════════════


class TestErrors:
    """Error cases produce clear ValueError messages."""

    def test_invalid_json(self):
        with pytest.raises(ValueError, match="invalid config JSON"):
            CrustTestMatcher.from_config("{bad json}")

    def test_unknown_type_url(self):
        config = json.dumps(pj({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {"type_url": "xuma.fake.v1.Unknown", "config": {}},
                    "value_match": {"Exact": "x"},
                },
                "on_match": {"type": "action", "action": "x"},
            }],
        }))
        with pytest.raises(ValueError, match="xuma.fake.v1.Unknown"):
            CrustTestMatcher.from_config(config)

    def test_invalid_regex(self):
        config = json.dumps(pj({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.core.v1.MapInput",
                        "config": {"key": "name"},
                    },
                    "value_match": {"Regex": "[invalid"},
                },
                "on_match": {"type": "action", "action": "x"},
            }],
        }))
        with pytest.raises(ValueError):
            CrustTestMatcher.from_config(config)


# ═══════════════════════════════════════════════════════════════════════════════
# Trace debugging
# ═══════════════════════════════════════════════════════════════════════════════


class TestTrace:
    """Trace evaluation for debugging."""

    def test_trace_result_matches_evaluate(self):
        config = json.dumps(pj({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.core.v1.MapInput",
                        "config": {"key": "role"},
                    },
                    "value_match": {"Exact": "admin"},
                },
                "on_match": {"type": "action", "action": "allow"},
            }],
            "on_no_match": {"type": "action", "action": "deny"},
        }))
        matcher = CrustTestMatcher.from_config(config)
        eval_result = matcher.evaluate({"role": "admin"})
        trace = matcher.trace({"role": "admin"})
        assert trace.result == eval_result

    def test_trace_miss_uses_fallback(self):
        config = json.dumps(pj({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.core.v1.MapInput",
                        "config": {"key": "role"},
                    },
                    "value_match": {"Exact": "admin"},
                },
                "on_match": {"type": "action", "action": "allow"},
            }],
            "on_no_match": {"type": "action", "action": "deny"},
        }))
        matcher = CrustTestMatcher.from_config(config)
        trace = matcher.trace({"role": "viewer"})
        assert trace.result == "deny"
        assert trace.used_fallback is True


# ═══════════════════════════════════════════════════════════════════════════════
# DX
# ═══════════════════════════════════════════════════════════════════════════════


class TestDeveloperExperience:
    """Repr and usability checks."""

    def test_repr(self):
        config = json.dumps(pj({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.core.v1.MapInput",
                        "config": {"key": "x"},
                    },
                    "value_match": {"Exact": "y"},
                },
                "on_match": {"type": "action", "action": "z"},
            }],
        }))
        matcher = CrustTestMatcher.from_config(config)
        assert "KvMatcher" in repr(matcher)


# ═══════════════════════════════════════════════════════════════════════════════
# The decision object, the action table, and metadata (D-056)
# ═══════════════════════════════════════════════════════════════════════════════


def _named(name, **metadata):
    action = {"@type": "type.googleapis.com/xuma.core.v1.NamedAction", "name": name}
    if metadata:
        action["metadata"] = metadata
    return {"action": {"name": name, "typedConfig": action}}


def _rule(key, value, on_match):
    return {
        "predicate": {
            "singlePredicate": {
                "input": {
                    "name": key,
                    "typedConfig": {"@type": "type.googleapis.com/xuma.core.v1.MapInput", "key": key},
                },
                "valueMatch": {"exact": value},
            }
        },
        "onMatch": on_match,
    }


TWO_RULES_AND_A_FALLBACK = json.dumps({
    "matcherList": {"matchers": [
        _rule("role", "admin", _named("allow", processor="audit")),
        _rule("role", "viewer", _named("read_only")),
    ]},
    "onNoMatch": _named("deny"),
})


class TestDecision:
    def test_a_hit_is_a_decision_with_its_index(self):
        matcher = CrustTestMatcher.from_config(TWO_RULES_AND_A_FALLBACK)
        d = matcher.decide({"role": "viewer"})
        assert (d.matched, d.action, d.index, d.fallback) == (True, "read_only", 1, False)

    def test_the_fallback_says_so(self):
        matcher = CrustTestMatcher.from_config(TWO_RULES_AND_A_FALLBACK)
        d = matcher.decide({"role": "auditor"})
        assert (d.matched, d.action, d.index, d.fallback) == (True, "deny", 2, True)

    def test_no_decision_at_all(self):
        matcher = CrustTestMatcher.from_config(json.dumps({
            "matcherList": {"matchers": [_rule("role", "admin", _named("allow"))]},
        }))
        d = matcher.decide({"role": "nobody"})
        assert (d.matched, d.action, d.index, d.fallback) == (False, None, None, False)

    def test_a_required_value_that_is_absent_is_undecided(self):
        required = _rule("role", "guest", _named("allow"))
        single = required["predicate"]["singlePredicate"]
        single["input"]["typedConfig"]["required"] = True
        required["predicate"] = {"notMatcher": {"singlePredicate": single}}
        matcher = CrustTestMatcher.from_config(json.dumps({
            "matcherList": {"matchers": [required]},
            "onNoMatch": _named("deny"),
        }))
        d = matcher.decide({})
        assert (d.matched, d.action, d.fallback, d.undecided) == (False, None, False, True)
        assert matcher.trace({}).undecided

    def test_evaluate_and_decide_agree(self):
        matcher = CrustTestMatcher.from_config(TWO_RULES_AND_A_FALLBACK)
        for ctx in ({"role": "admin"}, {"role": "viewer"}, {"role": "x"}, {}):
            assert matcher.decide(ctx).action == matcher.evaluate(ctx)


class TestActionTable:
    def test_actions_are_listed_in_document_order_with_the_fallback_last(self):
        matcher = CrustTestMatcher.from_config(TWO_RULES_AND_A_FALLBACK)
        assert matcher.actions() == ["allow", "read_only", "deny"]

    def test_metadata_is_carried_not_dropped(self):
        matcher = CrustTestMatcher.from_config(TWO_RULES_AND_A_FALLBACK)
        assert matcher.action_metadata(0) == {"processor": "audit"}
        assert matcher.action_metadata(1) == {}

    def test_metadata_out_of_range_is_an_error(self):
        matcher = CrustTestMatcher.from_config(TWO_RULES_AND_A_FALLBACK)
        with pytest.raises(ValueError, match="no action at index 3"):
            matcher.action_metadata(3)

    def test_the_trace_carries_the_index(self):
        matcher = CrustTestMatcher.from_config(TWO_RULES_AND_A_FALLBACK)
        trace = matcher.trace({"role": "admin"})
        assert trace.index == 0
        assert matcher.actions()[trace.index] == trace.result
        assert matcher.trace({"role": "nope"}).index == 2


class TestDocumentBudget:
    def test_a_document_over_a_mebibyte_is_refused_by_its_size(self):
        padding = "x" * (1 << 20)
        document = json.dumps({"matcherList": {"matchers": []}, "pad": padding})
        with pytest.raises(ValueError, match="bytes"):
            CrustTestMatcher.from_config(document)
