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
// ═══════════════════════════════════════════════════════════════════════════════

/// Configuration for [`StringInput`].
#[cfg(feature = "registry")]
#[derive(serde::Deserialize)]
pub struct StringInputConfig {
    /// The key to extract from the test context.
    pub key: String,
}

#[cfg(feature = "registry")]
impl rumi::IntoDataInput<TestContext> for StringInput {
    type Config = StringInputConfig;

    fn from_config(
        config: Self::Config,
    ) -> Result<Box<dyn rumi::DataInput<TestContext>>, rumi::MatcherError> {
        Ok(Box::new(StringInput::new(config.key)))
    }
}

/// Register all rumi-test `DataInput` types with the given builder.
///
/// Type URLs:
/// - `xuma.test.v1.StringInput` → [`StringInput`]
#[cfg(feature = "registry")]
#[must_use]
pub fn register(builder: rumi::RegistryBuilder<TestContext>) -> rumi::RegistryBuilder<TestContext> {
    builder.input::<StringInput>("xuma.test.v1.StringInput")
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
