//! rumi-kv: the key-value matching domain.
//!
//! Match on a `String -> String` map. It is the simplest domain rumi has, a
//! peer of `rumi-http`, and the one the `rumi` CLI uses by default.
//!
//! It is a crate of its own, not part of `rumi-test`, because that crate is
//! `publish = false` — a fixture loader has no business in a published
//! artifact — and `rumi-cli` needs only this half.
//!
//! # Type URL
//!
//! This crate registers `xuma.core.v1.MapInput`: its proto field names the
//! lookup key, and there is one schema whatever features are enabled.
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
//! let input = MapInput::new("role").unwrap();
//! assert_eq!(input.get(&ctx), Some(MatchingData::String("admin".into())));
//! ```

use rumi::prelude::*;
use std::borrow::Cow;
use std::collections::HashMap;

/// A key-value context: a simple string-to-string map.
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

/// The type a [`MapInput`] reads its value as (D-069).
///
/// The names are the ones [`DataInput::data_type`] speaks, so a matcher that
/// cannot read the declared type is refused when the document loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValueType {
    /// The text as it is.
    #[default]
    String,
    /// A decimal `i64`.
    Int,
    /// `true` or `false`.
    Bool,
}

impl ValueType {
    /// Parse a declared type name; empty is `string`, proto3's default.
    ///
    /// # Errors
    ///
    /// [`MatcherError::InvalidConfig`](rumi::MatcherError::InvalidConfig) for
    /// any other name.
    pub fn from_name(name: &str) -> Result<Self, rumi::MatcherError> {
        match name {
            "" | "string" => Ok(Self::String),
            "int" => Ok(Self::Int),
            "bool" => Ok(Self::Bool),
            other => Err(rumi::MatcherError::InvalidConfig {
                source: format!("type \"{other}\" is not one of \"string\", \"int\", \"bool\""),
            }),
        }
    }

    /// The name [`DataInput::data_type`] reports.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Int => "int",
            Self::Bool => "bool",
        }
    }

    /// The value `text` reads as, or `None` if it does not parse, which makes
    /// it absent.
    fn read(self, text: &str) -> Option<MatchingData> {
        match self {
            Self::String => Some(MatchingData::String(text.to_owned())),
            Self::Int => text.parse().ok().map(MatchingData::Int),
            Self::Bool => match text {
                "true" => Some(MatchingData::Bool(true)),
                "false" => Some(MatchingData::Bool(false)),
                _ => None,
            },
        }
    }
}

/// Reads one entry of a [`KvContext`] as its declared [`ValueType`].
#[derive(Debug, Clone)]
pub struct MapInput {
    key: String,
    value_type: ValueType,
}

impl MapInput {
    /// Read the entry under `key` as a string.
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
        Ok(Self {
            key,
            value_type: ValueType::String,
        })
    }

    /// Read the entry as `value_type` instead.
    #[must_use]
    pub fn with_type(mut self, value_type: ValueType) -> Self {
        self.value_type = value_type;
        self
    }
}

impl DataInput<KvContext> for MapInput {
    fn get(&self, ctx: &KvContext) -> Option<MatchingData> {
        ctx.get(&self.key)
            .and_then(|text| self.value_type.read(text))
    }

    fn data_type(&self) -> &'static str {
        self.value_type.name()
    }

    fn identity(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Owned(format!(
            "xuma.core.v1.MapInput/{}/{}",
            self.value_type.name(),
            self.key
        )))
    }
}

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::{KvContext, MapInput, ValueType};
    pub use rumi::prelude::*;
}

// ═══════════════════════════════════════════════════════════════════════════════
// Config loading (feature = "registry")
//
// The config type is the generated proto message, unconditionally: a
// hand-written config selected by a feature once made the same crate read a
// different key depending on the build. Features add; they do not replace.
// `required` is read by the registry, not here (D-069).
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "registry")]
impl rumi::DataInputFactory<KvContext> for MapInput {
    type Config = rumi_proto::xuma::core::v1::MapInput;

    fn from_config(
        config: Self::Config,
    ) -> Result<Box<dyn rumi::DataInput<KvContext>>, rumi::MatcherError> {
        let value_type = ValueType::from_name(&config.r#type)?;
        Ok(Box::new(MapInput::new(config.key)?.with_type(value_type)))
    }
}

/// Register the key-value domain with the given builder.
///
/// Registers the core matchers and `xuma.core.v1.MapInput` → [`MapInput`].
#[cfg(feature = "registry")]
#[must_use]
pub fn register(builder: rumi::RegistryBuilder<KvContext>) -> rumi::RegistryBuilder<KvContext> {
    rumi::register_core_matchers(builder).input::<MapInput>("xuma.core.v1.MapInput")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_empty_key_is_rejected() {
        let err = MapInput::new("").unwrap_err();
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
        let input = MapInput::new("name").unwrap();

        assert_eq!(input.get(&ctx), Some(MatchingData::String("alice".into())));
    }

    #[test]
    fn a_typed_value_that_does_not_parse_is_absent() {
        let ctx = KvContext::new().with("port", "80").with("flag", "yes");
        let port = MapInput::new("port").unwrap().with_type(ValueType::Int);
        let flag = MapInput::new("flag").unwrap().with_type(ValueType::Bool);

        assert_eq!(port.get(&ctx), Some(MatchingData::Int(80)));
        assert_eq!(flag.get(&ctx), None);
        assert_ne!(
            port.identity(),
            MapInput::new("port").unwrap().identity(),
            "one key read as two types is two inputs"
        );
    }

    #[test]
    fn test_string_input_missing_key() {
        let ctx = KvContext::new();
        let input = MapInput::new("missing").unwrap();

        assert_eq!(input.get(&ctx), None);
    }

    #[test]
    fn test_full_matcher() {
        let ctx = KvContext::new().with("role", "admin");

        let matcher: Matcher<KvContext, &str> = Matcher::list(
            vec![FieldMatcher::new(
                Predicate::Single(SinglePredicate::new(
                    Box::new(MapInput::new("role").unwrap()),
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
// Config-path integration tests
// Verifies the full pipeline: config → registry → DataInput → evaluate
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(all(test, feature = "registry"))]
mod config_tests {
    use super::*;

    use rumi::{
        FieldMatcherIr, OnMatchIr, PredicateIr, SinglePredicateIr, StringMatchSpec,
        TypedExtensionConfig, ValueMatchIr,
    };

    // `MatcherIr` and friends are built, never parsed — see rumi's
    // `config` module. These keep the tests below readable without deciding
    // anything on their behalf.

    fn built_in(key: &str, spec: StringMatchSpec) -> PredicateIr {
        PredicateIr::Single(SinglePredicateIr {
            input: TypedExtensionConfig {
                type_url: "xuma.core.v1.MapInput".into(),
                config: serde_json::json!({ "key": key }),
            },
            matcher: ValueMatchIr::BuiltIn {
                spec,
                ignore_case: false,
            },
        })
    }

    fn act(action: &str) -> OnMatchIr<String> {
        OnMatchIr::Action {
            action: action.into(),
        }
    }

    fn field(predicate: PredicateIr, on_match: OnMatchIr<String>) -> FieldMatcherIr<String> {
        FieldMatcherIr {
            predicate,
            on_match,
        }
    }
    use rumi::MatcherIr;

    #[test]
    fn register_builds_with_proto_configs() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        // Core matchers + 1 test input
        assert!(registry.contains_input("xuma.core.v1.MapInput"));
        assert!(registry.contains_matcher("xuma.core.v1.BoolMatcher"));
    }

    #[test]
    fn load_matcher_with_map_input() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        let config = MatcherIr::list(vec![field(
            built_in("role", StringMatchSpec::Exact("admin".into())),
            act("allow"),
        )])
        .with_fallback(Some(act("deny")));
        let matcher = registry.load_matcher(config).unwrap();

        let ctx = KvContext::new().with("role", "admin");
        assert_eq!(matcher.evaluate(&ctx), Some("allow".to_string()));

        let ctx = KvContext::new().with("role", "viewer");
        assert_eq!(matcher.evaluate(&ctx), Some("deny".to_string()));
    }

    #[test]
    fn load_matcher_with_and_predicate() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        let config = MatcherIr::list(vec![field(
            PredicateIr::And {
                predicates: vec![
                    built_in("role", StringMatchSpec::Exact("admin".into())),
                    built_in("org", StringMatchSpec::Prefix("acme".into())),
                ],
            },
            act("admin_acme"),
        )]);
        let matcher = registry.load_matcher(config).unwrap();

        let ctx = KvContext::new()
            .with("role", "admin")
            .with("org", "acme-corp");
        assert_eq!(matcher.evaluate(&ctx), Some("admin_acme".to_string()));

        let ctx = KvContext::new().with("role", "admin").with("org", "other");
        assert_eq!(matcher.evaluate(&ctx), None);
    }
}

/// The crate README's Rust blocks, compiled and run as doctests: the README is the
/// crates.io front page, so its examples must load.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;
