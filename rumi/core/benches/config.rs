//! Config-path benchmarks — JSON → Registry → Matcher.
//!
//! Measures the cost of the config/registry loading layer added in Phase 13.
//! Compares against the manual construction path (Phase 9) to isolate overhead.

use rumi::prelude::*;
use rumi_test::TestContext;

fn main() {
    divan::main();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Shared JSON configs (identical across all implementations)
// ═══════════════════════════════════════════════════════════════════════════════

const SIMPLE_CONFIG: &str = r#"{
    "matchers": [{
        "predicate": {
            "type": "single",
            "input": { "type_url": "xuma.test.v1.StringInput", "config": { "key": "role" } },
            "value_match": { "Exact": "admin" }
        },
        "on_match": { "type": "action", "action": "matched" }
    }],
    "on_no_match": { "type": "action", "action": "default" }
}"#;

const COMPOUND_CONFIG: &str = r#"{
    "matchers": [{
        "predicate": {
            "type": "and",
            "predicates": [
                {
                    "type": "single",
                    "input": { "type_url": "xuma.test.v1.StringInput", "config": { "key": "role" } },
                    "value_match": { "Exact": "admin" }
                },
                {
                    "type": "single",
                    "input": { "type_url": "xuma.test.v1.StringInput", "config": { "key": "org" } },
                    "value_match": { "Prefix": "acme" }
                }
            ]
        },
        "on_match": { "type": "action", "action": "admin_acme" }
    }]
}"#;

const NESTED_CONFIG: &str = r#"{
    "matchers": [{
        "predicate": {
            "type": "single",
            "input": { "type_url": "xuma.test.v1.StringInput", "config": { "key": "tier" } },
            "value_match": { "Exact": "premium" }
        },
        "on_match": {
            "type": "matcher",
            "matcher": {
                "matchers": [{
                    "predicate": {
                        "type": "single",
                        "input": { "type_url": "xuma.test.v1.StringInput", "config": { "key": "region" } },
                        "value_match": { "Exact": "us" }
                    },
                    "on_match": { "type": "action", "action": "premium_us" }
                }],
                "on_no_match": { "type": "action", "action": "premium_other" }
            }
        }
    }],
    "on_no_match": { "type": "action", "action": "default" }
}"#;

// ═══════════════════════════════════════════════════════════════════════════════
// Registry construction (one-time cost)
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench]
fn config_registry_build(bencher: divan::Bencher) {
    bencher.bench_local(|| rumi_test::register(rumi::RegistryBuilder::new()).build());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Config loading: JSON string → Registry → Matcher
// ═══════════════════════════════════════════════════════════════════════════════

fn build_registry() -> rumi::Registry<TestContext> {
    rumi_test::register(rumi::RegistryBuilder::new()).build()
}

#[divan::bench]
fn config_load_simple(bencher: divan::Bencher) {
    let registry = build_registry();
    bencher.bench_local(|| {
        let config: rumi::MatcherConfig<String> = serde_json::from_str(SIMPLE_CONFIG).unwrap();
        registry.load_matcher(config).unwrap()
    });
}

#[divan::bench]
fn config_load_compound(bencher: divan::Bencher) {
    let registry = build_registry();
    bencher.bench_local(|| {
        let config: rumi::MatcherConfig<String> = serde_json::from_str(COMPOUND_CONFIG).unwrap();
        registry.load_matcher(config).unwrap()
    });
}

#[divan::bench]
fn config_load_nested(bencher: divan::Bencher) {
    let registry = build_registry();
    bencher.bench_local(|| {
        let config: rumi::MatcherConfig<String> = serde_json::from_str(NESTED_CONFIG).unwrap();
        registry.load_matcher(config).unwrap()
    });
}

// ═══════════════════════════════════════════════════════════════════════════════
// Evaluation parity: config-loaded matcher vs compiler-built matcher
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench]
fn config_evaluate_simple(bencher: divan::Bencher) {
    let registry = build_registry();
    let config: rumi::MatcherConfig<String> = serde_json::from_str(SIMPLE_CONFIG).unwrap();
    let matcher = registry.load_matcher(config).unwrap();
    let ctx = TestContext::new().with("role", "admin");

    bencher.bench_local(|| matcher.evaluate(&ctx));
}

#[divan::bench]
fn compiler_evaluate_simple(bencher: divan::Bencher) {
    let matcher = Matcher::new(
        vec![FieldMatcher::new(
            Predicate::Single(SinglePredicate::new(
                Box::new(rumi_test::StringInput::new("role")),
                Box::new(ExactMatcher::new("admin")),
            )),
            OnMatch::Action("matched".to_string()),
        )],
        Some(OnMatch::Action("default".to_string())),
    );
    let ctx = TestContext::new().with("role", "admin");

    bencher.bench_local(|| matcher.evaluate(&ctx));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Head-to-head: config load vs manual construction (same logical matcher)
// NOTE: config_construct_simple duplicates config_load_simple intentionally —
// both appear in the same divan output to compare config vs compiler
// construction side-by-side in benchmark results.
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench]
fn config_construct_simple(bencher: divan::Bencher) {
    let registry = build_registry();
    bencher.bench_local(|| {
        let config: rumi::MatcherConfig<String> = serde_json::from_str(SIMPLE_CONFIG).unwrap();
        registry.load_matcher(config).unwrap()
    });
}

#[divan::bench]
fn compiler_construct_simple(bencher: divan::Bencher) {
    bencher.bench_local(|| {
        Matcher::<TestContext, String>::new(
            vec![FieldMatcher::new(
                Predicate::Single(SinglePredicate::new(
                    Box::new(rumi_test::StringInput::new("role")),
                    Box::new(ExactMatcher::new("admin")),
                )),
                OnMatch::Action("matched".to_string()),
            )],
            Some(OnMatch::Action("default".to_string())),
        )
    });
}
