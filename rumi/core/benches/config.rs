//! Config-path benchmarks — protojson → proto → Registry → Matcher.
//!
//! Measures the cost of the config loading layer against the manual
//! construction path, to isolate what a config-driven caller pays.
//!
//! These configs are canonical protojson, and the benchmark walks the whole
//! shipping path: `parse_matcher` (which expands `@type` bodies) then
//! `load_proto_matcher`. Until 2026-08-18 it instead measured a terser JSON
//! dialect deserialized straight into `MatcherConfig`, which was always
//! cheaper than what any caller actually ran and, once that dialect was
//! retired, measured nothing that existed.

use rumi::prelude::*;
use rumi_kv::KvContext;
use rumi_proto::any_resolver::{AnyResolver, AnyResolverBuilder};
use rumi_proto::convert::load_proto_matcher;
use rumi_proto::protojson::parse_matcher_str;

fn main() {
    divan::main();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Shared JSON configs (identical across all implementations)
// ═══════════════════════════════════════════════════════════════════════════════

const SIMPLE_CONFIG: &str = r#"{
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
          "name": "matched",
          "typedConfig": {
            "@type": "type.googleapis.com/xuma.core.v1.NamedAction",
            "name": "matched"
          }
        }
      }
    }]
  },
  "onNoMatch": {
    "action": {
      "name": "default",
      "typedConfig": {
        "@type": "type.googleapis.com/xuma.core.v1.NamedAction",
        "name": "default"
      }
    }
  }
}"#;

const COMPOUND_CONFIG: &str = r#"{
  "matcherList": {
    "matchers": [{
      "predicate": {
        "andMatcher": {
          "predicate": [
            {
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
            {
              "singlePredicate": {
                "input": {
                  "name": "org",
                  "typedConfig": {
                    "@type": "type.googleapis.com/xuma.kv.v1.MapInput",
                    "key": "org"
                  }
                },
                "valueMatch": { "prefix": "acme" }
              }
            }
          ]
        }
      },
      "onMatch": {
        "action": {
          "name": "admin_acme",
          "typedConfig": {
            "@type": "type.googleapis.com/xuma.core.v1.NamedAction",
            "name": "admin_acme"
          }
        }
      }
    }]
  }
}"#;

const NESTED_CONFIG: &str = r#"{
  "matcherList": {
    "matchers": [{
      "predicate": {
        "singlePredicate": {
          "input": {
            "name": "tier",
            "typedConfig": {
              "@type": "type.googleapis.com/xuma.kv.v1.MapInput",
              "key": "tier"
            }
          },
          "valueMatch": { "exact": "premium" }
        }
      },
      "onMatch": {
        "matcher": {
          "matcherList": {
            "matchers": [{
              "predicate": {
                "singlePredicate": {
                  "input": {
                    "name": "region",
                    "typedConfig": {
                      "@type": "type.googleapis.com/xuma.kv.v1.MapInput",
                      "key": "region"
                    }
                  },
                  "valueMatch": { "exact": "us" }
                }
              },
              "onMatch": {
                "action": {
                  "name": "premium_us",
                  "typedConfig": {
                    "@type": "type.googleapis.com/xuma.core.v1.NamedAction",
                    "name": "premium_us"
                  }
                }
              }
            }]
          },
          "onNoMatch": {
            "action": {
              "name": "premium_other",
              "typedConfig": {
                "@type": "type.googleapis.com/xuma.core.v1.NamedAction",
                "name": "premium_other"
              }
            }
          }
        }
      }
    }]
  },
  "onNoMatch": {
    "action": {
      "name": "default",
      "typedConfig": {
        "@type": "type.googleapis.com/xuma.core.v1.NamedAction",
        "name": "default"
      }
    }
  }
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

fn build_registry() -> rumi::Registry<KvContext> {
    rumi_test::register(rumi::RegistryBuilder::new()).build()
}

fn resolver() -> AnyResolver {
    AnyResolverBuilder::new()
        .register::<rumi_proto::xuma::kv::v1::MapInput>("xuma.kv.v1.MapInput")
        .register::<rumi_proto::xuma::core::v1::NamedAction>("xuma.core.v1.NamedAction")
        .build()
}

/// `NamedAction` -> `String`, mirroring what the conformance runner registers.
struct NamedActionFactory;

impl rumi::IntoAction<String> for NamedActionFactory {
    type Config = rumi_proto::xuma::core::v1::NamedAction;

    fn from_config(config: Self::Config) -> Result<String, rumi::MatcherError> {
        if config.name.is_empty() {
            return Err(rumi::MatcherError::EmptyIdentifier {
                what: "action name",
            });
        }
        Ok(config.name)
    }
}

fn actions() -> rumi::ActionRegistry<String> {
    rumi::ActionRegistryBuilder::new()
        .action::<NamedActionFactory>("xuma.core.v1.NamedAction")
        .build()
}

/// The whole shipping config path: protojson text -> evaluable `Matcher`.
fn load(
    registry: &rumi::Registry<KvContext>,
    resolver: &AnyResolver,
    actions: &rumi::ActionRegistry<String>,
    json: &str,
) -> rumi::Matcher<KvContext, String> {
    let proto = parse_matcher_str(resolver, json).unwrap();
    load_proto_matcher(registry, actions, resolver, &proto).unwrap()
}

#[divan::bench]
fn config_load_simple(bencher: divan::Bencher) {
    let registry = build_registry();
    let resolver = resolver();
    let actions = actions();
    bencher.bench_local(|| load(&registry, &resolver, &actions, SIMPLE_CONFIG));
}

#[divan::bench]
fn config_load_compound(bencher: divan::Bencher) {
    let registry = build_registry();
    let resolver = resolver();
    let actions = actions();
    bencher.bench_local(|| load(&registry, &resolver, &actions, COMPOUND_CONFIG));
}

#[divan::bench]
fn config_load_nested(bencher: divan::Bencher) {
    let registry = build_registry();
    let resolver = resolver();
    let actions = actions();
    bencher.bench_local(|| load(&registry, &resolver, &actions, NESTED_CONFIG));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Evaluation parity: config-loaded matcher vs compiler-built matcher
// ═══════════════════════════════════════════════════════════════════════════════

#[divan::bench]
fn config_evaluate_simple(bencher: divan::Bencher) {
    let registry = build_registry();
    let matcher = load(&registry, &resolver(), &actions(), SIMPLE_CONFIG);
    let ctx = KvContext::new().with("role", "admin");

    bencher.bench_local(|| matcher.evaluate(&ctx));
}

#[divan::bench]
fn compiler_evaluate_simple(bencher: divan::Bencher) {
    let matcher = Matcher::new(
        vec![FieldMatcher::new(
            Predicate::Single(SinglePredicate::new(
                Box::new(rumi_kv::StringInput::new("role").unwrap()),
                Box::new(ExactMatcher::new("admin")),
            )),
            OnMatch::Action("matched".to_string()),
        )],
        Some(OnMatch::Action("default".to_string())),
    );
    let ctx = KvContext::new().with("role", "admin");

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
    let resolver = resolver();
    let actions = actions();
    bencher.bench_local(|| load(&registry, &resolver, &actions, SIMPLE_CONFIG));
}

#[divan::bench]
fn compiler_construct_simple(bencher: divan::Bencher) {
    bencher.bench_local(|| {
        Matcher::<KvContext, String>::new(
            vec![FieldMatcher::new(
                Predicate::Single(SinglePredicate::new(
                    Box::new(rumi_kv::StringInput::new("role").unwrap()),
                    Box::new(ExactMatcher::new("admin")),
                )),
                OnMatch::Action("matched".to_string()),
            )],
            Some(OnMatch::Action("default".to_string())),
        )
    });
}
