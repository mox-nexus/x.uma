//! Running the protojson conformance suite from inside a crust.
//!
//! The crusts are implementations four and five, so they run the same fixtures
//! through the same reader as rumi — `parse_matcher` and `load_proto_matcher`.
//! Reading a different config format from the other three would turn "all five
//! implementations agree" into a claim about five different questions.
//!
//! Byte-identical between the two crusts, deliberately: this file is the same
//! in `crusts/python` and `crusts/wasm` so a divergence between the wheel and
//! the npm package cannot start here.

use rumi_proto::convert::load_proto_matcher;
use rumi_proto::protojson::parse_matcher;
use rumi_test::proto_fixture::ProtoFixture;
use rumi_test::KvContext;

/// One line of the report: fixture, case, passed, detail.
pub type FixtureResult = (String, String, bool, String);

// The resolver and action registry are shared with `from_config` — see
// `crate::protojson`. Fixtures must exercise exactly the same door a real
// caller uses, or a passing suite proves nothing about the FFI surface.

fn build(fixture: &ProtoFixture) -> Result<rumi::Matcher<KvContext, String>, rumi::MatcherError> {
    let resolver = crate::protojson::resolver();
    let registry = rumi_test::register(rumi::RegistryBuilder::new()).build();
    let actions = crate::protojson::actions();

    let proto = parse_matcher(&resolver, fixture.proto_matcher.clone())?;
    load_proto_matcher(&registry, &actions, &resolver, &proto)
}

/// Run every fixture in a protojson YAML file.
///
/// # Errors
///
/// If the file is not a set of `ProtoFixture` documents. A fixture that fails
/// is reported in the results rather than raised, so one bad case does not hide
/// the rest.
pub fn run_protojson(yaml: &str) -> Result<Vec<FixtureResult>, String> {
    let fixtures = ProtoFixture::from_yaml_multi(yaml).map_err(|e| format!("invalid YAML: {e}"))?;

    let mut results = Vec::new();

    for fixture in &fixtures {
        if fixture.expect_error {
            let outcome = build(fixture);
            let passed = outcome.is_err();
            let detail = match &outcome {
                Err(e) => format!("correctly rejected: {e}"),
                Ok(_) => "expected a load error, but it loaded".into(),
            };
            results.push((fixture.name.clone(), "expect_error".into(), passed, detail));
            continue;
        }

        let matcher = match build(fixture) {
            Ok(m) => m,
            Err(e) => {
                results.push((
                    fixture.name.clone(),
                    "load".into(),
                    false,
                    format!("config load failed: {e}"),
                ));
                continue;
            }
        };

        for case in &fixture.cases {
            let ctx = case
                .context
                .iter()
                .fold(KvContext::new(), |c, (k, v)| c.with(k, v));
            let result = matcher.evaluate(&ctx);
            let passed = result == case.expect;
            let detail = if passed {
                format!("got {result:?}")
            } else {
                format!("expected {:?}, got {:?}", case.expect, result)
            };
            results.push((fixture.name.clone(), case.name.clone(), passed, detail));
        }
    }

    Ok(results)
}
