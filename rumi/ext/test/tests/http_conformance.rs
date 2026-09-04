//! `spec/tests/05_http/` against rumi's Gateway API compiler.
//!
//! Added 2026-08-31 (PLAN.md CONF1). This suite had two runners — puma's and
//! bumi's — and none in the reference implementation, so "all implementations
//! pass all fixtures in `spec/tests/`" was true of `07_protojson/` and merely
//! assumed here.
//!
//! It is not a hypothetical gap. `http_empty_routes_matches_all` required an
//! empty route list to match everything, which made a fail-open a contract the
//! suite enforced (D-050) — and rumi was not running the file that said so.

#![cfg(all(feature = "fixtures", feature = "http"))]

use rumi_http::{compile_route_matches, HttpMessageBuilder, HttpRouteMatch};
use rumi_test::http_fixture::{HttpFixture, HttpRequestSpec};
use rumi_test::implementations::Implementation;
use std::path::PathBuf;

const ME: Implementation = Implementation::Rust;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../spec/tests/05_http")
}

fn load_all() -> Vec<HttpFixture> {
    let dir = fixture_dir();
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
        .collect();
    paths.sort();

    let mut out = Vec::new();
    for path in paths {
        let yaml = std::fs::read_to_string(&path).expect("readable fixture");
        let fixtures =
            HttpFixture::all_from_yaml(&yaml).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        out.extend(fixtures);
    }
    out
}

fn build(
    fixture: &HttpFixture,
) -> Result<rumi::Matcher<rumi_http::HttpMessage, String>, rumi::MatcherError> {
    let matches: Vec<HttpRouteMatch> = fixture.route_matches();
    compile_route_matches(
        &matches,
        fixture.action.clone(),
        fixture.on_no_match.clone(),
    )
}

fn message(spec: &HttpRequestSpec) -> rumi_http::HttpMessage {
    let mut b = HttpMessageBuilder::new()
        .method(&spec.method)
        .path(&spec.path);
    for (k, v) in &spec.headers {
        b = b.header(k, v);
    }
    b.build()
}

#[test]
fn http_fixtures_pass() {
    let fixtures = load_all();
    assert!(
        !fixtures.is_empty(),
        "no fixtures found under {} — this test would pass on an empty suite",
        fixture_dir().display()
    );

    let mut ran = 0;
    let mut skipped = 0;

    for fixture in &fixtures {
        // Not listed: this runner must NOT be able to run it. A skip that
        // quietly starts working is as much a defect in the ledger as one that
        // quietly starts failing — it means the list reports on work already
        // done. Same rule as the protojson runner.
        if !fixture.implementations.contains(&ME) {
            assert!(
                build(fixture).is_err(),
                "fixture '{}' does not list rust, but rust compiles it. Add rust to \
                 `implementations` — a stale exception hides a finished migration.",
                fixture.name
            );
            skipped += 1;
            continue;
        }

        if fixture.expect_error {
            let needle = fixture.error_contains.as_ref().unwrap_or_else(|| {
                panic!(
                    "fixture '{}' sets expect_error without error_contains, so it \
                     would pass on any failure at all",
                    fixture.name
                )
            });
            let err = build(fixture).err().unwrap_or_else(|| {
                panic!(
                    "fixture '{}' expected a compile error, but it compiled",
                    fixture.name
                )
            });
            let text = err.to_string();
            assert!(
                text.contains(needle),
                "fixture '{}' failed for the wrong reason.\n  wanted: {needle}\n  got:    {text}",
                fixture.name
            );
            ran += 1;
            continue;
        }

        let matcher = build(fixture)
            .unwrap_or_else(|e| panic!("fixture '{}' did not compile: {e}", fixture.name));

        for case in &fixture.cases {
            let got = matcher.evaluate(&message(&case.http_request));
            let want = case.expect.as_deref();
            assert_eq!(
                got.as_deref(),
                want,
                "fixture '{}', case '{}': expected {want:?}, got {got:?}",
                fixture.name,
                case.name
            );
        }
        ran += 1;
    }

    println!("05_http: {ran} fixtures ran, {skipped} not listed for rust");
    assert!(
        ran > 0,
        "every fixture was skipped — the ledger excludes rust entirely"
    );
}
