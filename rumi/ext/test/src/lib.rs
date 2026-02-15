//! rumi-test: Test domain for conformance testing
//!
//! Provides simple context and `DataInput` implementations for testing matchers.
//! This is the reference extension that demonstrates how to build rumi extensions.
//!
//! # Example
//!
//! ```
//! use rumi_test::prelude::*;
//!
//! // TestContext is a simple key-value map
//! let ctx = TestContext::new()
//!     .with("name", "alice")
//!     .with("role", "admin");
//!
//! // StringInput extracts a value by key
//! let input = StringInput::new("role");
//! assert_eq!(input.get(&ctx), MatchingData::String("admin".into()));
//! ```

use rumi::prelude::*;
use std::collections::HashMap;

#[cfg(feature = "fixtures")]
pub mod config_fixture;
#[cfg(feature = "fixtures")]
pub mod fixture;

/// Test context: a simple string-to-string map.
///
/// Used for conformance testing where we need predictable,
/// controllable input data.
#[derive(Debug, Clone, Default)]
pub struct TestContext {
    values: HashMap<String, String>,
}

impl TestContext {
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

/// Extracts a string value from `TestContext` by key.
#[derive(Debug, Clone)]
pub struct StringInput {
    key: String,
}

impl StringInput {
    /// Create a new string input extractor.
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

impl DataInput<TestContext> for StringInput {
    fn get(&self, ctx: &TestContext) -> MatchingData {
        ctx.get(&self.key)
            .map_or(MatchingData::None, |s| MatchingData::String(s.to_string()))
    }
}

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::{StringInput, TestContext};
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
impl rumi::IntoDataInput<TestContext> for StringInput {
    type Config = StringInputConfig;

    fn from_config(
        config: Self::Config,
    ) -> Result<Box<dyn rumi::DataInput<TestContext>>, rumi::MatcherError> {
        Ok(Box::new(StringInput::new(config.key)))
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
    impl rumi::IntoDataInput<TestContext> for StringInput {
        type Config = proto::StringInput;

        fn from_config(
            config: proto::StringInput,
        ) -> Result<Box<dyn rumi::DataInput<TestContext>>, rumi::MatcherError> {
            Ok(Box::new(StringInput::new(config.value)))
        }
    }
}

/// Register all rumi-test types with the given builder.
///
/// Registers core matchers (`BoolMatcher`, `StringMatcher`) and test-domain inputs:
/// - `xuma.test.v1.StringInput` → [`StringInput`]
#[cfg(feature = "registry")]
#[must_use]
pub fn register(builder: rumi::RegistryBuilder<TestContext>) -> rumi::RegistryBuilder<TestContext> {
    rumi::register_core_matchers(builder).input::<StringInput>("xuma.test.v1.StringInput")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_builder() {
        let ctx = TestContext::new().with("foo", "bar").with("baz", "qux");

        assert_eq!(ctx.get("foo"), Some("bar"));
        assert_eq!(ctx.get("baz"), Some("qux"));
        assert_eq!(ctx.get("missing"), None);
    }

    #[test]
    fn test_string_input() {
        let ctx = TestContext::new().with("name", "alice");
        let input = StringInput::new("name");

        assert_eq!(input.get(&ctx), MatchingData::String("alice".into()));
    }

    #[test]
    fn test_string_input_missing_key() {
        let ctx = TestContext::new();
        let input = StringInput::new("missing");

        assert_eq!(input.get(&ctx), MatchingData::None);
    }

    #[test]
    fn test_full_matcher() {
        let ctx = TestContext::new().with("role", "admin");

        let matcher: Matcher<TestContext, &str> = Matcher::new(
            vec![FieldMatcher::new(
                Predicate::Single(SinglePredicate::new(
                    Box::new(StringInput::new("role")),
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

        let ctx = TestContext::new().with("role", "admin");
        assert_eq!(matcher.evaluate(&ctx), Some("allow".to_string()));

        let ctx = TestContext::new().with("role", "viewer");
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

        let ctx = TestContext::new()
            .with("role", "admin")
            .with("org", "acme-corp");
        assert_eq!(matcher.evaluate(&ctx), Some("admin_acme".to_string()));

        let ctx = TestContext::new()
            .with("role", "admin")
            .with("org", "other");
        assert_eq!(matcher.evaluate(&ctx), None);
    }
}
