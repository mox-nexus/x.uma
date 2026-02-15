//! Config conformance tests — validates the registry config loading path.
//!
//! These fixtures use the **same config format** across all implementations:
//! rumi (Rust), puma (Python), bumi (TypeScript), and both crusty bindings.
//!
//! Run with: cargo test -p rumi-test --test config_conformance --features rumi-test/registry,rumi-test/fixtures

#![cfg(all(feature = "fixtures", feature = "registry"))]

use rumi::MatcherConfig;
use rumi_test::config_fixture::ConfigFixture;
use std::fs;
use std::path::{Path, PathBuf};

/// Get the spec/tests directory relative to the workspace root.
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

/// Load and run all config fixtures in a directory.
fn run_config_fixtures(dir: &Path) {
    assert!(
        dir.exists(),
        "Config fixtures directory does not exist: {}",
        dir.display()
    );

    let registry = rumi_test::register(rumi::RegistryBuilder::new()).build();

    for entry in fs::read_dir(dir).expect("read dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();

        if !path
            .extension()
            .map_or(false, |e| e == "yaml" || e == "yml")
        {
            continue;
        }

        println!("Loading config fixture: {}", path.display());
        let yaml = fs::read_to_string(&path).expect("read yaml");
        let fixtures = ConfigFixture::from_yaml_multi(&yaml).unwrap_or_else(|e| {
            panic!("Failed to parse {}: {}", path.display(), e);
        });

        for fixture in fixtures {
            println!("  Running: {}", fixture.name);

            if fixture.expect_error {
                // Error fixture: either parse or load must fail
                let result: Result<MatcherConfig<String>, _> =
                    serde_json::from_value(fixture.config.clone());

                match result {
                    Err(_) => {
                        // Parse error — expected
                        println!("    -> parse error (expected)");
                    }
                    Ok(config) => {
                        // Parse succeeded, loading must fail
                        let load_result = registry.load_matcher(config);
                        assert!(
                            load_result.is_err(),
                            "Fixture '{}' expected error but load_matcher succeeded",
                            fixture.name,
                        );
                        println!("    -> load error: {} (expected)", load_result.unwrap_err());
                    }
                }
                continue;
            }

            // Positive fixture: parse and load must succeed
            let config: MatcherConfig<String> = serde_json::from_value(fixture.config.clone())
                .unwrap_or_else(|e| {
                    panic!("Fixture '{}' config parse failed: {}", fixture.name, e);
                });

            let matcher = registry.load_matcher(config).unwrap_or_else(|e| {
                panic!("Fixture '{}' load_matcher failed: {}", fixture.name, e);
            });

            // Run each test case
            for case in &fixture.cases {
                let ctx = case.build_context();
                let actual = matcher.evaluate(&ctx);
                assert_eq!(
                    actual, case.expect,
                    "Fixture '{}' case '{}': expected {:?}, got {:?}",
                    fixture.name, case.name, case.expect, actual,
                );
            }
        }
    }
}

#[test]
fn test_config_conformance() {
    run_config_fixtures(&fixtures_dir().join("06_config"));
}
