//! Converts proto `Matcher` → `MatcherConfig<TypedConfig>`.
//!
//! Walks the xDS proto matcher tree and produces the rumi config types that
//! can be loaded through [`Registry::load_typed_matcher()`](rumi::Registry::load_typed_matcher).
//!
//! # Proto → Config mapping
//!
//! | Proto type | Config type |
//! |-----------|-------------|
//! | `Matcher` | `MatcherConfig<TypedConfig>` |
//! | `MatcherList.FieldMatcher` | `FieldMatcherConfig<TypedConfig>` |
//! | `Predicate` | `PredicateConfig` |
//! | `SinglePredicate` | `SinglePredicateConfig` |
//! | `StringMatcher` | `ValueMatchConfig::BuiltIn(StringMatchSpec)` |
//! | `TypedExtensionConfig` (matcher) | `ValueMatchConfig::Custom(TypedConfig)` |
//! | `TypedExtensionConfig` (input) | `TypedConfig` (in `SinglePredicateConfig.input`) |
//! | `OnMatch::Action` | `OnMatchConfig::Action { action: TypedConfig }` |
//! | `OnMatch::Matcher` | `OnMatchConfig::Matcher { matcher: Box<...> }` |

use rumi::{
    ActionRegistry, FieldMatcherConfig, Matcher, MatcherConfig, MatcherError, OnMatchConfig,
    PredicateConfig, Registry, SinglePredicateConfig, StringMatchSpec, TypedConfig,
    ValueMatchConfig,
};

use crate::any_resolver::AnyResolver;
use crate::xds::r#type::matcher::v3 as proto_matcher;

/// Load a proto `Matcher` into a runtime `Matcher<Ctx, A>`.
///
/// This is the one-shot convenience function for the full proto → runtime path:
///
/// 1. **Convert** proto `Matcher` → `MatcherConfig<TypedConfig>` (via [`convert_matcher`])
/// 2. **Load** config through the [`Registry`] → runtime `Matcher<Ctx, A>`
///
/// # Example
///
/// ```ignore
/// let matcher = load_proto_matcher(&registry, &actions, &resolver, proto)?;
/// let result = matcher.evaluate(&ctx);
/// ```
///
/// # Errors
///
/// Returns [`MatcherError`] if the proto structure is invalid, an `Any` payload
/// can't be decoded, a type URL isn't registered in the `Registry` or
/// `ActionRegistry`, or validation fails (e.g., depth > 32).
pub fn load_proto_matcher<Ctx, A>(
    registry: &Registry<Ctx>,
    actions: &ActionRegistry<A>,
    resolver: &AnyResolver,
    proto: &proto_matcher::Matcher,
) -> Result<Matcher<Ctx, A>, MatcherError>
where
    Ctx: 'static,
    A: Clone + Send + Sync + 'static,
{
    let config = convert_matcher(proto, resolver)?;
    registry.load_typed_matcher(config, actions)
}

/// Convert a proto `Matcher` into a `MatcherConfig<TypedConfig>`.
///
/// The `AnyResolver` decodes `google.protobuf.Any` payloads in
/// `TypedExtensionConfig` messages into `serde_json::Value` for the registry.
///
/// # Supported matcher types
///
/// Currently supports `MatcherList` (linear first-match-wins). `MatcherTree`
/// support will be added when needed.
///
/// # Errors
///
/// - [`MatcherError::InvalidConfig`] for missing required fields
/// - [`MatcherError::UnknownTypeUrl`] if `AnyResolver` can't decode a type
pub fn convert_matcher(
    matcher: &proto_matcher::Matcher,
    resolver: &AnyResolver,
) -> Result<MatcherConfig<TypedConfig>, MatcherError> {
    let matchers = match &matcher.matcher_type {
        Some(proto_matcher::matcher::MatcherType::MatcherList(list)) => list
            .matchers
            .iter()
            .map(|fm| convert_field_matcher(fm, resolver))
            .collect::<Result<Vec<_>, _>>()?,
        Some(proto_matcher::matcher::MatcherType::MatcherTree(_)) => {
            return Err(MatcherError::InvalidConfig {
                source: "MatcherTree is not yet supported; use MatcherList".into(),
            });
        }
        None => {
            return Err(MatcherError::InvalidConfig {
                source: "Matcher has no matcher_type set".into(),
            });
        }
    };

    let on_no_match = matcher
        .on_no_match
        .as_deref()
        .map(|om| convert_on_match(om, resolver))
        .transpose()?;

    Ok(MatcherConfig {
        matchers,
        on_no_match,
    })
}

fn convert_field_matcher(
    fm: &proto_matcher::matcher::matcher_list::FieldMatcher,
    resolver: &AnyResolver,
) -> Result<FieldMatcherConfig<TypedConfig>, MatcherError> {
    let predicate = fm
        .predicate
        .as_ref()
        .ok_or_else(|| MatcherError::InvalidConfig {
            source: "FieldMatcher has no predicate".into(),
        })?;

    let on_match = fm
        .on_match
        .as_ref()
        .ok_or_else(|| MatcherError::InvalidConfig {
            source: "FieldMatcher has no on_match".into(),
        })?;

    Ok(FieldMatcherConfig {
        predicate: convert_predicate(predicate, resolver)?,
        on_match: convert_on_match(on_match, resolver)?,
    })
}

fn convert_predicate(
    pred: &proto_matcher::matcher::matcher_list::Predicate,
    resolver: &AnyResolver,
) -> Result<PredicateConfig, MatcherError> {
    use proto_matcher::matcher::matcher_list::predicate::MatchType;

    let match_type = pred
        .match_type
        .as_ref()
        .ok_or_else(|| MatcherError::InvalidConfig {
            source: "Predicate has no match_type".into(),
        })?;

    match match_type {
        MatchType::SinglePredicate(sp) => {
            let single = convert_single_predicate(sp, resolver)?;
            Ok(PredicateConfig::Single(single))
        }
        MatchType::OrMatcher(list) => {
            let predicates = list
                .predicate
                .iter()
                .map(|p| convert_predicate(p, resolver))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(PredicateConfig::Or { predicates })
        }
        MatchType::AndMatcher(list) => {
            let predicates = list
                .predicate
                .iter()
                .map(|p| convert_predicate(p, resolver))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(PredicateConfig::And { predicates })
        }
        MatchType::NotMatcher(inner) => {
            let predicate = convert_predicate(inner, resolver)?;
            Ok(PredicateConfig::Not {
                predicate: Box::new(predicate),
            })
        }
    }
}

fn convert_single_predicate(
    sp: &proto_matcher::matcher::matcher_list::predicate::SinglePredicate,
    resolver: &AnyResolver,
) -> Result<SinglePredicateConfig, MatcherError> {
    // Resolve input TypedExtensionConfig → TypedConfig
    let input_ext = sp
        .input
        .as_ref()
        .ok_or_else(|| MatcherError::InvalidConfig {
            source: "SinglePredicate has no input".into(),
        })?;
    let input = resolver.resolve(input_ext)?;

    // Resolve matcher: value_match (StringMatcher) or custom_match (TypedExtensionConfig)
    use proto_matcher::matcher::matcher_list::predicate::single_predicate::Matcher as ProtoMatcher;

    let matcher_oneof = sp
        .matcher
        .as_ref()
        .ok_or_else(|| MatcherError::InvalidConfig {
            // Names the fields, so the error says what to write rather than
            // what is absent — and so it agrees with puma's wording, which is
            // a conformance property the fixtures pin.
            source: "singlePredicate: one of 'valueMatch' or 'customMatch' is required".into(),
        })?;

    let matcher = match matcher_oneof {
        ProtoMatcher::ValueMatch(string_matcher) => {
            let spec = convert_string_matcher(string_matcher)?;
            ValueMatchConfig::BuiltIn {
                spec,
                ignore_case: string_matcher.ignore_case,
            }
        }
        ProtoMatcher::CustomMatch(ext) => {
            let typed = resolver.resolve(ext)?;
            ValueMatchConfig::Custom(typed)
        }
    };

    Ok(SinglePredicateConfig { input, matcher })
}

fn convert_string_matcher(
    sm: &proto_matcher::StringMatcher,
) -> Result<StringMatchSpec, MatcherError> {
    use proto_matcher::string_matcher::MatchPattern;

    let pattern = sm
        .match_pattern
        .as_ref()
        .ok_or_else(|| MatcherError::InvalidConfig {
            source: "StringMatcher has no match_pattern".into(),
        })?;

    // `ignore_case` is not read here: it belongs to the comparison, not the
    // pattern, so the caller carries it on ValueMatchConfig::BuiltIn. A comment
    // in this spot used to claim the registry handled it. It did not, and the
    // flag was dropped — a rule that read case-insensitive and was not.

    match pattern {
        MatchPattern::Exact(s) => Ok(StringMatchSpec::Exact(s.clone())),
        MatchPattern::Prefix(s) => Ok(StringMatchSpec::Prefix(s.clone())),
        MatchPattern::Suffix(s) => Ok(StringMatchSpec::Suffix(s.clone())),
        MatchPattern::Contains(s) => Ok(StringMatchSpec::Contains(s.clone())),
        MatchPattern::SafeRegex(re) => Ok(StringMatchSpec::Regex(re.regex.clone())),
        MatchPattern::Custom(_) => Err(MatcherError::InvalidConfig {
            source: "Custom StringMatcher extensions not yet supported".into(),
        }),
    }
}

fn convert_on_match(
    om: &proto_matcher::matcher::OnMatch,
    resolver: &AnyResolver,
) -> Result<OnMatchConfig<TypedConfig>, MatcherError> {
    use proto_matcher::matcher::on_match::OnMatch as ProtoOnMatch;

    // `keep_matching` is one keystroke from being set and, until 2026-08-18,
    // did nothing. In xDS it records the action and continues evaluating, which
    // is a different answer from first-match-wins — so accepting it and
    // ignoring it means a config that reads one way behaves another. It is
    // deferred, not implemented (CLAUDE.md), so it is refused rather than
    // silently reinterpreted. Rejecting is reversible; a wrong answer is not.
    if om.keep_matching {
        return Err(MatcherError::InvalidConfig {
            source: "keep_matching is not implemented. In xDS it records the action and \
                     continues evaluating; this engine returns the first match. Accepting \
                     the field would give a different answer than the config asks for, so \
                     it is refused. Remove it, or restructure the rule."
                .into(),
        });
    }

    let on_match = om
        .on_match
        .as_ref()
        .ok_or_else(|| MatcherError::InvalidConfig {
            source: "OnMatch has no on_match variant set".into(),
        })?;

    match on_match {
        ProtoOnMatch::Action(ext) => {
            let typed = resolver.resolve(ext)?;
            Ok(OnMatchConfig::Action { action: typed })
        }
        ProtoOnMatch::Matcher(nested) => {
            let config = convert_matcher(nested, resolver)?;
            Ok(OnMatchConfig::Matcher {
                matcher: Box::new(config),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::any_resolver::AnyResolverBuilder;
    use crate::xds::core::v3::TypedExtensionConfig;
    use prost::Message;

    fn make_any<T: Message>(type_url: &str, msg: &T) -> prost_types::Any {
        prost_types::Any {
            type_url: type_url.into(),
            value: msg.encode_to_vec().into(),
        }
    }

    fn make_ext<T: Message>(name: &str, type_url: &str, msg: &T) -> TypedExtensionConfig {
        TypedExtensionConfig {
            name: name.into(),
            typed_config: Some(make_any(type_url, msg)),
        }
    }

    fn test_resolver() -> AnyResolver {
        AnyResolverBuilder::new()
            .register::<crate::xuma::kv::v1::MapInput>("xuma.kv.v1.MapInput")
            .register::<crate::xuma::core::v1::NamedAction>("xuma.core.v1.NamedAction")
            .build()
    }

    #[test]
    fn convert_simple_exact_match() {
        let resolver = test_resolver();

        // Build proto: StringInput → Exact("hello") → NamedAction("hit")
        let input_config = crate::xuma::kv::v1::MapInput { key: "role".into() };
        let action_config = crate::xuma::core::v1::NamedAction {
            metadata: Default::default(),
            name: "allow".into(),
        };

        let proto = proto_matcher::Matcher {
            on_no_match: None,
            matcher_type: Some(proto_matcher::matcher::MatcherType::MatcherList(
                proto_matcher::matcher::MatcherList {
                    matchers: vec![proto_matcher::matcher::matcher_list::FieldMatcher {
                        predicate: Some(proto_matcher::matcher::matcher_list::Predicate {
                            match_type: Some(
                                proto_matcher::matcher::matcher_list::predicate::MatchType::SinglePredicate(
                                    proto_matcher::matcher::matcher_list::predicate::SinglePredicate {
                                        input: Some(make_ext("input", "xuma.kv.v1.MapInput", &input_config)),
                                        matcher: Some(
                                            proto_matcher::matcher::matcher_list::predicate::single_predicate::Matcher::ValueMatch(
                                                proto_matcher::StringMatcher {
                                                    ignore_case: false,
                                                    match_pattern: Some(
                                                        proto_matcher::string_matcher::MatchPattern::Exact("admin".into()),
                                                    ),
                                                },
                                            ),
                                        ),
                                    },
                                ),
                            ),
                        }),
                        on_match: Some(proto_matcher::matcher::OnMatch {
                            keep_matching: false,
                            on_match: Some(
                                proto_matcher::matcher::on_match::OnMatch::Action(
                                    make_ext("action", "xuma.core.v1.NamedAction", &action_config),
                                ),
                            ),
                        }),
                    }],
                },
            )),
        };

        let config = convert_matcher(&proto, &resolver).unwrap();
        assert_eq!(config.matchers.len(), 1);
        assert!(config.on_no_match.is_none());

        // Check the predicate has our input
        match &config.matchers[0].predicate {
            PredicateConfig::Single(sp) => {
                assert_eq!(sp.input.type_url, "xuma.kv.v1.MapInput");
                assert_eq!(sp.input.config["key"], "role");
            }
            other => panic!("expected Single, got {other:?}"),
        }

        // Check the action
        match &config.matchers[0].on_match {
            OnMatchConfig::Action { action } => {
                assert_eq!(action.type_url, "xuma.core.v1.NamedAction");
                assert_eq!(action.config["name"], "allow");
            }
            other => panic!("expected Action, got {other:?}"),
        }
    }

    #[test]
    fn convert_with_on_no_match() {
        let resolver = test_resolver();

        let input_config = crate::xuma::kv::v1::MapInput { key: "key".into() };
        let hit_action = crate::xuma::core::v1::NamedAction {
            metadata: Default::default(),
            name: "hit".into(),
        };
        let miss_action = crate::xuma::core::v1::NamedAction {
            metadata: Default::default(),
            name: "miss".into(),
        };

        let proto = proto_matcher::Matcher {
            on_no_match: Some(Box::new(proto_matcher::matcher::OnMatch {
                keep_matching: false,
                on_match: Some(proto_matcher::matcher::on_match::OnMatch::Action(
                    make_ext("fallback", "xuma.core.v1.NamedAction", &miss_action),
                )),
            })),
            matcher_type: Some(proto_matcher::matcher::MatcherType::MatcherList(
                proto_matcher::matcher::MatcherList {
                    matchers: vec![proto_matcher::matcher::matcher_list::FieldMatcher {
                        predicate: Some(proto_matcher::matcher::matcher_list::Predicate {
                            match_type: Some(
                                proto_matcher::matcher::matcher_list::predicate::MatchType::SinglePredicate(
                                    proto_matcher::matcher::matcher_list::predicate::SinglePredicate {
                                        input: Some(make_ext("input", "xuma.kv.v1.MapInput", &input_config)),
                                        matcher: Some(
                                            proto_matcher::matcher::matcher_list::predicate::single_predicate::Matcher::ValueMatch(
                                                proto_matcher::StringMatcher {
                                                    ignore_case: false,
                                                    match_pattern: Some(
                                                        proto_matcher::string_matcher::MatchPattern::Prefix("admin".into()),
                                                    ),
                                                },
                                            ),
                                        ),
                                    },
                                ),
                            ),
                        }),
                        on_match: Some(proto_matcher::matcher::OnMatch {
                            keep_matching: false,
                            on_match: Some(proto_matcher::matcher::on_match::OnMatch::Action(
                                make_ext("action", "xuma.core.v1.NamedAction", &hit_action),
                            )),
                        }),
                    }],
                },
            )),
        };

        let config = convert_matcher(&proto, &resolver).unwrap();
        assert!(config.on_no_match.is_some());
        match config.on_no_match.unwrap() {
            OnMatchConfig::Action { action } => {
                assert_eq!(action.config["name"], "miss");
            }
            other => panic!("expected Action, got {other:?}"),
        }
    }

    #[test]
    fn convert_and_predicate() {
        let resolver = test_resolver();

        let input1 = crate::xuma::kv::v1::MapInput { key: "role".into() };
        let input2 = crate::xuma::kv::v1::MapInput { key: "org".into() };
        let action = crate::xuma::core::v1::NamedAction {
            metadata: Default::default(),
            name: "both".into(),
        };

        let make_single = |input: &crate::xuma::kv::v1::MapInput, pattern: &str| {
            proto_matcher::matcher::matcher_list::Predicate {
                match_type: Some(
                    proto_matcher::matcher::matcher_list::predicate::MatchType::SinglePredicate(
                        proto_matcher::matcher::matcher_list::predicate::SinglePredicate {
                            input: Some(make_ext("in", "xuma.kv.v1.MapInput", input)),
                            matcher: Some(
                                proto_matcher::matcher::matcher_list::predicate::single_predicate::Matcher::ValueMatch(
                                    proto_matcher::StringMatcher {
                                        ignore_case: false,
                                        match_pattern: Some(
                                            proto_matcher::string_matcher::MatchPattern::Exact(pattern.into()),
                                        ),
                                    },
                                ),
                            ),
                        },
                    ),
                ),
            }
        };

        let proto = proto_matcher::Matcher {
            on_no_match: None,
            matcher_type: Some(proto_matcher::matcher::MatcherType::MatcherList(
                proto_matcher::matcher::MatcherList {
                    matchers: vec![proto_matcher::matcher::matcher_list::FieldMatcher {
                        predicate: Some(proto_matcher::matcher::matcher_list::Predicate {
                            match_type: Some(
                                proto_matcher::matcher::matcher_list::predicate::MatchType::AndMatcher(
                                    proto_matcher::matcher::matcher_list::predicate::PredicateList {
                                        predicate: vec![
                                            make_single(&input1, "admin"),
                                            make_single(&input2, "acme"),
                                        ],
                                    },
                                ),
                            ),
                        }),
                        on_match: Some(proto_matcher::matcher::OnMatch {
                            keep_matching: false,
                            on_match: Some(proto_matcher::matcher::on_match::OnMatch::Action(
                                make_ext("act", "xuma.core.v1.NamedAction", &action),
                            )),
                        }),
                    }],
                },
            )),
        };

        let config = convert_matcher(&proto, &resolver).unwrap();
        match &config.matchers[0].predicate {
            PredicateConfig::And { predicates } => assert_eq!(predicates.len(), 2),
            other => panic!("expected And, got {other:?}"),
        }
    }

    #[test]
    fn convert_missing_matcher_type_errors() {
        let resolver = test_resolver();

        let proto = proto_matcher::Matcher {
            on_no_match: None,
            matcher_type: None,
        };

        let err = convert_matcher(&proto, &resolver).unwrap_err();
        assert!(matches!(err, MatcherError::InvalidConfig { .. }));
    }

    // ═══════════════════════════════════════════════════════════════════
    // End-to-end: proto → convert → load → evaluate
    // ═══════════════════════════════════════════════════════════════════

    /// NamedAction → String: extracts the `name` field as the action value.
    struct NamedActionFactory;

    impl rumi::IntoAction<String> for NamedActionFactory {
        type Config = crate::xuma::core::v1::NamedAction;

        fn from_config(config: Self::Config) -> Result<String, MatcherError> {
            Ok(config.name)
        }
    }

    fn test_action_registry() -> rumi::ActionRegistry<String> {
        rumi::ActionRegistryBuilder::new()
            .action::<NamedActionFactory>("xuma.core.v1.NamedAction")
            .build()
    }

    /// `ignoreCase` survives the whole proto path.
    ///
    /// It used to be read from the proto and discarded in
    /// `convert_string_matcher`, under a comment claiming the registry handled
    /// it. The registry never saw it. A config that said `ignoreCase: true`
    /// matched case-sensitively, and nothing reported that.
    ///
    /// The conformance suite cannot express this yet — the terse `config:`
    /// dialect has no `ignore_case` field — so this is where SF1 is pinned
    /// until the fixtures move to protojson.
    #[test]
    fn e2e_ignore_case_survives_the_proto_path() {
        let registry = rumi_kv::register(rumi::RegistryBuilder::new()).build();
        let actions = test_action_registry();
        let resolver = test_resolver();

        let build = |ignore_case: bool| {
            let input_config = crate::xuma::kv::v1::MapInput { key: "role".into() };
            let action_config = crate::xuma::core::v1::NamedAction {
                metadata: Default::default(),
                name: "allow".into(),
            };
            let proto = proto_matcher::Matcher {
                on_no_match: None,
                matcher_type: Some(proto_matcher::matcher::MatcherType::MatcherList(
                    proto_matcher::matcher::MatcherList {
                        matchers: vec![proto_matcher::matcher::matcher_list::FieldMatcher {
                            predicate: Some(proto_matcher::matcher::matcher_list::Predicate {
                                match_type: Some(
                                    proto_matcher::matcher::matcher_list::predicate::MatchType::SinglePredicate(
                                        proto_matcher::matcher::matcher_list::predicate::SinglePredicate {
                                            input: Some(make_ext(
                                                "input",
                                                "xuma.kv.v1.MapInput",
                                                &input_config,
                                            )),
                                            matcher: Some(
                                                proto_matcher::matcher::matcher_list::predicate::single_predicate::Matcher::ValueMatch(
                                                    proto_matcher::StringMatcher {
                                                        ignore_case,
                                                        match_pattern: Some(
                                                            proto_matcher::string_matcher::MatchPattern::Exact(
                                                                "admin".into(),
                                                            ),
                                                        ),
                                                    },
                                                ),
                                            ),
                                        },
                                    ),
                                ),
                            }),
                            on_match: Some(proto_matcher::matcher::OnMatch {
                                keep_matching: false,
                                on_match: Some(
                                    proto_matcher::matcher::on_match::OnMatch::Action(make_ext(
                                        "action",
                                        "xuma.core.v1.NamedAction",
                                        &action_config,
                                    )),
                                ),
                            }),
                        }],
                    },
                )),
            };
            load_proto_matcher::<rumi_kv::KvContext, String>(&registry, &actions, &resolver, &proto)
                .unwrap()
        };

        let shouting = rumi_kv::KvContext::new().with("role", "ADMIN");

        assert_eq!(
            build(true).evaluate(&shouting),
            Some("allow".to_string()),
            "ignoreCase: true must match ADMIN"
        );
        assert_eq!(
            build(false).evaluate(&shouting),
            None,
            "without the flag it must stay case-sensitive"
        );
    }

    #[test]
    fn e2e_proto_to_evaluate_exact_match() {
        // Setup: registry with test domain, action registry, resolver
        let registry = rumi_kv::register(rumi::RegistryBuilder::new()).build();
        let actions = test_action_registry();
        let resolver = test_resolver();

        // Build proto: StringInput("role") → Exact("admin") → NamedAction("allow")
        let input_config = crate::xuma::kv::v1::MapInput { key: "role".into() };
        let action_config = crate::xuma::core::v1::NamedAction {
            metadata: Default::default(),
            name: "allow".into(),
        };

        let proto = proto_matcher::Matcher {
            on_no_match: None,
            matcher_type: Some(proto_matcher::matcher::MatcherType::MatcherList(
                proto_matcher::matcher::MatcherList {
                    matchers: vec![proto_matcher::matcher::matcher_list::FieldMatcher {
                        predicate: Some(proto_matcher::matcher::matcher_list::Predicate {
                            match_type: Some(
                                proto_matcher::matcher::matcher_list::predicate::MatchType::SinglePredicate(
                                    proto_matcher::matcher::matcher_list::predicate::SinglePredicate {
                                        input: Some(make_ext("input", "xuma.kv.v1.MapInput", &input_config)),
                                        matcher: Some(
                                            proto_matcher::matcher::matcher_list::predicate::single_predicate::Matcher::ValueMatch(
                                                proto_matcher::StringMatcher {
                                                    ignore_case: false,
                                                    match_pattern: Some(
                                                        proto_matcher::string_matcher::MatchPattern::Exact("admin".into()),
                                                    ),
                                                },
                                            ),
                                        ),
                                    },
                                ),
                            ),
                        }),
                        on_match: Some(proto_matcher::matcher::OnMatch {
                            keep_matching: false,
                            on_match: Some(proto_matcher::matcher::on_match::OnMatch::Action(
                                make_ext("action", "xuma.core.v1.NamedAction", &action_config),
                            )),
                        }),
                    }],
                },
            )),
        };

        // Full pipeline: proto → convert → load → evaluate
        let matcher = load_proto_matcher(&registry, &actions, &resolver, &proto).unwrap();

        let ctx = rumi_kv::KvContext::new().with("role", "admin");
        assert_eq!(matcher.evaluate(&ctx), Some("allow".to_string()));

        let ctx = rumi_kv::KvContext::new().with("role", "viewer");
        assert_eq!(matcher.evaluate(&ctx), None);
    }

    #[test]
    fn e2e_proto_with_on_no_match_fallback() {
        let registry = rumi_kv::register(rumi::RegistryBuilder::new()).build();
        let actions = test_action_registry();
        let resolver = test_resolver();

        let input_config = crate::xuma::kv::v1::MapInput { key: "role".into() };
        let hit_action = crate::xuma::core::v1::NamedAction {
            metadata: Default::default(),
            name: "allow".into(),
        };
        let miss_action = crate::xuma::core::v1::NamedAction {
            metadata: Default::default(),
            name: "deny".into(),
        };

        let proto = proto_matcher::Matcher {
            on_no_match: Some(Box::new(proto_matcher::matcher::OnMatch {
                keep_matching: false,
                on_match: Some(proto_matcher::matcher::on_match::OnMatch::Action(
                    make_ext("fallback", "xuma.core.v1.NamedAction", &miss_action),
                )),
            })),
            matcher_type: Some(proto_matcher::matcher::MatcherType::MatcherList(
                proto_matcher::matcher::MatcherList {
                    matchers: vec![proto_matcher::matcher::matcher_list::FieldMatcher {
                        predicate: Some(proto_matcher::matcher::matcher_list::Predicate {
                            match_type: Some(
                                proto_matcher::matcher::matcher_list::predicate::MatchType::SinglePredicate(
                                    proto_matcher::matcher::matcher_list::predicate::SinglePredicate {
                                        input: Some(make_ext("input", "xuma.kv.v1.MapInput", &input_config)),
                                        matcher: Some(
                                            proto_matcher::matcher::matcher_list::predicate::single_predicate::Matcher::ValueMatch(
                                                proto_matcher::StringMatcher {
                                                    ignore_case: false,
                                                    match_pattern: Some(
                                                        proto_matcher::string_matcher::MatchPattern::Exact("admin".into()),
                                                    ),
                                                },
                                            ),
                                        ),
                                    },
                                ),
                            ),
                        }),
                        on_match: Some(proto_matcher::matcher::OnMatch {
                            keep_matching: false,
                            on_match: Some(proto_matcher::matcher::on_match::OnMatch::Action(
                                make_ext("action", "xuma.core.v1.NamedAction", &hit_action),
                            )),
                        }),
                    }],
                },
            )),
        };

        let matcher = load_proto_matcher(&registry, &actions, &resolver, &proto).unwrap();

        // Match: role=admin → "allow"
        let ctx = rumi_kv::KvContext::new().with("role", "admin");
        assert_eq!(matcher.evaluate(&ctx), Some("allow".to_string()));

        // No match: role=viewer → fallback "deny"
        let ctx = rumi_kv::KvContext::new().with("role", "viewer");
        assert_eq!(matcher.evaluate(&ctx), Some("deny".to_string()));
    }

    #[test]
    fn e2e_proto_and_predicate() {
        let registry = rumi_kv::register(rumi::RegistryBuilder::new()).build();
        let actions = test_action_registry();
        let resolver = test_resolver();

        let input_role = crate::xuma::kv::v1::MapInput { key: "role".into() };
        let input_org = crate::xuma::kv::v1::MapInput { key: "org".into() };
        let action = crate::xuma::core::v1::NamedAction {
            metadata: Default::default(),
            name: "admin_acme".into(),
        };

        let make_single = |input: &crate::xuma::kv::v1::MapInput, pattern: &str| {
            proto_matcher::matcher::matcher_list::Predicate {
                match_type: Some(
                    proto_matcher::matcher::matcher_list::predicate::MatchType::SinglePredicate(
                        proto_matcher::matcher::matcher_list::predicate::SinglePredicate {
                            input: Some(make_ext("in", "xuma.kv.v1.MapInput", input)),
                            matcher: Some(
                                proto_matcher::matcher::matcher_list::predicate::single_predicate::Matcher::ValueMatch(
                                    proto_matcher::StringMatcher {
                                        ignore_case: false,
                                        match_pattern: Some(
                                            proto_matcher::string_matcher::MatchPattern::Exact(pattern.into()),
                                        ),
                                    },
                                ),
                            ),
                        },
                    ),
                ),
            }
        };

        let proto = proto_matcher::Matcher {
            on_no_match: None,
            matcher_type: Some(proto_matcher::matcher::MatcherType::MatcherList(
                proto_matcher::matcher::MatcherList {
                    matchers: vec![proto_matcher::matcher::matcher_list::FieldMatcher {
                        predicate: Some(proto_matcher::matcher::matcher_list::Predicate {
                            match_type: Some(
                                proto_matcher::matcher::matcher_list::predicate::MatchType::AndMatcher(
                                    proto_matcher::matcher::matcher_list::predicate::PredicateList {
                                        predicate: vec![
                                            make_single(&input_role, "admin"),
                                            make_single(&input_org, "acme"),
                                        ],
                                    },
                                ),
                            ),
                        }),
                        on_match: Some(proto_matcher::matcher::OnMatch {
                            keep_matching: false,
                            on_match: Some(proto_matcher::matcher::on_match::OnMatch::Action(
                                make_ext("act", "xuma.core.v1.NamedAction", &action),
                            )),
                        }),
                    }],
                },
            )),
        };

        let matcher = load_proto_matcher(&registry, &actions, &resolver, &proto).unwrap();

        // Both match → action
        let ctx = rumi_kv::KvContext::new()
            .with("role", "admin")
            .with("org", "acme");
        assert_eq!(matcher.evaluate(&ctx), Some("admin_acme".to_string()));

        // Only one matches → None
        let ctx = rumi_kv::KvContext::new()
            .with("role", "admin")
            .with("org", "other");
        assert_eq!(matcher.evaluate(&ctx), None);

        // Neither matches → None
        let ctx = rumi_kv::KvContext::new()
            .with("role", "viewer")
            .with("org", "other");
        assert_eq!(matcher.evaluate(&ctx), None);
    }
}

#[cfg(test)]
mod keep_matching_tests {
    use super::*;
    use crate::any_resolver::AnyResolverBuilder;
    use crate::protojson::parse_matcher_str;

    fn resolver() -> AnyResolver {
        AnyResolverBuilder::new()
            .register::<crate::xuma::kv::v1::MapInput>("xuma.kv.v1.MapInput")
            .register::<crate::xuma::core::v1::NamedAction>("xuma.core.v1.NamedAction")
            .build()
    }

    fn doc(keep_matching: &str) -> String {
        format!(
            r#"{{
              "matcherList": {{
                "matchers": [{{
                  "predicate": {{
                    "singlePredicate": {{
                      "input": {{
                        "name": "role",
                        "typedConfig": {{
                          "@type": "type.googleapis.com/xuma.kv.v1.MapInput",
                          "key": "role"
                        }}
                      }},
                      "valueMatch": {{ "exact": "admin" }}
                    }}
                  }},
                  "onMatch": {{
                    {keep_matching}
                    "action": {{
                      "name": "allow",
                      "typedConfig": {{
                        "@type": "type.googleapis.com/xuma.core.v1.NamedAction",
                        "name": "allow"
                      }}
                    }}
                  }}
                }}]
              }}
            }}"#
        )
    }

    /// Refused, not ignored. xDS `keep_matching` records the action and keeps
    /// evaluating; this engine returns the first match. Accepting the field
    /// silently would answer a different question than the config asked.
    #[test]
    fn keep_matching_is_refused_rather_than_ignored() {
        let r = resolver();
        let proto = parse_matcher_str(&r, &doc(r#""keepMatching": true,"#)).unwrap();
        let err = convert_matcher(&proto, &r).unwrap_err();
        match err {
            MatcherError::InvalidConfig { ref source } => {
                assert!(source.contains("keep_matching"), "{source}");
            }
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    /// The refusal must be about the flag being *set*, not about the field
    /// existing — an explicit `false` is the same config as omitting it.
    #[test]
    fn keep_matching_false_and_absent_both_load() {
        let r = resolver();
        for variant in ["", r#""keepMatching": false,"#] {
            let proto = parse_matcher_str(&r, &doc(variant)).unwrap();
            assert!(convert_matcher(&proto, &r).is_ok(), "variant {variant:?}");
        }
    }
}
