//! Running the protojson conformance suite from inside a crust.
//!
//! A binding runs the same fixtures through the same reader as the engine —
//! `parse_matcher` and `load_proto_matcher` — because what the suite checks is
//! that the binding's door reaches the engine unchanged.
//!
//! Byte-identical between the two crusts, deliberately: this file is the same
//! in `crusts/python` and `crusts/wasm` so a divergence between the wheel and
//! the npm package cannot start here.

use rumi_proto::convert::load_proto_matcher;
use rumi_proto::protojson::parse_matcher;
use rumi_test::implementations::Implementation;
use rumi_test::proto_fixture::{Domain, ProtoFixture};
use rumi_test::KvContext;

/// One line of the report: fixture, case, passed, detail.
pub type FixtureResult = (String, String, bool, String);

// A fixture loads through the same resolver, registries and action table as
// `from_config`, or a passing suite proves nothing about the FFI surface.

fn build(fixture: &ProtoFixture) -> Result<rumi::Matcher<KvContext, String>, rumi::MatcherError> {
    let resolver = crate::protojson::resolver();
    let proto = parse_matcher(resolver, fixture.proto_matcher.clone())?;
    load_proto_matcher(
        crate::kv_matcher::registry(),
        crate::protojson::actions(),
        resolver,
        &proto,
    )
}

/// The HTTP-domain matcher a fixture describes.
fn build_http(
    fixture: &ProtoFixture,
) -> Result<rumi::Matcher<rumi_http::HttpRequest, String>, rumi::MatcherError> {
    let resolver = crate::protojson::resolver();
    let proto = parse_matcher(resolver, fixture.proto_matcher.clone())?;
    load_proto_matcher(
        crate::http_matcher::registry(),
        crate::protojson::actions(),
        resolver,
        &proto,
    )
}

/// Load whichever domain the fixture names, discarding the matcher.
fn try_load(fixture: &ProtoFixture) -> Result<(), rumi::MatcherError> {
    match fixture.domain {
        Domain::Kv => build(fixture).map(|_| ()),
        Domain::Http => build_http(fixture).map(|_| ()),
    }
}

/// Evaluate every case, reporting one line each.
fn run_cases(fixture: &ProtoFixture, results: &mut Vec<FixtureResult>) {
    macro_rules! evaluate {
        ($matcher:expr, $context:expr) => {
            for case in &fixture.cases {
                let result = $matcher.evaluate(&$context(case));
                let passed = result == case.expect;
                let detail = if passed {
                    format!("got {result:?}")
                } else {
                    format!("expected {:?}, got {:?}", case.expect, result)
                };
                results.push((fixture.name.clone(), case.name.clone(), passed, detail));
            }
        };
    }

    match fixture.domain {
        Domain::Kv => match build(fixture) {
            Ok(m) => evaluate!(m, |case: &rumi_test::proto_fixture::ProtoTestCase| {
                case.context
                    .iter()
                    .fold(KvContext::new(), |c, (k, v)| c.with(k, v))
            }),
            Err(e) => results.push((
                fixture.name.clone(),
                "load".into(),
                false,
                format!("config load failed: {e}"),
            )),
        },
        Domain::Http => match build_http(fixture) {
            Ok(m) => evaluate!(m, |case: &rumi_test::proto_fixture::ProtoTestCase| {
                let spec = case
                    .http_request
                    .as_ref()
                    .expect("http fixture case has no http_request");
                spec.headers
                    .iter()
                    .fold(
                        rumi_http::HttpRequest::builder()
                            .method(&spec.method)
                            .path(&spec.path),
                        |b, (k, v)| b.header(k, v),
                    )
                    .build()
            }),
            Err(e) => results.push((
                fixture.name.clone(),
                "load".into(),
                false,
                format!("config load failed: {e}"),
            )),
        },
    }
}

/// Run every fixture in a protojson YAML file, as the binding `me`.
///
/// # Errors
///
/// If the file is not a set of `ProtoFixture` documents. A fixture that fails
/// is reported in the results rather than raised, so one bad case does not hide
/// the rest.
pub fn run_protojson(yaml: &str, me: Implementation) -> Result<Vec<FixtureResult>, String> {
    let fixtures = ProtoFixture::from_yaml_multi(yaml).map_err(|e| format!("invalid YAML: {e}"))?;

    let mut results = Vec::new();

    for fixture in &fixtures {
        // Not listed: this binding must not be able to run it. A skip that
        // quietly starts working is as much a defect in the ledger as one that
        // quietly starts failing.
        if !fixture.expects(me) {
            let passed = try_load(fixture).is_err();
            let detail = if passed {
                "not listed in `implementations`, and correctly does not load here".into()
            } else {
                "not listed in `implementations`, but it loads here — add this binding to the ledger"
                    .into()
            };
            results.push((fixture.name.clone(), "ledger".into(), passed, detail));
            continue;
        }

        if fixture.expect_error {
            let outcome = try_load(fixture);
            let passed = outcome.is_err();
            let detail = match &outcome {
                Err(e) => format!("correctly rejected: {e}"),
                Ok(()) => "expected a load error, but it loaded".into(),
            };
            results.push((fixture.name.clone(), "expect_error".into(), passed, detail));
            continue;
        }

        run_cases(fixture, &mut results);
    }

    Ok(results)
}
