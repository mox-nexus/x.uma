//! rumi-kv: the key-value matching domain.
//!
//! Match on a `String -> String` map. It is the simplest domain rumi has, a
//! peer of `rumi-http`, and the one the `rumi` CLI uses by default.
//!
//! It lived inside `rumi-test` until 2026-08-17, next to the conformance
//! suite's YAML fixture loader. That crate is `publish = false` — a fixture
//! loader has no business in a published artifact — which meant `rumi-cli`
//! could not be published either, despite only ever wanting this half. The
//! concept was never "test"; the misleading name is what hid the blocker.
//!
//! # Type URLs, and why they still say `test`
//!
//! This crate registers `xuma.test.v1.StringInput`, not `xuma.kv.v1.*`. Type
//! URLs are part of the config schema and freeze at first publish, so renaming
//! them is a schema decision that belongs with the protojson migration, not
//! with a crate split. Doing it here would break all 27 conformance fixtures
//! for no gain. See `PLAN.md` SF8.
//!
//! # Example
//!
//! ```
//! use rumi_kv::prelude::*;
//!
//! let ctx = KvContext::new()
//!     .with("name", "alice")
//!     .with("role", "admin");
//!
//! let input = StringInput::new("role").unwrap();
//! assert_eq!(input.get(&ctx), MatchingData::String("admin".into()));
//! ```

use rumi::prelude::*;
use std::collections::HashMap;

/// A key-value context: a simple string-to-string map.
///
/// `KvContext` is kept as a deprecated alias, since the conformance suite and
/// the crusts refer to it by that name.
#[derive(Debug, Clone, Default)]
pub struct KvContext {
    values: HashMap<String, String>,
}

impl KvContext {
    /// Create an empty test context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a key-value pair (builder pattern).
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }

    /// Get a value by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }
}

/// Extracts a string value from `KvContext` by key.
#[derive(Debug, Clone)]
pub struct StringInput {
    key: String,
}

impl StringInput {
    /// Create a new string input extractor.
    ///
    /// # Errors
    ///
    /// [`MatcherError::EmptyIdentifier`](rumi::MatcherError::EmptyIdentifier)
    /// if `key` is empty. The key names which entry to read; an empty one reads
    /// nothing, so the predicate is always false.
    pub fn new(key: impl Into<String>) -> Result<Self, rumi::MatcherError> {
        let key = key.into();
        if key.is_empty() {
            return Err(rumi::MatcherError::EmptyIdentifier { what: "map key" });
        }
        Ok(Self { key })
    }
}

impl DataInput<KvContext> for StringInput {
    fn get(&self, ctx: &KvContext) -> MatchingData {
        ctx.get(&self.key)
            .map_or(MatchingData::None, |s| MatchingData::String(s.to_string()))
    }
}

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::{KvContext, StringInput};
    pub use rumi::prelude::*;
}

// ═══════════════════════════════════════════════════════════════════════════════
// Registry support (feature = "registry")
// Hand-written config types — used when proto feature is not enabled.
// ═══════════════════════════════════════════════════════════════════════════════

/// Configuration for [`StringInput`].
#[cfg(all(feature = "registry", not(feature = "proto")))]
#[derive(serde::Deserialize)]
pub struct StringInputConfig {
    /// The key to extract from the test context.
    pub key: String,
}

#[cfg(all(feature = "registry", not(feature = "proto")))]
impl rumi::IntoDataInput<KvContext> for StringInput {
    type Config = StringInputConfig;

    fn from_config(
        config: Self::Config,
    ) -> Result<Box<dyn rumi::DataInput<KvContext>>, rumi::MatcherError> {
        Ok(Box::new(StringInput::new(config.key)?))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Proto config types (feature = "proto")
// Uses proto-generated types as Config, enabling xDS control plane integration.
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "proto")]
mod proto_configs {
    use super::*;
    use rumi_proto::xuma::test::v1 as proto;

    /// Proto `StringInput.value` maps to domain `StringInput.key`.
    /// The proto field is named "value" (the extracted string value),
    /// while the domain field is named "key" (the lookup key in test context).
    impl rumi::IntoDataInput<KvContext> for StringInput {
        type Config = proto::StringInput;

        fn from_config(
            config: proto::StringInput,
        ) -> Result<Box<dyn rumi::DataInput<KvContext>>, rumi::MatcherError> {
            Ok(Box::new(StringInput::new(config.value)?))
        }
    }
}

/// Register all rumi-test types with the given builder.
///
/// Registers core matchers (`BoolMatcher`, `StringMatcher`) and test-domain inputs:
/// - `xuma.test.v1.StringInput` → [`StringInput`]
#[cfg(feature = "registry")]
#[must_use]
pub fn register(builder: rumi::RegistryBuilder<KvContext>) -> rumi::RegistryBuilder<KvContext> {
    rumi::register_core_matchers(builder).input::<StringInput>("xuma.test.v1.StringInput")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_key_is_rejected() {
        let err = StringInput::new("").unwrap_err();
        assert!(
            matches!(err, rumi::MatcherError::EmptyIdentifier { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn test_context_builder() {
        let ctx = KvContext::new().with("foo", "bar").with("baz", "qux");

        assert_eq!(ctx.get("foo"), Some("bar"));
        assert_eq!(ctx.get("baz"), Some("qux"));
        assert_eq!(ctx.get("missing"), None);
    }

    #[test]
    fn test_string_input() {
        let ctx = KvContext::new().with("name", "alice");
        let input = StringInput::new("name").unwrap();

        assert_eq!(input.get(&ctx), MatchingData::String("alice".into()));
    }

    #[test]
    fn test_string_input_missing_key() {
        let ctx = KvContext::new();
        let input = StringInput::new("missing").unwrap();

        assert_eq!(input.get(&ctx), MatchingData::None);
    }

    #[test]
    fn test_full_matcher() {
        let ctx = KvContext::new().with("role", "admin");

        let matcher: Matcher<KvContext, &str> = Matcher::new(
            vec![FieldMatcher::new(
                Predicate::Single(SinglePredicate::new(
                    Box::new(StringInput::new("role").unwrap()),
                    Box::new(ExactMatcher::new("admin")),
                )),
                OnMatch::Action("allowed"),
            )],
            Some(OnMatch::Action("denied")),
        );

        assert_eq!(matcher.evaluate(&ctx), Some("allowed"));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Proto registry integration tests
// Verifies the full pipeline: proto config → registry → DataInput → evaluate
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(all(test, feature = "proto"))]
mod proto_tests {
    use super::*;
    use rumi::MatcherConfig;

    #[test]
    fn register_builds_with_proto_configs() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        // Core matchers + 1 test input
        assert!(registry.contains_input("xuma.test.v1.StringInput"));
        assert!(registry.contains_matcher("xuma.core.v1.StringMatcher"));
        assert!(registry.contains_matcher("xuma.core.v1.BoolMatcher"));
    }

    #[test]
    fn load_matcher_with_proto_string_input() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        // Proto StringInput config uses "value" field (maps to lookup key)
        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.test.v1.StringInput",
                        "config": { "value": "role" }
                    },
                    "value_match": { "Exact": "admin" }
                },
                "on_match": { "type": "action", "action": "allow" }
            }],
            "on_no_match": { "type": "action", "action": "deny" }
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        let matcher = registry.load_matcher(config).unwrap();

        let ctx = KvContext::new().with("role", "admin");
        assert_eq!(matcher.evaluate(&ctx), Some("allow".to_string()));

        let ctx = KvContext::new().with("role", "viewer");
        assert_eq!(matcher.evaluate(&ctx), Some("deny".to_string()));
    }

    #[test]
    fn load_matcher_with_and_predicate() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "and",
                    "predicates": [
                        {
                            "type": "single",
                            "input": {
                                "type_url": "xuma.test.v1.StringInput",
                                "config": { "value": "role" }
                            },
                            "value_match": { "Exact": "admin" }
                        },
                        {
                            "type": "single",
                            "input": {
                                "type_url": "xuma.test.v1.StringInput",
                                "config": { "value": "org" }
                            },
                            "value_match": { "Prefix": "acme" }
                        }
                    ]
                },
                "on_match": { "type": "action", "action": "admin_acme" }
            }]
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        let matcher = registry.load_matcher(config).unwrap();

        let ctx = KvContext::new()
            .with("role", "admin")
            .with("org", "acme-corp");
        assert_eq!(matcher.evaluate(&ctx), Some("admin_acme".to_string()));

        let ctx = KvContext::new().with("role", "admin").with("org", "other");
        assert_eq!(matcher.evaluate(&ctx), None);
    }
}
