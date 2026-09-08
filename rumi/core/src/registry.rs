//! Type registry for config-driven matcher construction.
//!
//! The registry enables **generic config loading**: JSON/YAML/proto config -> compiled
//! `Matcher<Ctx, A>` without domain-specific compile code.
//!
//! # Architecture (axum `BoxedIntoRoute` pattern)
//!
//! Each `DataInput` type registers itself via [`DataInputFactory`]. Each `InputMatcher` type
//! registers via [`InputMatcherFactory`]. At registration time, the concrete type `T` is
//! monomorphized into a closure and erased behind `Box<dyn Fn>`. This is the same pattern
//! axum uses for route handlers — early type erasure at registration, late invocation at
//! request time.
//!
//! # Three Extension Seams (xDS-faithful)
//!
//! | Seam | Trait | Registry Method | Envoy Category |
//! |------|-------|-----------------|----------------|
//! | Inputs | [`DataInputFactory`] | `builder.input::<T>(url)` | `envoy.matching.common_inputs` |
//! | Matchers | [`InputMatcherFactory`] | `builder.matcher::<T>(url)` | `envoy.matching.input_matchers` |
//! | Actions | [`ActionFactory`] | `action_builder.action::<T>(url)` | `envoy.matching.action` |
//!
//! `Registry<Ctx>` handles inputs + matchers. [`ActionRegistry<A>`] handles actions
//! separately because `A` is unknown at `Registry` build time — it's introduced at
//! `load_typed_matcher()` call time.
//!
//! # Example
//!
//! ```ignore
//! let registry = RegistryBuilder::new()
//!     .input::<PathInput>("xuma.http.v1.PathInput")
//!     .input::<HeaderInput>("xuma.http.v1.HeaderInput")
//!     .build();
//!
//! let proto = rumi_proto::protojson::parse_matcher(&resolver, document)?;
//! let matcher = rumi_proto::convert::load_proto_matcher(&registry, &actions, &resolver, &proto)?;
//! ```
//!
//! The IR is built by a protojson reader, never parsed from a dialect of its
//! own (D-042).

use std::collections::HashMap;
use std::marker::PhantomData;

use serde::de::DeserializeOwned;

use crate::{
    ir::{FieldMatcherIr, MatcherIr, OnMatchIr, PredicateIr, SinglePredicateIr, ValueMatchIr},
    DataInput, FieldMatcher, InputMatcher, Matcher, MatcherError, MatcherKind, OnMatch, Predicate,
    SinglePredicate, MAX_FIELD_MATCHERS, MAX_PREDICATES_PER_COMPOUND,
};

/// Trait for `DataInput` types that can be constructed from configuration.
///
/// Each `DataInput` type knows its own config shape via the associated `Config` type.
/// The registry calls [`from_config`](Self::from_config) to construct the input at load time.
///
/// # Design (tower double dispatch)
///
/// `DataInput<Ctx>` describes *what* to extract. `DataInputFactory<Ctx>` describes *how* to
/// construct from config. The registry composes both: look up the `type_url` -> deserialize
/// config -> construct input -> pair with matcher.
pub trait DataInputFactory<Ctx: 'static>: Send + Sync + 'static {
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

/// Trait for `InputMatcher` types that can be constructed from configuration.
///
/// Maps to Envoy's `envoy.matching.input_matchers` extension category.
/// Unlike [`DataInputFactory`], this is NOT generic over `Ctx` — `InputMatcher` is
/// domain-agnostic by design.
///
/// # Example
///
/// ```ignore
/// impl InputMatcherFactory for BoolMatcher {
///     type Config = BoolMatcherConfig;
///     fn from_config(config: Self::Config) -> Result<Box<dyn InputMatcher>, MatcherError> {
///         Ok(Box::new(BoolMatcher::new(config.expected)))
///     }
/// }
/// ```
pub trait InputMatcherFactory: Send + Sync + 'static {
    /// The configuration type deserialized from JSON/YAML/proto.
    type Config: DeserializeOwned + Send + Sync;

    /// Construct an `InputMatcher` from deserialized configuration.
    ///
    /// # Errors
    ///
    /// Returns [`MatcherError::InvalidConfig`] if the config is semantically invalid,
    /// or [`MatcherError::InvalidPattern`] if a regex pattern is invalid.
    fn from_config(config: Self::Config) -> Result<Box<dyn InputMatcher>, MatcherError>;
}

/// Trait for action types that can be constructed from configuration.
///
/// Maps to Envoy's `envoy.matching.action` extension category.
/// Unlike [`DataInputFactory`] and [`InputMatcherFactory`], this is generic over the
/// action type `A` — different registries can produce different action types.
///
/// # Example
///
/// ```ignore
/// struct HttpAction { backend: String, timeout_ms: u64 }
///
/// struct HttpActionFactory;
/// impl ActionFactory<HttpAction> for HttpActionFactory {
///     type Config = HttpActionConfig;
///     fn from_config(config: Self::Config) -> Result<HttpAction, MatcherError> {
///         Ok(HttpAction { backend: config.backend, timeout_ms: config.timeout_ms })
///     }
/// }
/// ```
pub trait ActionFactory<A: Clone + Send + Sync + 'static>: Send + Sync + 'static {
    /// The configuration type deserialized from JSON/YAML/proto.
    type Config: DeserializeOwned + Send + Sync;

    /// Construct an action value from deserialized configuration.
    ///
    /// # Errors
    ///
    /// Returns [`MatcherError::InvalidConfig`] if the config is semantically invalid.
    fn from_config(config: Self::Config) -> Result<A, MatcherError>;
}

/// Type-erased input factory closure.
type BoxedInputFactory<Ctx> =
    Box<dyn Fn(&serde_json::Value) -> Result<Box<dyn DataInput<Ctx>>, MatcherError> + Send + Sync>;

/// Type-erased matcher factory closure.
type BoxedMatcherFactory =
    Box<dyn Fn(&serde_json::Value) -> Result<Box<dyn InputMatcher>, MatcherError> + Send + Sync>;

/// Type-erased action factory closure.
type BoxedActionFactory<A> =
    Box<dyn Fn(&serde_json::Value) -> Result<A, MatcherError> + Send + Sync>;

/// Builder for constructing a [`Registry`].
///
/// Register `DataInput` and `InputMatcher` types with their type URLs, then call
/// [`build()`](Self::build) to produce an immutable `Registry`.
///
/// # Arch-guild constraint: immutability after build
///
/// The builder pattern enforces the guild constraint that the registry must be
/// immutable after initialization. No runtime registration is possible.
pub struct RegistryBuilder<Ctx> {
    input_factories: HashMap<String, BoxedInputFactory<Ctx>>,
    matcher_factories: HashMap<String, BoxedMatcherFactory>,
    _phantom: PhantomData<Ctx>,
}

impl<Ctx: 'static> RegistryBuilder<Ctx> {
    /// Create a new empty registry builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input_factories: HashMap::new(),
            matcher_factories: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Register a `DataInput` type with a type URL.
    ///
    /// The concrete type `T` is monomorphized here and erased behind a closure.
    /// At load time, the registry deserializes config as `T::Config` and calls
    /// `T::from_config()` to produce the `DataInput`.
    #[must_use]
    pub fn input<T: DataInputFactory<Ctx>>(mut self, type_url: &str) -> Self {
        self.input_factories.insert(
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

    /// Register an `InputMatcher` type with a type URL.
    ///
    /// The concrete type `T` is monomorphized here and erased behind a closure.
    /// At load time, the registry deserializes config as `T::Config` and calls
    /// `T::from_config()` to produce the `InputMatcher`.
    #[must_use]
    pub fn matcher<T: InputMatcherFactory>(mut self, type_url: &str) -> Self {
        self.matcher_factories.insert(
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
            input_factories: self.input_factories,
            matcher_factories: self.matcher_factories,
            _phantom: PhantomData,
        }
    }
}

impl<Ctx: 'static> Default for RegistryBuilder<Ctx> {
    fn default() -> Self {
        Self::new()
    }
}

/// Register core built-in matchers (`BoolMatcher`, `StringMatcher`).
///
/// Call this in domain `register()` functions to avoid duplicating core matcher
/// registrations. Domain-specific inputs are then added on top.
///
/// # Example
///
/// ```ignore
/// pub fn register(builder: RegistryBuilder<MyCtx>) -> RegistryBuilder<MyCtx> {
///     rumi::register_core_matchers(builder)
///         .input::<MyInput>("xuma.my.v1.MyInput")
/// }
/// ```
#[must_use]
pub fn register_core_matchers<Ctx: 'static>(builder: RegistryBuilder<Ctx>) -> RegistryBuilder<Ctx> {
    use crate::BoolMatcher;

    // `valueMatch` compares strings only, so a boolean has nowhere else to go;
    // `xuma.core.v1.StringMatcher` is not registered because `valueMatch`
    // already says it (D-041).
    builder.matcher::<BoolMatcher>("xuma.core.v1.BoolMatcher")
}

/// Builder for constructing an [`ActionRegistry`].
///
/// Separate from [`RegistryBuilder`] because the action type `A` is not known
/// at `Registry<Ctx>` build time — it's introduced at `load_typed_matcher()` call time.
pub struct ActionRegistryBuilder<A> {
    factories: HashMap<String, BoxedActionFactory<A>>,
}

impl<A: Clone + Send + Sync + 'static> ActionRegistryBuilder<A> {
    /// Create a new empty action registry builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Register an action type with a type URL.
    #[must_use]
    pub fn action<T: ActionFactory<A>>(mut self, type_url: &str) -> Self {
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

    /// Freeze the action registry. No further registration is possible.
    #[must_use]
    pub fn build(self) -> ActionRegistry<A> {
        ActionRegistry {
            factories: self.factories,
        }
    }
}

impl<A: Clone + Send + Sync + 'static> Default for ActionRegistryBuilder<A> {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable registry of action factories.
///
/// Resolves `TypedExtensionConfig` actions into concrete `A` values via registered factories.
/// Used with [`Registry::load_typed_matcher()`] for fully-typed config loading.
pub struct ActionRegistry<A> {
    factories: HashMap<String, BoxedActionFactory<A>>,
}

impl<A: Clone + Send + Sync + 'static> ActionRegistry<A> {
    /// Returns the number of registered action types.
    #[must_use]
    pub fn len(&self) -> usize {
        self.factories.len()
    }

    /// Returns `true` if no action types are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }

    /// Returns `true` if the given action type URL is registered.
    #[must_use]
    pub fn contains(&self, type_url: &str) -> bool {
        self.factories.contains_key(type_url)
    }

    /// Returns the registered action type URLs.
    #[must_use]
    pub fn type_urls(&self) -> Vec<&str> {
        let mut urls: Vec<&str> = self.factories.keys().map(String::as_str).collect();
        urls.sort_unstable();
        urls
    }

    /// Resolve one action's config into an `A` through its registered factory.
    ///
    /// # Errors
    ///
    /// [`MatcherError::UnknownTypeUrl`] if no factory is registered for the
    /// config's type URL, or whatever the factory returns.
    pub fn resolve(&self, config: &crate::ir::TypedExtensionConfig) -> Result<A, MatcherError> {
        let factory =
            self.factories
                .get(&config.type_url)
                .ok_or_else(|| MatcherError::UnknownTypeUrl {
                    type_url: config.type_url.clone(),
                    registry: "action",
                    available: self.factories.keys().cloned().collect(),
                })?;
        factory(&config.config)
    }
}

/// Immutable registry of `DataInput` and `InputMatcher` factories.
///
/// Constructed via [`RegistryBuilder`]. Use [`load_matcher()`](Self::load_matcher)
/// to compile config into a runtime `Matcher`.
/// What one load has spent against the document-wide budgets (D-054).
///
/// Counted as the walk goes, so a document is refused at the leaf or regex
/// that crosses a budget rather than after everything past it has been built.
#[derive(Default)]
struct LoadBudget {
    leaves: usize,
    regexes: usize,
}

impl LoadBudget {
    fn leaf(&mut self) -> Result<(), MatcherError> {
        self.leaves += 1;
        if self.leaves > crate::MAX_LEAF_PREDICATES {
            return Err(MatcherError::TooManyLeafPredicates {
                count: self.leaves,
                max: crate::MAX_LEAF_PREDICATES,
            });
        }
        Ok(())
    }

    fn regex(&mut self) -> Result<(), MatcherError> {
        self.regexes += 1;
        if self.regexes > crate::MAX_REGEXES {
            return Err(MatcherError::TooManyRegexes {
                count: self.regexes,
                max: crate::MAX_REGEXES,
            });
        }
        Ok(())
    }
}

pub struct Registry<Ctx> {
    input_factories: HashMap<String, BoxedInputFactory<Ctx>>,
    matcher_factories: HashMap<String, BoxedMatcherFactory>,
    _phantom: PhantomData<Ctx>,
}

impl<Ctx: 'static> Registry<Ctx> {
    /// Load a `Matcher` from configuration.
    ///
    /// Walks the config tree, constructs `DataInput`s and `InputMatcher`s via
    /// registered factories, validates type compatibility, builds predicates and
    /// field matchers, and validates depth constraints.
    ///
    /// # Errors
    ///
    /// - [`MatcherError::UnknownTypeUrl`] — input or matcher `type_url` not registered
    /// - [`MatcherError::InvalidConfig`] — config deserialization or construction failed
    /// - [`MatcherError::InvalidPattern`] — regex pattern is invalid
    /// - [`MatcherError::IncompatibleTypes`] — input data type vs matcher supported types
    /// - [`MatcherError::DepthExceeded`] — matcher nesting exceeds [`MAX_DEPTH`](crate::MAX_DEPTH)
    pub fn load_matcher<A>(&self, config: MatcherIr<A>) -> Result<Matcher<Ctx, A>, MatcherError>
    where
        A: Clone + Send + Sync + 'static,
    {
        let mut budget = LoadBudget::default();
        let matcher = self.load_matcher_in(config, &mut budget)?;
        matcher.validate()?;
        Ok(matcher)
    }

    fn load_matcher_in<A>(
        &self,
        config: MatcherIr<A>,
        budget: &mut LoadBudget,
    ) -> Result<Matcher<Ctx, A>, MatcherError>
    where
        A: Clone + Send + Sync + 'static,
    {
        let kind = match config.kind {
            crate::ir::MatcherKindIr::List(matchers) => {
                if matchers.len() > MAX_FIELD_MATCHERS {
                    return Err(MatcherError::TooManyFieldMatchers {
                        count: matchers.len(),
                        max: MAX_FIELD_MATCHERS,
                    });
                }
                let matchers = matchers
                    .into_iter()
                    .map(|fm| self.load_field_matcher(fm, budget))
                    .collect::<Result<Vec<_>, _>>()?;
                MatcherKind::List(matchers)
            }
            crate::ir::MatcherKindIr::Tree(tree) => {
                MatcherKind::Tree(
                    self.load_tree(tree, budget, |this, om, b| this.load_on_match(om, b))?,
                )
            }
        };
        let on_no_match = config
            .on_no_match
            .map(|om| self.load_on_match(om, budget))
            .transpose()?;
        Ok(match kind {
            MatcherKind::List(matchers) => Matcher::list(matchers, on_no_match),
            MatcherKind::Tree(tree) => Matcher::tree(tree, on_no_match),
        })
    }

    /// Load a `Matcher` from configuration with typed action resolution.
    ///
    /// Like [`load_matcher()`](Self::load_matcher), but actions in the config are
    /// `TypedExtensionConfig` values resolved through the [`ActionRegistry`] instead of
    /// being deserialized directly as `A`.
    ///
    /// # Two loaders
    ///
    /// | Loader | Config Action Type | Resolution |
    /// |--------|-------------------|------------|
    /// | `load_matcher()` | `A` (direct) | `serde::Deserialize` |
    /// | `load_typed_matcher()` | `TypedExtensionConfig` | `ActionRegistry<A>` factory |
    ///
    /// # Errors
    ///
    /// Same as [`load_matcher()`](Self::load_matcher), plus:
    /// - [`MatcherError::UnknownTypeUrl`] with `registry: "action"` for unregistered action type URLs
    pub fn load_typed_matcher<A>(
        &self,
        config: MatcherIr<crate::ir::TypedExtensionConfig>,
        actions: &ActionRegistry<A>,
    ) -> Result<Matcher<Ctx, A>, MatcherError>
    where
        A: Clone + Send + Sync + 'static,
    {
        let mut budget = LoadBudget::default();
        let matcher = self.load_typed_matcher_in(config, actions, &mut budget)?;
        matcher.validate()?;
        Ok(matcher)
    }

    fn load_typed_matcher_in<A>(
        &self,
        config: MatcherIr<crate::ir::TypedExtensionConfig>,
        actions: &ActionRegistry<A>,
        budget: &mut LoadBudget,
    ) -> Result<Matcher<Ctx, A>, MatcherError>
    where
        A: Clone + Send + Sync + 'static,
    {
        let kind = match config.kind {
            crate::ir::MatcherKindIr::List(matchers) => {
                if matchers.len() > MAX_FIELD_MATCHERS {
                    return Err(MatcherError::TooManyFieldMatchers {
                        count: matchers.len(),
                        max: MAX_FIELD_MATCHERS,
                    });
                }
                let matchers = matchers
                    .into_iter()
                    .map(|fm| self.load_typed_field_matcher(fm, actions, budget))
                    .collect::<Result<Vec<_>, _>>()?;
                MatcherKind::List(matchers)
            }
            crate::ir::MatcherKindIr::Tree(tree) => {
                MatcherKind::Tree(self.load_tree(tree, budget, |this, om, b| {
                    this.load_typed_on_match(om, actions, b)
                })?)
            }
        };
        let on_no_match = config
            .on_no_match
            .map(|om| self.load_typed_on_match(om, actions, budget))
            .transpose()?;
        Ok(match kind {
            MatcherKind::List(matchers) => Matcher::list(matchers, on_no_match),
            MatcherKind::Tree(tree) => Matcher::tree(tree, on_no_match),
        })
    }

    /// Returns the number of registered input types.
    #[must_use]
    pub fn input_count(&self) -> usize {
        self.input_factories.len()
    }

    /// Build a [`MatcherTree`] from config, resolving its input and entries.
    ///
    /// Generic over how an `OnMatch` is loaded so the direct and typed-action
    /// loaders share one implementation — the entry semantics are identical and
    /// only action resolution differs.
    fn load_tree<A, C, F>(
        &self,
        config: crate::ir::MatcherTreeIr<C>,
        budget: &mut LoadBudget,
        mut load_on_match: F,
    ) -> Result<crate::MatcherTree<Ctx, A>, MatcherError>
    where
        A: Clone + Send + Sync + 'static,
        F: FnMut(
            &Self,
            crate::ir::OnMatchIr<C>,
            &mut LoadBudget,
        ) -> Result<crate::OnMatch<Ctx, A>, MatcherError>,
    {
        use crate::ir::TreeTypeIr;

        let input_factory = self
            .input_factories
            .get(&config.input.type_url)
            .ok_or_else(|| MatcherError::UnknownTypeUrl {
                type_url: config.input.type_url.clone(),
                registry: "input",
                available: self.input_factories.keys().cloned().collect(),
            })?;
        let input = input_factory(&config.input.config)?;

        let (entries, is_prefix) = match config.tree {
            TreeTypeIr::ExactMatchMap(e) => (e, false),
            TreeTypeIr::PrefixMatchMap(e) => (e, true),
        };

        // Width is checked before building, so a config that would blow memory
        // is rejected rather than materialised and then measured.
        if entries.len() > crate::MAX_TREE_ENTRIES {
            return Err(MatcherError::TooManyTreeEntries {
                count: entries.len(),
                max: crate::MAX_TREE_ENTRIES,
            });
        }

        let loaded = entries
            .into_iter()
            .map(|(key, om)| load_on_match(self, om, budget).map(|om| (key, om)))
            .collect::<Result<Vec<_>, _>>()?;

        if is_prefix {
            crate::MatcherTree::prefix(input, loaded)
        } else {
            crate::MatcherTree::exact(input, loaded)
        }
    }

    /// Returns the number of registered matcher types.
    #[must_use]
    pub fn matcher_count(&self) -> usize {
        self.matcher_factories.len()
    }

    /// Returns `true` if no types are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.input_factories.is_empty() && self.matcher_factories.is_empty()
    }

    /// Returns `true` if the given input type URL is registered.
    #[must_use]
    pub fn contains_input(&self, type_url: &str) -> bool {
        self.input_factories.contains_key(type_url)
    }

    /// Returns `true` if the given matcher type URL is registered.
    #[must_use]
    pub fn contains_matcher(&self, type_url: &str) -> bool {
        self.matcher_factories.contains_key(type_url)
    }

    /// Returns all registered input type URLs (sorted).
    #[must_use]
    pub fn input_type_urls(&self) -> Vec<&str> {
        let mut urls: Vec<&str> = self.input_factories.keys().map(String::as_str).collect();
        urls.sort_unstable();
        urls
    }

    /// Returns all registered matcher type URLs (sorted).
    #[must_use]
    pub fn matcher_type_urls(&self) -> Vec<&str> {
        let mut urls: Vec<&str> = self.matcher_factories.keys().map(String::as_str).collect();
        urls.sort_unstable();
        urls
    }

    fn load_field_matcher<A>(
        &self,
        config: FieldMatcherIr<A>,
        budget: &mut LoadBudget,
    ) -> Result<FieldMatcher<Ctx, A>, MatcherError>
    where
        A: Clone + Send + Sync + 'static,
    {
        let predicate = self.load_predicate(config.predicate, budget)?;
        let on_match = self.load_on_match(config.on_match, budget)?;
        Ok(FieldMatcher::new(predicate, on_match))
    }

    fn load_predicate(
        &self,
        config: PredicateIr,
        budget: &mut LoadBudget,
    ) -> Result<Predicate<Ctx>, MatcherError> {
        match config {
            PredicateIr::Single(single) => {
                let sp = self.load_single(single, budget)?;
                Ok(Predicate::Single(sp))
            }
            PredicateIr::And { predicates } => {
                if predicates.len() > MAX_PREDICATES_PER_COMPOUND {
                    return Err(MatcherError::TooManyPredicates {
                        count: predicates.len(),
                        max: MAX_PREDICATES_PER_COMPOUND,
                    });
                }
                let ps = predicates
                    .into_iter()
                    .map(|p| self.load_predicate(p, budget))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Predicate::And(ps))
            }
            PredicateIr::Or { predicates } => {
                if predicates.len() > MAX_PREDICATES_PER_COMPOUND {
                    return Err(MatcherError::TooManyPredicates {
                        count: predicates.len(),
                        max: MAX_PREDICATES_PER_COMPOUND,
                    });
                }
                let ps = predicates
                    .into_iter()
                    .map(|p| self.load_predicate(p, budget))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Predicate::Or(ps))
            }
            PredicateIr::Not { predicate } => {
                let inner = self.load_predicate(*predicate, budget)?;
                Ok(Predicate::Not(Box::new(inner)))
            }
        }
    }

    #[allow(clippy::needless_pass_by_value)] // Consistent with other load_ methods
    fn load_single(
        &self,
        config: SinglePredicateIr,
        budget: &mut LoadBudget,
    ) -> Result<SinglePredicate<Ctx>, MatcherError> {
        budget.leaf()?;
        if let ValueMatchIr::BuiltIn {
            spec: crate::StringMatchSpec::Regex(_),
            ..
        } = &config.matcher
        {
            budget.regex()?;
        }
        let input_factory = self
            .input_factories
            .get(&config.input.type_url)
            .ok_or_else(|| MatcherError::UnknownTypeUrl {
                type_url: config.input.type_url.clone(),
                registry: "input",
                available: self.input_factories.keys().cloned().collect(),
            })?;
        let input = input_factory(&config.input.config)?;

        let matcher = match config.matcher {
            ValueMatchIr::BuiltIn {
                ref spec,
                ignore_case,
            } => {
                // The constructor that owns the resource holds the limits and
                // the flag, so the loader cannot keep a copy that drifts (D-029).
                spec.to_input_matcher_with_case(ignore_case)?
            }
            ValueMatchIr::Custom(tc) => {
                let matcher_factory =
                    self.matcher_factories.get(&tc.type_url).ok_or_else(|| {
                        MatcherError::UnknownTypeUrl {
                            type_url: tc.type_url.clone(),
                            registry: "matcher",
                            available: self.matcher_factories.keys().cloned().collect(),
                        }
                    })?;
                matcher_factory(&tc.config)?
            }
        };

        let data_type = input.data_type();
        let supported = matcher.supported_types();
        if !supported.contains(&data_type) {
            return Err(MatcherError::IncompatibleTypes {
                input_type: data_type.to_string(),
                matcher_types: supported.iter().map(|s| (*s).to_string()).collect(),
            });
        }

        Ok(SinglePredicate::new(input, matcher))
    }

    fn load_on_match<A>(
        &self,
        config: OnMatchIr<A>,
        budget: &mut LoadBudget,
    ) -> Result<OnMatch<Ctx, A>, MatcherError>
    where
        A: Clone + Send + Sync + 'static,
    {
        match config {
            OnMatchIr::Action { action } => Ok(OnMatch::Action(action)),
            OnMatchIr::Matcher { matcher } => {
                let m = self.load_matcher_in(*matcher, budget)?;
                Ok(OnMatch::Matcher(Box::new(m)))
            }
        }
    }

    fn load_typed_field_matcher<A>(
        &self,
        config: FieldMatcherIr<crate::ir::TypedExtensionConfig>,
        actions: &ActionRegistry<A>,
        budget: &mut LoadBudget,
    ) -> Result<FieldMatcher<Ctx, A>, MatcherError>
    where
        A: Clone + Send + Sync + 'static,
    {
        let predicate = self.load_predicate(config.predicate, budget)?;
        let on_match = self.load_typed_on_match(config.on_match, actions, budget)?;
        Ok(FieldMatcher::new(predicate, on_match))
    }

    fn load_typed_on_match<A>(
        &self,
        config: OnMatchIr<crate::ir::TypedExtensionConfig>,
        actions: &ActionRegistry<A>,
        budget: &mut LoadBudget,
    ) -> Result<OnMatch<Ctx, A>, MatcherError>
    where
        A: Clone + Send + Sync + 'static,
    {
        match config {
            OnMatchIr::Action { action } => {
                let resolved = actions.resolve(&action)?;
                Ok(OnMatch::Action(resolved))
            }
            OnMatchIr::Matcher { matcher } => {
                let m = self.load_typed_matcher_in(*matcher, actions, budget)?;
                Ok(OnMatch::Matcher(Box::new(m)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::TypedExtensionConfig;
    use crate::MatchingData;
    use crate::StringMatchSpec;

    #[derive(Debug)]
    struct TestCtx {
        value: String,
    }

    #[derive(Debug)]
    struct ValueInput {
        key: String,
    }

    impl DataInput<TestCtx> for ValueInput {
        fn get(&self, ctx: &TestCtx) -> Option<MatchingData> {
            (self.key == "value").then(|| MatchingData::String(ctx.value.clone()))
        }
    }

    #[derive(serde::Deserialize)]
    struct ValueInputConfig {
        key: String,
    }

    impl DataInputFactory<TestCtx> for ValueInput {
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

        assert_eq!(registry.input_count(), 1);
        assert!(registry.contains_input("test.ValueInput"));
        assert!(!registry.contains_input("test.Unknown"));
    }

    #[test]
    fn builder_registers_matchers() {
        let registry = register_core_matchers(RegistryBuilder::<TestCtx>::new())
            .input::<ValueInput>("test.ValueInput")
            .build();

        assert_eq!(registry.input_count(), 1);
        assert_eq!(registry.matcher_count(), 1);
        assert!(registry.contains_matcher("xuma.core.v1.BoolMatcher"));
        assert!(!registry.contains_matcher("xuma.core.v1.StringMatcher"));
    }

    // Built, never parsed (D-042). The helpers decide nothing, so each test
    // asserts only what it states.

    fn typed(type_url: &str, config: serde_json::Value) -> TypedExtensionConfig {
        TypedExtensionConfig {
            type_url: type_url.into(),
            config,
        }
    }

    /// The `ValueInput` reading key `value`, which most tests here use.
    fn value_input() -> TypedExtensionConfig {
        typed("test.ValueInput", serde_json::json!({ "key": "value" }))
    }

    /// A single predicate matching with a built-in `StringMatchSpec`.
    ///
    /// `ignore_case: false` because the terse dialect these tests were written
    /// in had no `ignore_case` sibling — it is protojson that carries the flag.
    /// Passing `false` keeps every assertion below testing what it tested.
    fn built_in(input: TypedExtensionConfig, spec: StringMatchSpec) -> PredicateIr {
        PredicateIr::Single(SinglePredicateIr {
            input,
            matcher: ValueMatchIr::BuiltIn {
                spec,
                ignore_case: false,
            },
        })
    }

    /// A single predicate matching via a registered matcher type URL.
    fn custom(input: TypedExtensionConfig, matcher: TypedExtensionConfig) -> PredicateIr {
        PredicateIr::Single(SinglePredicateIr {
            input,
            matcher: ValueMatchIr::Custom(matcher),
        })
    }

    fn act(action: &str) -> OnMatchIr<String> {
        OnMatchIr::Action {
            action: action.into(),
        }
    }

    /// An action resolved through the `ActionRegistry`, for `load_typed_matcher`.
    fn act_typed(type_url: &str, config: serde_json::Value) -> OnMatchIr<TypedExtensionConfig> {
        OnMatchIr::Action {
            action: typed(type_url, config),
        }
    }

    fn field<A>(predicate: PredicateIr, on_match: OnMatchIr<A>) -> FieldMatcherIr<A> {
        FieldMatcherIr {
            predicate,
            on_match,
        }
    }

    /// A matcher of one field matcher and no fallback — the common shape.
    fn one<A>(predicate: PredicateIr, on_match: OnMatchIr<A>) -> MatcherIr<A> {
        MatcherIr::list(vec![field(predicate, on_match)])
    }

    #[test]
    fn load_simple_matcher() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let config = MatcherIr::list(vec![field(
            built_in(value_input(), StringMatchSpec::Exact("hello".into())),
            act("matched!"),
        )])
        .with_fallback(Some(act("default")));
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
    fn load_custom_match_bool_matcher() {
        #[derive(Debug)]
        struct BoolInput;
        impl DataInput<TestCtx> for BoolInput {
            fn get(&self, ctx: &TestCtx) -> Option<MatchingData> {
                Some(MatchingData::Bool(ctx.value == "yes"))
            }
            fn data_type(&self) -> &'static str {
                "bool"
            }
        }

        impl DataInputFactory<TestCtx> for BoolInput {
            type Config = crate::UnitConfig;
            fn from_config(_: Self::Config) -> Result<Box<dyn DataInput<TestCtx>>, MatcherError> {
                Ok(Box::new(BoolInput))
            }
        }

        let registry = register_core_matchers(RegistryBuilder::<TestCtx>::new())
            .input::<BoolInput>("test.BoolInput")
            .build();

        let config = one(
            custom(
                typed("test.BoolInput", serde_json::json!({})),
                typed(
                    "xuma.core.v1.BoolMatcher",
                    serde_json::json!({ "expected": true }),
                ),
            ),
            act("bool_hit"),
        );
        let matcher = registry.load_matcher(config).unwrap();

        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "yes".into()
            }),
            Some("bool_hit".to_string())
        );
        assert_eq!(matcher.evaluate(&TestCtx { value: "no".into() }), None);
    }

    #[test]
    fn load_custom_match_string_matcher() {
        // Registered by hand, as a custom domain would: the shipped
        // registration was removed because `valueMatch` already says it (D-041).
        let registry = register_core_matchers(RegistryBuilder::<TestCtx>::new())
            .input::<ValueInput>("test.ValueInput")
            .matcher::<crate::StringMatcher>("xuma.core.v1.StringMatcher")
            .build();

        let config = one(
            custom(
                value_input(),
                typed(
                    "xuma.core.v1.StringMatcher",
                    serde_json::json!({ "value": "/API/", "match_type": "prefix", "ignore_case": true }),
                ),
            ),
            act("api_route"),
        );
        let matcher = registry.load_matcher(config).unwrap();

        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "/api/users".into()
            }),
            Some("api_route".to_string())
        );
        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "/other".into()
            }),
            None
        );
    }

    #[test]
    fn unknown_input_type_url() {
        let registry = RegistryBuilder::<TestCtx>::new().build();

        let config = one(
            built_in(
                typed("unknown.Input", serde_json::json!({})),
                StringMatchSpec::Exact("x".into()),
            ),
            act("x"),
        );
        let err = registry.load_matcher(config).unwrap_err();
        match err {
            MatcherError::UnknownTypeUrl {
                ref type_url,
                registry,
                ref available,
            } => {
                assert_eq!(type_url, "unknown.Input");
                assert_eq!(registry, "input");
                assert!(available.is_empty());
            }
            _ => panic!("expected UnknownTypeUrl, got {err:?}"),
        }
    }

    #[test]
    fn unknown_input_lists_available() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let config = one(
            built_in(
                typed("unknown.Input", serde_json::json!({})),
                StringMatchSpec::Exact("x".into()),
            ),
            act("x"),
        );
        let err = registry.load_matcher(config).unwrap_err();
        match err {
            MatcherError::UnknownTypeUrl { ref available, .. } => {
                assert_eq!(available, &["test.ValueInput"]);
                let msg = err.to_string();
                assert!(
                    msg.contains("test.ValueInput"),
                    "error should list available URLs: {msg}"
                );
            }
            _ => panic!("expected UnknownTypeUrl, got {err:?}"),
        }
    }

    #[test]
    fn unknown_matcher_type_url() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let config = one(
            custom(
                value_input(),
                typed("unknown.Matcher", serde_json::json!({})),
            ),
            act("x"),
        );
        let err = registry.load_matcher(config).unwrap_err();
        match err {
            MatcherError::UnknownTypeUrl {
                ref type_url,
                registry,
                ..
            } => {
                assert_eq!(type_url, "unknown.Matcher");
                assert_eq!(registry, "matcher");
            }
            _ => panic!("expected UnknownTypeUrl, got {err:?}"),
        }
    }

    #[test]
    fn incompatible_types_error() {
        let registry = register_core_matchers(RegistryBuilder::<TestCtx>::new())
            .input::<ValueInput>("test.ValueInput")
            .build();

        let config = one(
            custom(
                value_input(),
                typed(
                    "xuma.core.v1.BoolMatcher",
                    serde_json::json!({ "expected": true }),
                ),
            ),
            act("x"),
        );
        let err = registry.load_matcher(config).unwrap_err();
        match err {
            MatcherError::IncompatibleTypes {
                ref input_type,
                ref matcher_types,
            } => {
                assert_eq!(input_type, "string");
                assert_eq!(matcher_types, &["bool"]);
            }
            _ => panic!("expected IncompatibleTypes, got {err:?}"),
        }
    }

    #[test]
    fn invalid_config_returns_error() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let config = one(
            built_in(
                typed("test.ValueInput", serde_json::json!({ "wrong_field": 42 })),
                StringMatchSpec::Exact("x".into()),
            ),
            act("x"),
        );
        let err = registry.load_matcher(config).unwrap_err();
        assert!(matches!(err, MatcherError::InvalidConfig { .. }));
    }

    #[test]
    fn load_and_predicate() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let config = one(
            PredicateIr::And {
                predicates: vec![
                    built_in(value_input(), StringMatchSpec::Prefix("hel".into())),
                    built_in(value_input(), StringMatchSpec::Suffix("llo".into())),
                ],
            },
            act("both_matched"),
        );
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

        let config = MatcherIr::list(vec![field(
            built_in(value_input(), StringMatchSpec::Prefix(String::new())),
            OnMatchIr::Matcher {
                matcher: Box::new(one(
                    built_in(value_input(), StringMatchSpec::Exact("deep".into())),
                    act("nested_hit"),
                )),
            },
        )])
        .with_fallback(Some(act("fallback")));
        let matcher = registry.load_matcher(config).unwrap();

        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "deep".into()
            }),
            Some("nested_hit".to_string())
        );
        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "shallow".into()
            }),
            Some("fallback".to_string())
        );
    }

    struct StringActionFactory;

    #[derive(serde::Deserialize)]
    struct StringActionConfig {
        value: String,
    }

    impl ActionFactory<String> for StringActionFactory {
        type Config = StringActionConfig;

        fn from_config(config: Self::Config) -> Result<String, MatcherError> {
            Ok(config.value)
        }
    }

    #[test]
    fn action_registry_builds_and_resolves() {
        let actions = ActionRegistryBuilder::new()
            .action::<StringActionFactory>("test.StringAction")
            .build();

        assert_eq!(actions.len(), 1);
        assert!(actions.contains("test.StringAction"));
        assert!(!actions.contains("test.Unknown"));
    }

    #[test]
    fn load_typed_matcher_end_to_end() {
        let registry = register_core_matchers(RegistryBuilder::<TestCtx>::new())
            .input::<ValueInput>("test.ValueInput")
            .build();

        let actions = ActionRegistryBuilder::new()
            .action::<StringActionFactory>("test.StringAction")
            .build();

        let config = MatcherIr::list(vec![field(
            built_in(value_input(), StringMatchSpec::Exact("hello".into())),
            act_typed(
                "test.StringAction",
                serde_json::json!({ "value": "typed_hit" }),
            ),
        )])
        .with_fallback(Some(act_typed(
            "test.StringAction",
            serde_json::json!({ "value": "typed_miss" }),
        )));
        let matcher = registry.load_typed_matcher(config, &actions).unwrap();

        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "hello".into()
            }),
            Some("typed_hit".to_string())
        );
        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "world".into()
            }),
            Some("typed_miss".to_string())
        );
    }

    #[test]
    fn load_typed_matcher_unknown_action_type_url() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let actions: ActionRegistry<String> = ActionRegistryBuilder::new().build();

        let config = one(
            built_in(value_input(), StringMatchSpec::Exact("x".into())),
            act_typed("unknown.Action", serde_json::json!({})),
        );
        let err = registry.load_typed_matcher(config, &actions).unwrap_err();
        match err {
            MatcherError::UnknownTypeUrl {
                ref type_url,
                registry,
                ..
            } => {
                assert_eq!(type_url, "unknown.Action");
                assert_eq!(registry, "action");
            }
            _ => panic!("expected UnknownTypeUrl for action, got {err:?}"),
        }
    }

    #[test]
    fn load_typed_matcher_with_nested_matcher() {
        let registry = register_core_matchers(RegistryBuilder::<TestCtx>::new())
            .input::<ValueInput>("test.ValueInput")
            .build();

        let actions = ActionRegistryBuilder::new()
            .action::<StringActionFactory>("test.StringAction")
            .build();

        let config = one(
            built_in(value_input(), StringMatchSpec::Prefix(String::new())),
            OnMatchIr::Matcher {
                matcher: Box::new(one(
                    built_in(value_input(), StringMatchSpec::Exact("deep".into())),
                    act_typed(
                        "test.StringAction",
                        serde_json::json!({ "value": "nested_typed" }),
                    ),
                )),
            },
        );
        let matcher = registry.load_typed_matcher(config, &actions).unwrap();

        assert_eq!(
            matcher.evaluate(&TestCtx {
                value: "deep".into()
            }),
            Some("nested_typed".to_string())
        );
        assert_eq!(matcher.evaluate(&TestCtx { value: "x".into() }), None);
    }

    #[test]
    fn introspection_input_type_urls() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("b.Input")
            .input::<ValueInput>("a.Input")
            .build();

        assert_eq!(registry.input_type_urls(), vec!["a.Input", "b.Input"]);
    }

    #[test]
    fn introspection_matcher_type_urls() {
        let registry = register_core_matchers(RegistryBuilder::<TestCtx>::new()).build();

        let urls = registry.matcher_type_urls();
        assert_eq!(urls.len(), 1);
        assert!(urls.contains(&"xuma.core.v1.BoolMatcher"));
        assert!(!urls.contains(&"xuma.core.v1.StringMatcher"));
    }

    #[test]
    fn too_many_field_matchers() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let fm = || {
            field(
                built_in(value_input(), StringMatchSpec::Exact("x".into())),
                act("x"),
            )
        };
        let matchers: Vec<_> = (0..=crate::MAX_FIELD_MATCHERS).map(|_| fm()).collect();
        let config = MatcherIr::list(matchers);
        let err = registry.load_matcher(config).unwrap_err();
        match err {
            MatcherError::TooManyFieldMatchers { count, max } => {
                assert_eq!(count, crate::MAX_FIELD_MATCHERS + 1);
                assert_eq!(max, crate::MAX_FIELD_MATCHERS);
            }
            _ => panic!("expected TooManyFieldMatchers, got {err:?}"),
        }
    }

    #[test]
    fn too_many_predicates_and() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let predicates: Vec<_> = (0..=crate::MAX_PREDICATES_PER_COMPOUND)
            .map(|_| built_in(value_input(), StringMatchSpec::Exact("x".into())))
            .collect();

        let config = one(PredicateIr::And { predicates }, act("x"));
        let err = registry.load_matcher(config).unwrap_err();
        assert!(
            matches!(err, MatcherError::TooManyPredicates { .. }),
            "expected TooManyPredicates, got {err:?}"
        );
    }

    #[test]
    fn too_many_predicates_or() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let predicates: Vec<_> = (0..=crate::MAX_PREDICATES_PER_COMPOUND)
            .map(|_| built_in(value_input(), StringMatchSpec::Exact("x".into())))
            .collect();

        let config = one(PredicateIr::Or { predicates }, act("x"));
        let err = registry.load_matcher(config).unwrap_err();
        assert!(
            matches!(err, MatcherError::TooManyPredicates { .. }),
            "expected TooManyPredicates, got {err:?}"
        );
    }

    #[test]
    fn pattern_too_long_exact() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let long_pattern = "x".repeat(crate::MAX_PATTERN_LENGTH + 1);
        let config = one(
            built_in(value_input(), StringMatchSpec::Exact(long_pattern)),
            act("x"),
        );
        let err = registry.load_matcher(config).unwrap_err();
        match err {
            MatcherError::PatternTooLong { len, max } => {
                assert_eq!(len, crate::MAX_PATTERN_LENGTH + 1);
                assert_eq!(max, crate::MAX_PATTERN_LENGTH);
            }
            _ => panic!("expected PatternTooLong, got {err:?}"),
        }
    }

    #[test]
    fn regex_pattern_too_long() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let long_regex = "a".repeat(crate::MAX_REGEX_PATTERN_LENGTH + 1);
        let config = one(
            built_in(value_input(), StringMatchSpec::Regex(long_regex)),
            act("x"),
        );
        let err = registry.load_matcher(config).unwrap_err();
        match err {
            MatcherError::PatternTooLong { len, max } => {
                assert_eq!(len, crate::MAX_REGEX_PATTERN_LENGTH + 1);
                assert_eq!(max, crate::MAX_REGEX_PATTERN_LENGTH);
            }
            _ => panic!("expected PatternTooLong, got {err:?}"),
        }
    }

    #[test]
    fn pattern_at_limit_succeeds() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let pattern = "x".repeat(crate::MAX_PATTERN_LENGTH);
        let config = one(
            built_in(value_input(), StringMatchSpec::Exact(pattern)),
            act("x"),
        );
        assert!(registry.load_matcher(config).is_ok());
    }

    #[test]
    fn field_matchers_at_limit_succeeds() {
        let registry = RegistryBuilder::<TestCtx>::new()
            .input::<ValueInput>("test.ValueInput")
            .build();

        let fm = || {
            field(
                built_in(value_input(), StringMatchSpec::Exact("x".into())),
                act("x"),
            )
        };
        let matchers: Vec<_> = (0..crate::MAX_FIELD_MATCHERS).map(|_| fm()).collect();
        let config = MatcherIr::list(matchers);
        assert!(registry.load_matcher(config).is_ok());
    }
}
