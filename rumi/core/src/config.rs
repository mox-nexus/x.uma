//! Config types for generic matcher construction.
//!
//! These types mirror the runtime matcher types but are serde-deserializable,
//! enabling config-driven matcher construction via [`Registry::load_matcher()`].
//!
//! # Relationship to runtime types
//!
//! | Config type | Runtime type | Loader method |
//! |-------------|-------------|---------------|
//! | [`MatcherConfig`] | [`Matcher`](crate::Matcher) | `Registry::load_matcher()` |
//! | [`FieldMatcherConfig`] | [`FieldMatcher`](crate::FieldMatcher) | `Registry::load_field_matcher()` |
//! | [`PredicateConfig`] | [`Predicate`](crate::Predicate) | `Registry::load_predicate()` |
//! | [`SinglePredicateConfig`] | [`SinglePredicate`](crate::SinglePredicate) | `Registry::load_single()` |
//! | [`ValueMatchConfig`] | `Box<dyn InputMatcher>` | built-in or via registry factory |
//! | [`OnMatchConfig`] | [`OnMatch`](crate::OnMatch) | `Registry::load_on_match()` |
//! | [`TypedConfig`] | `Box<dyn DataInput<Ctx>>` or `Box<dyn InputMatcher>` | via registry factory |

use crate::StringMatchSpec;
use serde::Deserialize;

/// Configuration for a [`Matcher`](crate::Matcher).
///
/// Deserializes from JSON/YAML and can be loaded into a runtime `Matcher`
/// via [`Registry::load_matcher()`](crate::Registry::load_matcher).
#[derive(Debug, Clone, Deserialize)]
#[serde(bound(deserialize = "A: Deserialize<'de>"))]
pub struct MatcherConfig<A> {
    /// Field matchers to evaluate in order (first-match-wins).
    pub matchers: Vec<FieldMatcherConfig<A>>,

    /// Fallback when no field matcher matches.
    #[serde(default)]
    pub on_no_match: Option<OnMatchConfig<A>>,
}

/// Configuration for a [`FieldMatcher`](crate::FieldMatcher).
#[derive(Debug, Clone, Deserialize)]
#[serde(bound(deserialize = "A: Deserialize<'de>"))]
pub struct FieldMatcherConfig<A> {
    /// The predicate that gates this field matcher.
    pub predicate: PredicateConfig,

    /// What to do when the predicate matches.
    pub on_match: OnMatchConfig<A>,
}

/// Configuration for a [`Predicate`](crate::Predicate).
///
/// Uses `#[serde(tag = "type")]` for discriminated union deserialization:
///
/// ```json
/// { "type": "single", "input": { ... }, "value_match": { ... } }
/// { "type": "and", "predicates": [...] }
/// { "type": "or", "predicates": [...] }
/// { "type": "not", "predicate": { ... } }
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum PredicateConfig {
    /// A single predicate: input + value match.
    #[serde(rename = "single")]
    Single(SinglePredicateConfig),

    /// All predicates must match (logical AND).
    #[serde(rename = "and")]
    And {
        /// Child predicates (all must match).
        predicates: Vec<PredicateConfig>,
    },

    /// Any predicate must match (logical OR).
    #[serde(rename = "or")]
    Or {
        /// Child predicates (any must match).
        predicates: Vec<PredicateConfig>,
    },

    /// Inverts the inner predicate (logical NOT).
    #[serde(rename = "not")]
    Not {
        /// The predicate to negate.
        predicate: Box<PredicateConfig>,
    },
}

/// How to match the extracted value in a [`SinglePredicateConfig`].
///
/// Mirrors Envoy's `oneof matcher` in `SinglePredicate`:
/// - `BuiltIn` — built-in string matching (Envoy: `StringMatcher value_match`)
/// - `Custom` — custom matcher via registry (Envoy: `TypedExtensionConfig custom_match`)
///
/// The enum makes illegal states unrepresentable: exactly one variant is active.
#[derive(Debug, Clone)]
pub enum ValueMatchConfig {
    /// Built-in string matching (exact, prefix, suffix, contains, regex).
    BuiltIn(StringMatchSpec),
    /// Custom matcher resolved via the registry's matcher factories.
    Custom(TypedConfig),
}

/// Configuration for a [`SinglePredicate`](crate::SinglePredicate).
///
/// Combines a typed input reference (resolved via registry) with a
/// value matcher. The matcher is either a built-in `StringMatchSpec`
/// (via `value_match` JSON field) or a custom registered type
/// (via `custom_match` JSON field).
///
/// # JSON format
///
/// ```json
/// { "input": { "type_url": "..." }, "value_match": { "Exact": "hello" } }
/// { "input": { "type_url": "..." }, "custom_match": { "type_url": "...", "config": {...} } }
/// ```
///
/// Exactly one of `value_match` or `custom_match` must be set.
#[derive(Debug, Clone)]
pub struct SinglePredicateConfig {
    /// The input to extract data from context.
    /// Resolved at load time via the registry's `type_url` lookup.
    pub input: TypedConfig,

    /// How to match the extracted value.
    pub matcher: ValueMatchConfig,
}

impl<'de> Deserialize<'de> for SinglePredicateConfig {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Helper struct pattern: two Option fields for the oneof,
        // validated to exactly-one-set at deserialization time.
        #[derive(Deserialize)]
        struct Helper {
            input: TypedConfig,
            #[serde(default)]
            value_match: Option<StringMatchSpec>,
            #[serde(default)]
            custom_match: Option<TypedConfig>,
        }

        let helper = Helper::deserialize(deserializer)?;
        let matcher = match (helper.value_match, helper.custom_match) {
            (Some(spec), None) => ValueMatchConfig::BuiltIn(spec),
            (None, Some(tc)) => ValueMatchConfig::Custom(tc),
            (Some(_), Some(_)) => {
                return Err(serde::de::Error::custom(
                    "exactly one of `value_match` or `custom_match` must be set, got both",
                ))
            }
            (None, None) => {
                return Err(serde::de::Error::custom(
                    "one of `value_match` or `custom_match` is required",
                ))
            }
        };
        Ok(SinglePredicateConfig {
            input: helper.input,
            matcher,
        })
    }
}

/// Reference to a registered type with its configuration.
///
/// Maps to xDS `TypedExtensionConfig`:
/// - `type_url` identifies the registered type (input, matcher, or action)
/// - `config` carries the type-specific configuration payload
#[derive(Debug, Clone, Deserialize)]
pub struct TypedConfig {
    /// The type URL identifying the registered type.
    /// Must match a `type_url` registered in the [`Registry`](crate::Registry).
    pub type_url: String,

    /// Type-specific configuration payload.
    /// Deserialized as the `Config` associated type of the registered trait impl.
    #[serde(default = "default_config")]
    pub config: serde_json::Value,
}

fn default_config() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

/// Empty configuration for [`DataInput`](crate::DataInput) types that need no configuration.
///
/// Accepts any JSON value (`{}`, `null`, etc.) and ignores it.
/// Use as the `Config` associated type in [`IntoDataInput`](crate::IntoDataInput)
/// for inputs that are self-contained (no construction parameters).
#[derive(Debug, Clone, Copy)]
pub struct UnitConfig;

impl<'de> Deserialize<'de> for UnitConfig {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        serde::de::IgnoredAny::deserialize(deserializer)?;
        Ok(UnitConfig)
    }
}

/// Configuration for [`OnMatch`](crate::OnMatch).
///
/// Either an action (leaf) or a nested matcher (tree).
/// `OnMatch` exclusivity is enforced by the enum: action XOR matcher.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
#[serde(bound(deserialize = "A: Deserialize<'de>"))]
pub enum OnMatchConfig<A> {
    /// Return this action when the predicate matches.
    #[serde(rename = "action")]
    Action {
        /// The action value.
        action: A,
    },

    /// Evaluate a nested matcher when the predicate matches.
    #[serde(rename = "matcher")]
    Matcher {
        /// The nested matcher configuration.
        matcher: Box<MatcherConfig<A>>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_simple_config() {
        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": { "type_url": "test.Input", "config": { "key": "val" } },
                    "value_match": { "Exact": "hello" }
                },
                "on_match": { "type": "action", "action": "hit" }
            }],
            "on_no_match": { "type": "action", "action": "miss" }
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        assert_eq!(config.matchers.len(), 1);
        assert!(config.on_no_match.is_some());
    }

    #[test]
    fn deserialize_and_predicate() {
        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "and",
                    "predicates": [
                        {
                            "type": "single",
                            "input": { "type_url": "a" },
                            "value_match": { "Exact": "x" }
                        },
                        {
                            "type": "single",
                            "input": { "type_url": "b" },
                            "value_match": { "Prefix": "y" }
                        }
                    ]
                },
                "on_match": { "type": "action", "action": "ok" }
            }]
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        match &config.matchers[0].predicate {
            PredicateConfig::And { predicates } => assert_eq!(predicates.len(), 2),
            _ => panic!("expected And"),
        }
    }

    #[test]
    fn deserialize_not_predicate() {
        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "not",
                    "predicate": {
                        "type": "single",
                        "input": { "type_url": "a" },
                        "value_match": { "Exact": "x" }
                    }
                },
                "on_match": { "type": "action", "action": "ok" }
            }]
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        assert!(matches!(
            config.matchers[0].predicate,
            PredicateConfig::Not { .. }
        ));
    }

    #[test]
    fn deserialize_nested_matcher() {
        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": { "type_url": "a" },
                    "value_match": { "Prefix": "" }
                },
                "on_match": {
                    "type": "matcher",
                    "matcher": {
                        "matchers": [{
                            "predicate": {
                                "type": "single",
                                "input": { "type_url": "a" },
                                "value_match": { "Exact": "deep" }
                            },
                            "on_match": { "type": "action", "action": "nested" }
                        }]
                    }
                }
            }]
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        match &config.matchers[0].on_match {
            OnMatchConfig::Matcher { matcher } => assert_eq!(matcher.matchers.len(), 1),
            _ => panic!("expected nested matcher"),
        }
    }

    #[test]
    fn typed_config_defaults_to_empty_object() {
        let json = serde_json::json!({ "type_url": "test.Input" });
        let tc: TypedConfig = serde_json::from_value(json).unwrap();
        assert_eq!(tc.config, serde_json::json!({}));
    }

    #[test]
    fn no_on_no_match_is_none() {
        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": { "type_url": "a" },
                    "value_match": { "Exact": "x" }
                },
                "on_match": { "type": "action", "action": "ok" }
            }]
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        assert!(config.on_no_match.is_none());
    }
}
