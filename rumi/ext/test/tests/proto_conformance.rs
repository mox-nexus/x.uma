//! Conformance over fixtures written in canonical protojson.
//!
//! Run with:
//! `cargo test -p rumi-test --test proto_conformance --features rumi-test/registry,rumi-test/fixtures`

#![cfg(all(feature = "fixtures", feature = "registry"))]

use rumi_proto::any_resolver::{AnyResolver, AnyResolverBuilder};
use rumi_proto::convert::load_proto_matcher;
use rumi_proto::protojson::parse_matcher;
use rumi_test::proto_fixture::{Implementation, ProtoFixture};
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
fn build(fixture: &ProtoFixture) -> Result<rumi::Matcher<KvContext, String>, rumi::MatcherError> {
    let resolver = resolver();
    let registry = rumi_test::register(rumi::RegistryBuilder::new()).build();
    let actions = rumi::ActionRegistryBuilder::new()
        .action::<NamedActionFactory>("xuma.core.v1.NamedAction")
        .build();

    let proto = parse_matcher(&resolver, fixture.proto_matcher.clone())?;
    load_proto_matcher(&registry, &actions, &resolver, &proto)
}

fn run(fixture: &ProtoFixture) {
    // Not listed: this runner must NOT be able to run it. A skip that quietly
    // starts working is as much a defect in the ledger as one that quietly
    // starts failing — it means the list is reporting on work already done.
    if !fixture.expects(ME) {
        assert!(
            build(fixture).is_err(),
            "fixture '{}' does not list rust, but rust loads it. Add rust to \
             `implementations` — a stale exception hides a finished migration.",
            fixture.name
        );
        println!("  skipped (not listed): {}", fixture.name);
        return;
    }

    if fixture.expect_error {
        let err = build(fixture).err().unwrap_or_else(|| {
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

    let matcher =
        build(fixture).unwrap_or_else(|e| panic!("fixture '{}' failed to load: {e}", fixture.name));

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
