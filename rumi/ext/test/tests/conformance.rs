//! Conformance tests that run YAML fixtures against rumi
//!
//! Run with: cargo test -p rumi-test --test conformance --features rumi-test/fixtures
//!
//! Note: This test file requires the `fixtures` feature to be enabled.

#![cfg(feature = "fixtures")]

use rumi_test::fixture::Fixture;
use std::fs;
use std::path::{Path, PathBuf};

/// Get the spec/tests directory relative to the workspace root
fn fixtures_dir() -> PathBuf {
    // The manifest dir is ext/test, we need to go up to x.uma root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let ext_test = Path::new(manifest_dir);

    // Go up: ext/test -> ext -> rumi -> x.uma
    let xuma_root = ext_test
        .parent() // ext
        .and_then(|p| p.parent()) // rumi
        .and_then(|p| p.parent()) // x.uma
        .expect("Could not find x.uma root");

    xuma_root.join("spec").join("tests")
}

/// Load and run all fixtures in a directory
fn run_fixtures_in_dir(dir: &Path) {
    if !dir.exists() {
        panic!("Fixtures directory does not exist: {}", dir.display());
    }

    for entry in fs::read_dir(dir).expect("read dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();

        if path
            .extension()
            .map_or(false, |e| e == "yaml" || e == "yml")
        {
            println!("Running fixture: {}", path.display());

            let yaml = fs::read_to_string(&path).expect("read yaml");

            // Parse potentially multiple fixtures (separated by ---)
            let fixtures = Fixture::from_yaml_multi(&yaml).unwrap_or_else(|e| {
                panic!("Failed to parse {}: {}", path.display(), e);
            });

            for fixture in fixtures {
                println!("  Running: {}", fixture.name);
                fixture.run_and_assert();
            }
        }
    }
}

#[test]
fn test_string_matchers() {
    run_fixtures_in_dir(&fixtures_dir().join("01_string_matchers"));
}

#[test]
fn test_predicates() {
    run_fixtures_in_dir(&fixtures_dir().join("02_predicates"));
}

#[test]
fn test_semantics() {
    run_fixtures_in_dir(&fixtures_dir().join("03_semantics"));
}

#[test]
fn test_invariants() {
    run_fixtures_in_dir(&fixtures_dir().join("04_invariants"));
}
