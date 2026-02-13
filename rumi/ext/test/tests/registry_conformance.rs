//! Registry-based conformance tests
//!
//! Proves that the generic config-driven path (Path B: `Registry::load_matcher()`)
//! produces identical results to the hand-coded path (Path A: `fixture::MatcherConfig::build()`).
//!
//! Run with: cargo test -p rumi-test --test registry_conformance --features rumi-test/registry,rumi-test/fixtures

#![cfg(all(feature = "fixtures", feature = "registry"))]

use rumi::{MatcherConfig, OnMatchConfig, PredicateConfig, SinglePredicateConfig, TypedConfig};
use rumi_test::fixture;
use std::fs;
use std::path::{Path, PathBuf};

/// Get the spec/tests directory relative to the workspace root
fn fixtures_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let ext_test = Path::new(manifest_dir);
    ext_test
        .parent() // ext
        .and_then(|p| p.parent()) // rumi
        .and_then(|p| p.parent()) // x.uma
        .expect("Could not find x.uma root")
        .join("spec")
        .join("tests")
}

// ═══════════════════════════════════════════════════════════════════════════════
// Fixture → Registry Config Conversion
// ═══════════════════════════════════════════════════════════════════════════════

fn convert_matcher(config: &fixture::MatcherConfig) -> MatcherConfig<String> {
    MatcherConfig {
        matchers: config.matchers.iter().map(convert_field_matcher).collect(),
        on_no_match: config.on_no_match.as_ref().map(|om| convert_on_match(om)),
    }
}

fn convert_field_matcher(fm: &fixture::FieldMatcherConfig) -> rumi::FieldMatcherConfig<String> {
    rumi::FieldMatcherConfig {
        predicate: convert_predicate(&fm.predicate),
        on_match: convert_on_match(&fm.on_match),
    }
}

fn convert_predicate(pred: &fixture::PredicateConfig) -> PredicateConfig {
    match pred {
        fixture::PredicateConfig::Single(s) => PredicateConfig::Single(convert_single(&s.single)),
        fixture::PredicateConfig::And(a) => PredicateConfig::And {
            predicates: a.and.iter().map(convert_predicate).collect(),
        },
        fixture::PredicateConfig::Or(o) => PredicateConfig::Or {
            predicates: o.or.iter().map(convert_predicate).collect(),
        },
        fixture::PredicateConfig::Not(n) => PredicateConfig::Not {
            predicate: Box::new(convert_predicate(&n.not)),
        },
    }
}

fn convert_single(single: &fixture::SinglePredicateConfig) -> SinglePredicateConfig {
    SinglePredicateConfig {
        input: TypedConfig {
            type_url: "xuma.test.v1.StringInput".to_string(),
            config: serde_json::json!({ "key": single.input.key }),
        },
        value_match: convert_value_match(&single.value_match),
    }
}

fn convert_value_match(vm: &fixture::ValueMatchConfig) -> rumi::StringMatchSpec {
    match vm {
        fixture::ValueMatchConfig::Exact(e) => rumi::StringMatchSpec::Exact(e.exact.clone()),
        fixture::ValueMatchConfig::Prefix(p) => rumi::StringMatchSpec::Prefix(p.prefix.clone()),
        fixture::ValueMatchConfig::Suffix(s) => rumi::StringMatchSpec::Suffix(s.suffix.clone()),
        fixture::ValueMatchConfig::Contains(c) => {
            rumi::StringMatchSpec::Contains(c.contains.clone())
        }
    }
}

fn convert_on_match(om: &fixture::OnMatchConfig) -> OnMatchConfig<String> {
    match om {
        fixture::OnMatchConfig::Action(a) => OnMatchConfig::Action {
            action: a.action.clone(),
        },
        fixture::OnMatchConfig::Matcher(m) => OnMatchConfig::Matcher {
            matcher: Box::new(convert_matcher(&m.matcher)),
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Runner
// ═══════════════════════════════════════════════════════════════════════════════

fn run_fixtures_via_registry(dir: &Path) {
    assert!(
        dir.exists(),
        "Fixtures directory does not exist: {}",
        dir.display()
    );

    let registry = rumi_test::register(rumi::RegistryBuilder::new()).build();

    for entry in fs::read_dir(dir).expect("read dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();

        if path
            .extension()
            .map_or(false, |e| e == "yaml" || e == "yml")
        {
            println!("Running fixture via registry: {}", path.display());

            let yaml = fs::read_to_string(&path).expect("read yaml");
            let fixtures = fixture::Fixture::from_yaml_multi(&yaml).unwrap_or_else(|e| {
                panic!("Failed to parse {}: {}", path.display(), e);
            });

            for fixture in fixtures {
                println!("  Running: {} (via registry)", fixture.name);

                // Convert fixture config → registry config
                let registry_config = convert_matcher(&fixture.matcher);

                // Load via registry (Path B)
                let matcher = registry.load_matcher(registry_config).unwrap_or_else(|e| {
                    panic!("Registry load failed for '{}': {}", fixture.name, e);
                });

                // Verify each case produces identical results
                for case in &fixture.cases {
                    let ctx = case.build_context();
                    let result = matcher.evaluate(&ctx);
                    assert_eq!(
                        result, case.expect,
                        "Fixture '{}' case '{}' (via registry): expected {:?}, got {:?}",
                        fixture.name, case.name, case.expect, result
                    );
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests — mirror the conformance.rs structure
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_string_matchers_via_registry() {
    run_fixtures_via_registry(&fixtures_dir().join("01_string_matchers"));
}

#[test]
fn test_predicates_via_registry() {
    run_fixtures_via_registry(&fixtures_dir().join("02_predicates"));
}

#[test]
fn test_semantics_via_registry() {
    run_fixtures_via_registry(&fixtures_dir().join("03_semantics"));
}

#[test]
fn test_invariants_via_registry() {
    run_fixtures_via_registry(&fixtures_dir().join("04_invariants"));
}
