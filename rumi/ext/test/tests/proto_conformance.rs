//! Conformance over fixtures written in canonical protojson.
//!
//! Run with:
//! `cargo test -p rumi-test --test proto_conformance --features rumi-test/registry,rumi-test/fixtures`

#![cfg(all(feature = "fixtures", feature = "registry"))]

use rumi_proto::any_resolver::{AnyResolver, AnyResolverBuilder};
use rumi_proto::convert::load_proto_matcher;
use rumi_proto::protojson::parse_matcher;
use rumi_test::proto_fixture::{Domain, Implementation, ProtoFixture};
use rumi_test::KvContext;
use std::fs;
use std::path::{Path, PathBuf};

const ME: Implementation = Implementation::Rust;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace root")
        .join("spec")
        .join("tests")
}

fn resolver() -> AnyResolver {
    AnyResolverBuilder::new()
        .register::<rumi_proto::xuma::kv::v1::MapInput>("xuma.kv.v1.MapInput")
        .register::<rumi_proto::xuma::http::v1::HeaderInput>("xuma.http.v1.HeaderInput")
        .register::<rumi_proto::xuma::http::v1::PathInput>("xuma.http.v1.PathInput")
        .register::<rumi_proto::xuma::http::v1::MethodInput>("xuma.http.v1.MethodInput")
        .register::<rumi_proto::xuma::http::v1::QueryParamInput>("xuma.http.v1.QueryParamInput")
        .register::<rumi_proto::xuma::http::v1::AuthorityInput>("xuma.http.v1.AuthorityInput")
        .register::<rumi_proto::xuma::http::v1::SchemeInput>("xuma.http.v1.SchemeInput")
        .register::<rumi_proto::xuma::core::v1::BoolMatcher>("xuma.core.v1.BoolMatcher")
        .register::<rumi_proto::xuma::core::v1::NamedAction>("xuma.core.v1.NamedAction")
        .build()
}

/// `NamedAction` → `String`. The action's `name` is the action.
struct NamedActionFactory;

impl rumi::IntoAction<String> for NamedActionFactory {
    type Config = rumi_proto::xuma::core::v1::NamedAction;

    fn from_config(config: Self::Config) -> Result<String, rumi::MatcherError> {
        // An empty name would make the rule *fire* and return "", leaving the
        // polarity to whatever the host does with it. Every other empty
        // identifier merely makes a predicate false.
        if config.name.is_empty() {
            return Err(rumi::MatcherError::EmptyIdentifier {
                what: "action name",
            });
        }
        Ok(config.name)
    }
}

/// Build the matcher a fixture describes, or say why it could not be built.
fn actions() -> rumi::ActionRegistry<String> {
    rumi::ActionRegistryBuilder::new()
        .action::<NamedActionFactory>("xuma.core.v1.NamedAction")
        .build()
}

fn build_matcher(
    fixture: &ProtoFixture,
) -> Result<rumi::Matcher<KvContext, String>, rumi::MatcherError> {
    let resolver = resolver();
    let registry = rumi_test::register(rumi::RegistryBuilder::new()).build();
    let proto = parse_matcher(&resolver, fixture.proto_matcher.clone())?;
    load_proto_matcher(&registry, &actions(), &resolver, &proto)
}

/// Build the HTTP-domain matcher a fixture describes.
///
/// `register_simple`, not `register`: the fixture's `http_request` describes a
/// plain request, and `HttpRequest` is the context that reads one. Both bind
/// the same `xuma.http.v1.*` type URLs, and a fixture here is what holds them
/// to meaning the same thing.
fn build_http_matcher(
    fixture: &ProtoFixture,
) -> Result<rumi::Matcher<rumi_http::HttpRequest, String>, rumi::MatcherError> {
    let resolver = resolver();
    let registry = rumi_http::register_simple(rumi::RegistryBuilder::new()).build();
    let proto = parse_matcher(&resolver, fixture.proto_matcher.clone())?;
    load_proto_matcher(&registry, &actions(), &resolver, &proto)
}

/// Load whichever domain the fixture names, discarding the matcher.
///
/// Used where only the *outcome* of loading matters, so the two domains'
/// different `Matcher<Ctx, _>` types do not need to unify.
fn try_load(fixture: &ProtoFixture) -> Result<(), rumi::MatcherError> {
    match fixture.domain {
        Domain::Kv => build_matcher(fixture).map(|_| ()),
        Domain::Http => build_http_matcher(fixture).map(|_| ()),
    }
}

fn run(fixture: &ProtoFixture) {
    // Not listed: this runner must NOT be able to run it. A skip that quietly
    // starts working is as much a defect in the ledger as one that quietly
    // starts failing — it means the list is reporting on work already done.
    if !fixture.expects(ME) {
        assert!(
            try_load(fixture).is_err(),
            "fixture '{}' does not list rust, but rust loads it. Add rust to \
             `implementations` — a stale exception hides a finished migration.",
            fixture.name
        );
        println!("  skipped (not listed): {}", fixture.name);
        return;
    }

    if fixture.expect_error {
        let err = try_load(fixture).err().unwrap_or_else(|| {
            panic!(
                "fixture '{}' expected a load error, but it loaded",
                fixture.name
            )
        });
        if let Some(needle) = &fixture.error_contains {
            let text = err.to_string();
            assert!(
                text.contains(needle),
                "fixture '{}' failed for the wrong reason.\n  wanted: {needle}\n  got:    {text}",
                fixture.name
            );
        }
        println!("  {} -> load error (expected): {err}", fixture.name);
        return;
    }

    match fixture.domain {
        Domain::Kv => {
            let matcher = build_matcher(fixture)
                .unwrap_or_else(|e| panic!("fixture '{}' failed to load: {e}", fixture.name));
            for case in &fixture.cases {
                let ctx = case
                    .context
                    .iter()
                    .fold(KvContext::new(), |c, (k, v)| c.with(k, v));
                assert_eq!(
                    matcher.evaluate(&ctx),
                    case.expect,
                    "fixture '{}' case '{}'",
                    fixture.name,
                    case.name
                );
            }
        }
        Domain::Http => {
            let matcher = build_http_matcher(fixture)
                .unwrap_or_else(|e| panic!("fixture '{}' failed to load: {e}", fixture.name));
            for case in &fixture.cases {
                let spec = case.http_request.as_ref().unwrap_or_else(|| {
                    panic!(
                        "fixture '{}' case '{}' is in the http domain but has no http_request",
                        fixture.name, case.name
                    )
                });
                let req = spec.headers.iter().fold(
                    rumi_http::HttpRequest::builder()
                        .method(&spec.method)
                        .path(&spec.path),
                    |b, (k, v)| b.header(k, v),
                );
                assert_eq!(
                    matcher.evaluate(&req.build()),
                    case.expect,
                    "fixture '{}' case '{}'",
                    fixture.name,
                    case.name
                );
            }
        }
    }
}

#[test]
fn protojson_conformance() {
    let dir = fixtures_dir().join("07_protojson");
    assert!(dir.exists(), "missing {}", dir.display());

    let mut count = 0;
    for entry in fs::read_dir(&dir).expect("read dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().is_none_or(|e| e != "yaml" && e != "yml") {
            continue;
        }
        let yaml = fs::read_to_string(&path).expect("read yaml");
        for fixture in ProtoFixture::from_yaml_multi(&yaml)
            .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
        {
            run(&fixture);
            count += 1;
        }
    }

    assert!(
        count > 0,
        "no protojson fixtures found — the runner is inert"
    );
    println!("protojson conformance: {count} fixtures");
}

/// SF9 — the same rule in YAML and in JSON builds the same matcher.
///
/// Both syntaxes are accepted (`rumi/cli/src/main.rs` sniffs on extension), and
/// D-026 makes that a guarantee rather than an accident, so it needs a test.
///
/// It holds by construction rather than by care: both front ends parse to a
/// `serde_json::Value` and meet at `parse_matcher`. This asserts the
/// construction has not been routed around — a second parser that reads YAML
/// straight into the proto would pass every other test in this file and fail
/// this one.
#[test]
fn yaml_and_json_build_the_same_matcher() {
    const YAML: &str = r#"
matcherList:
  matchers:
    - predicate:
        singlePredicate:
          input:
            name: role
            typedConfig:
              "@type": type.googleapis.com/xuma.kv.v1.MapInput
              key: role
          valueMatch:
            exact: admin
      onMatch:
        action:
          name: allow
          typedConfig:
            "@type": type.googleapis.com/xuma.core.v1.NamedAction
            name: allow
onNoMatch:
  action:
    name: deny
    typedConfig:
      "@type": type.googleapis.com/xuma.core.v1.NamedAction
      name: deny
"#;

    const JSON: &str = r#"{
      "matcherList": {
        "matchers": [{
          "predicate": {
            "singlePredicate": {
              "input": {
                "name": "role",
                "typedConfig": {
                  "@type": "type.googleapis.com/xuma.kv.v1.MapInput",
                  "key": "role"
                }
              },
              "valueMatch": { "exact": "admin" }
            }
          },
          "onMatch": {
            "action": {
              "name": "allow",
              "typedConfig": {
                "@type": "type.googleapis.com/xuma.core.v1.NamedAction",
                "name": "allow"
              }
            }
          }
        }]
      },
      "onNoMatch": {
        "action": {
          "name": "deny",
          "typedConfig": {
            "@type": "type.googleapis.com/xuma.core.v1.NamedAction",
            "name": "deny"
          }
        }
      }
    }"#;

    let from_yaml: serde_json::Value =
        serde_yaml::from_str(YAML).expect("the YAML form should parse");
    let from_json: serde_json::Value =
        serde_json::from_str(JSON).expect("the JSON form should parse");

    assert_eq!(
        from_yaml, from_json,
        "the two syntaxes must reach the same document"
    );

    // And the document must reach the same behaviour, not merely the same
    // shape — a matcher that never fires would satisfy the assert above.
    let build = |doc: serde_json::Value| {
        let fixture = ProtoFixture {
            name: "sf9".into(),
            description: String::new(),
            proto_matcher: doc,
            implementations: rumi_test::proto_fixture::ALL.to_vec(),
            domain: Domain::Kv,
            cases: Vec::new(),
            expect_error: false,
            error_contains: None,
        };
        build_matcher(&fixture).expect("should load")
    };

    let (a, b) = (build(from_yaml), build(from_json));
    for role in ["admin", "viewer", ""] {
        let ctx = KvContext::new().with("role", role);
        assert_eq!(
            a.evaluate(&ctx),
            b.evaluate(&ctx),
            "the two syntaxes disagree on role={role:?}"
        );
    }
    assert_eq!(
        a.evaluate(&KvContext::new().with("role", "admin")),
        Some("allow".to_string()),
        "and the rule must actually fire, or this test proves nothing"
    );
}
