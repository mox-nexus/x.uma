//! Type registry for config-driven matcher construction.
//!
//! The registry enables **generic config loading**: JSON/YAML/proto config → compiled
//! `Matcher<Ctx, A>` without domain-specific compile code.
//!
//! # Architecture (axum `BoxedIntoRoute` pattern)
//!
//! Each `DataInput` type registers itself via [`IntoDataInput`]. At registration time,
//! the concrete type `T` is monomorphized into a closure and erased behind `Box<dyn Fn>`.
//! This is the same pattern axum uses for route handlers — early type erasure at
//! registration, late invocation at request time.
//!
//! # Example
//!
//! ```ignore
//! let registry = RegistryBuilder::new()
//!     .input::<PathInput>("xuma.http.v1.PathInput")
//!     .input::<HeaderInput>("xuma.http.v1.HeaderInput")
//!     .build();
//!
//! let config: MatcherConfig<String> = serde_json::from_str(json)?;
//! let matcher = registry.load_matcher(config)?;
//! ```

use std::collections::HashMap;
use std::marker::PhantomData;

use serde::de::DeserializeOwned;

use crate::{
    config::{
        FieldMatcherConfig, MatcherConfig, OnMatchConfig, PredicateConfig, SinglePredicateConfig,
    },
    DataInput, FieldMatcher, Matcher, MatcherError, OnMatch, Predicate, SinglePredicate,
};

/// Trait for `DataInput` types that can be constructed from configuration.
///
/// Each `DataInput` type knows its own config shape via the associated `Config` type.
/// The registry calls [`from_config`](Self::from_config) to construct the input at load time.
///
/// # Design (tower double dispatch)
///
/// `DataInput<Ctx>` describes *what* to extract. `IntoDataInput<Ctx>` describes *how* to
/// construct from config. The registry composes both: look up the `type_url` → deserialize
/// config → construct input → pair with matcher.
pub trait IntoDataInput<Ctx: 'static>: Send + Sync + 'static {
    /// The configuration type deserialized from JSON/YAML/proto.
    type Config: DeserializeOwned + Send + Sync;

    /// Construct a `DataInput` from deserialized configuration.
    ///
    /// # Errors
    ///
    /// Returns [`MatcherError::InvalidConfig`] if the config is semantically invalid
    /// (e.g., empty header name).
    fn from_config(config: Self::Config) -> Result<Box<dyn DataInput<Ctx>>, MatcherError>;
}

/// Type-erased factory closure.
///
/// Takes a `serde_json::Value` (the deserialized config payload) and produces a
/// `Box<dyn DataInput<Ctx>>`. The concrete type `T` is captured in the closure
/// at registration time via monomorphization.
type BoxedFactory<Ctx> =
    Box<dyn Fn(&serde_json::Value) -> Result<Box<dyn DataInput<Ctx>>, MatcherError> + Send + Sync>;

/// Builder for constructing a [`Registry`].
///
/// Register `DataInput` types with their type URLs, then call [`build()`](Self::build)
/// to produce an immutable `Registry`.
///
/// # Arch-guild constraint: immutability after build
///
/// The builder pattern enforces the guild constraint that the registry must be
/// immutable after initialization. No runtime registration is possible.
pub struct RegistryBuilder<Ctx> {
    factories: HashMap<String, BoxedFactory<Ctx>>,
    _phantom: PhantomData<Ctx>,
}

impl<Ctx: 'static> RegistryBuilder<Ctx> {
    /// Create a new empty registry builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Register a `DataInput` type with a type URL.
    ///
    /// The concrete type `T` is monomorphized here and erased behind a closure.
    /// At load time, the registry deserializes config as `T::Config` and calls
    /// `T::from_config()` to produce the `DataInput`.
    #[must_use]
    pub fn input<T: IntoDataInput<Ctx>>(mut self, type_url: &str) -> Self {
        self.factories.insert(
            type_url.to_owned(),
            Box::new(|value: &serde_json::Value| {
                let config: T::Config = serde_json::from_value(value.clone()).map_err(|e| {
                    MatcherError::InvalidConfig {
                        source: e.to_string(),
                    }
                })?;
                T::from_config(config)
            }),
        );
        self
    }

    /// Freeze the registry. No further registration is possible.
    #[must_use]
    pub fn build(self) -> Registry<Ctx> {
        Registry {
            factories: self.factories,
            _phantom: PhantomData,
        }
    }
}

impl<Ctx: 'static> Default for RegistryBuilder<Ctx> {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable registry of `DataInput` factories.
///
/// Constructed via [`RegistryBuilder`]. Use [`load_matcher()`](Self::load_matcher)
/// to compile config into a runtime `Matcher`.
pub struct Registry<Ctx> {
    factories: HashMap<String, BoxedFactory<Ctx>>,
    _phantom: PhantomData<Ctx>,
}

impl<Ctx: 'static> Registry<Ctx> {
    /// Load a `Matcher` from configuration.
    ///
    /// Walks the config tree, constructs `DataInput`s via registered factories,
    /// builds predicates and field matchers, and validates depth constraints.
    ///
    /// # Errors
    ///
    /// - [`MatcherError::UnknownTypeUrl`] — input `type_url` not registered
    /// - [`MatcherError::InvalidConfig`] — config deserialization or construction failed
    /// - [`MatcherError::InvalidPattern`] — regex pattern is invalid
    /// - [`MatcherError::DepthExceeded`] — matcher nesting exceeds [`MAX_DEPTH`](crate::MAX_DEPTH)
    pub fn load_matcher<A>(&self, config: MatcherConfig<A>) -> Result<Matcher<Ctx, A>, MatcherError>
    where
        A: Clone + Send + Sync + 'static,
    {
        let matchers = config
            .matchers
            .into_iter()
            .map(|fm| self.load_field_matcher(fm))
            .collect::<Result<Vec<_>, _>>()?;
        let on_no_match = config
            .on_no_match
            .map(|om| self.load_on_match(om))
            .transpose()?;
        let matcher = Matcher::new(matchers, on_no_match);
        matcher.validate()?;
        Ok(matcher)
    }

    /// Returns the number of registered input types.
    #[must_use]
    pub fn len(&self) -> usize {
        self.factories.len()
    }

    /// Returns `true` if no input types are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }

    /// Returns `true` if the given type URL is registered.
    #[must_use]
    pub fn contains(&self, type_url: &str) -> bool {
        self.factories.contains_key(type_url)
    }

    fn load_field_matcher<A>(
        &self,
        config: FieldMatcherConfig<A>,
    ) -> Result<FieldMatcher<Ctx, A>, MatcherError>
    where
        A: Clone + Send + Sync + 'static,
    {
        let predicate = self.load_predicate(config.predicate)?;
        let on_match = self.load_on_match(config.on_match)?;
        Ok(FieldMatcher::new(predicate, on_match))
    }

    fn load_predicate(&self, config: PredicateConfig) -> Result<Predicate<Ctx>, MatcherError> {
        match config {
            PredicateConfig::Single(single) => {
                let sp = self.load_single(single)?;
                Ok(Predicate::Single(sp))
            }
            PredicateConfig::And { predicates } => {
                let ps = predicates
                    .into_iter()
                    .map(|p| self.load_predicate(p))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Predicate::And(ps))
            }
            PredicateConfig::Or { predicates } => {
                let ps = predicates
                    .into_iter()
                    .map(|p| self.load_predicate(p))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Predicate::Or(ps))
            }
            PredicateConfig::Not { predicate } => {
                let inner = self.load_predicate(*predicate)?;
                Ok(Predicate::Not(Box::new(inner)))
            }
        }
    }

    #[allow(clippy::needless_pass_by_value)] // Consistent with other load_ methods
    fn load_single(
        &self,
        config: SinglePredicateConfig,
    ) -> Result<SinglePredicate<Ctx>, MatcherError> {
        let factory = self.factories.get(&config.input.type_url).ok_or_else(|| {
            MatcherError::UnknownTypeUrl {
                type_url: config.input.type_url.clone(),
            }
        })?;
        let input = factory(&config.input.config)?;
        let matcher = config.value_match.to_input_matcher()?;
        Ok(SinglePredicate::new(input, matcher))
    }

    fn load_on_match<A>(&self, config: OnMatchConfig<A>) -> Result<OnMatch<Ctx, A>, MatcherError>
    where
        A: Clone + Send + Sync + 'static,
    {
        match config {
            OnMatchConfig::Action { action } => Ok(OnMatch::Action(action)),
            OnMatchConfig::Matcher { matcher } => {
                let m = self.load_matcher(*matcher)?;
                Ok(OnMatch::Matcher(Box::new(m)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MatchingData;

    #[derive(Debug)]
    struct TestCtx {
        value: String,
    }

    #[derive(Debug)]
    struct ValueInput {
        key: String,
    }

    impl DataInput<TestCtx> for ValueInput {
        fn get(&self, ctx: &TestCtx) -> MatchingData {
            if self.key == "value" {
                MatchingData::String(ctx.value.clone())
            } else {
                MatchingData::None
            }
        }
    }

    #[derive(serde::Deserialize)]
    struct ValueInputConfig {
        key: String,
    }

    impl IntoDataInput<TestCtx> for ValueInput {
        type Config = ValueInputConfig;

        fn from_config(config: Self::Config) -> Result<Box<dyn DataInput<TestCtx>>, MatcherError> {
            Ok(Box::new(ValueInput { key: config.key }))
        }
    }

    #[test]
    fn builder_registers_and_freezes() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test.ValueInput"));
        assert!(!registry.contains("test.Unknown"));
    }

    #[test]
    fn load_simple_matcher() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "test.ValueInput",
                        "config": { "key": "value" }
                    },
                    "value_match": { "Exact": "hello" }
                },
                "on_match": {
                    "type": "action",
                    "action": "matched!"
                }
            }],
            "on_no_match": {
                "type": "action",
                "action": "default"
            }
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        let matcher = registry.load_matcher(config).unwrap();

        let ctx = TestCtx {
            value: "hello".into(),
        };
        assert_eq!(matcher.evaluate(&ctx), Some("matched!".to_string()));

        let ctx = TestCtx {
            value: "world".into(),
        };
        assert_eq!(matcher.evaluate(&ctx), Some("default".to_string()));
    }

    #[test]
    fn unknown_type_url_returns_error() {
        let registry = RegistryBuilder::<TestCtx>::new().build();

        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "unknown.Input",
                        "config": {}
                    },
                    "value_match": { "Exact": "x" }
                },
                "on_match": { "type": "action", "action": "x" }
            }]
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        let err = registry.load_matcher(config).unwrap_err();
        assert!(matches!(err, MatcherError::UnknownTypeUrl { .. }));
    }

    #[test]
    fn invalid_config_returns_error() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "test.ValueInput",
                        "config": { "wrong_field": 42 }
                    },
                    "value_match": { "Exact": "x" }
                },
                "on_match": { "type": "action", "action": "x" }
            }]
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        let err = registry.load_matcher(config).unwrap_err();
        assert!(matches!(err, MatcherError::InvalidConfig { .. }));
    }

    #[test]
    fn load_and_predicate() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "and",
                    "predicates": [
                        {
                            "type": "single",
                            "input": { "type_url": "test.ValueInput", "config": { "key": "value" } },
                            "value_match": { "Prefix": "hel" }
                        },
                        {
                            "type": "single",
                            "input": { "type_url": "test.ValueInput", "config": { "key": "value" } },
                            "value_match": { "Suffix": "llo" }
                        }
                    ]
                },
                "on_match": { "type": "action", "action": "both_matched" }
            }]
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        let matcher = registry.load_matcher(config).unwrap();

        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "hello".into()
            }),
            Some("both_matched".to_string())
        );
        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "help".into()
            }),
            None
        );
    }

    #[test]
    fn load_nested_matcher() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": { "type_url": "test.ValueInput", "config": { "key": "value" } },
                    "value_match": { "Prefix": "" }
                },
                "on_match": {
                    "type": "matcher",
                    "matcher": {
                        "matchers": [{
                            "predicate": {
                                "type": "single",
                                "input": { "type_url": "test.ValueInput", "config": { "key": "value" } },
                                "value_match": { "Exact": "deep" }
                            },
                            "on_match": { "type": "action", "action": "nested_hit" }
                        }]
                    }
                }
            }],
            "on_no_match": { "type": "action", "action": "fallback" }
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        let matcher = registry.load_matcher(config).unwrap();

        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "deep".into()
            }),
            Some("nested_hit".to_string())
        );
        // Nested matcher fails → falls through to on_no_match
        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "shallow".into()
            }),
            Some("fallback".to_string())
        );
    }

    #[test]
    fn registry_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Registry<TestCtx>>();
    }
}
