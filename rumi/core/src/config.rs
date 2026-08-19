//! Config types for generic matcher construction.
//!
//! These mirror the runtime matcher types and are what
//! [`Registry::load_matcher()`] consumes. They are **built, not parsed**: the
//! authored config format is canonical protojson, read by `rumi-proto` into
//! xDS proto types and converted here.
//!
//! None of the tree-shape types below derives `Deserialize`, and that is the
//! enforcement rather than a style choice. While they did, a second, terser
//! JSON dialect remained authorable in Rust — one no fixture used, no other
//! implementation could read, and nothing documented. Removing the impl makes
//! writing that dialect a compile error instead of a convention.
//!
//! The payload types keep theirs: a `TypedConfig.config` body is deserialized
//! as the registered factory's `Config` associated type, which is the
//! extension seam working as designed.
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
/// Built by a protojson reader, then loaded into a runtime `Matcher` via
/// [`Registry::load_matcher()`](crate::Registry::load_matcher).
#[derive(Debug, Clone)]
pub struct MatcherConfig<A> {
    /// A list of field matchers, or a lookup tree. Never both.
    pub kind: MatcherKindConfig<A>,

    /// Fallback when nothing matched.
    pub on_no_match: Option<OnMatchConfig<A>>,
}

impl<A> MatcherConfig<A> {
    /// A list matcher, which is what most configs are.
    #[must_use]
    pub fn list(matchers: Vec<FieldMatcherConfig<A>>) -> Self {
        Self {
            kind: MatcherKindConfig::List(matchers),
            on_no_match: None,
        }
    }

    /// Set the fallback.
    #[must_use]
    pub fn with_fallback(mut self, on_no_match: Option<OnMatchConfig<A>>) -> Self {
        self.on_no_match = on_no_match;
        self
    }
}

/// Config mirror of [`MatcherKind`](crate::MatcherKind) — xDS `oneof matcher_type`.
#[derive(Debug, Clone)]
pub enum MatcherKindConfig<A> {
    /// Field matchers evaluated in order.
    List(Vec<FieldMatcherConfig<A>>),
    /// A single map lookup.
    Tree(MatcherTreeConfig<A>),
}

/// Config for a [`MatcherTree`](crate::MatcherTree).
///
/// Carries no fallback: the proto `MatcherTree` has no such field and the
/// enclosing `Matcher` owns it. See `DECISIONS.md` D-044.
#[derive(Debug, Clone)]
pub struct MatcherTreeConfig<A> {
    /// The input producing the lookup key, resolved via the registry.
    pub input: TypedConfig,
    /// Which lookup rule applies, and the entries it applies to.
    pub tree: TreeTypeConfig<A>,
}

/// Config mirror of xDS `MatcherTree.tree_type`.
#[derive(Debug, Clone)]
pub enum TreeTypeConfig<A> {
    /// O(1) exact key lookup.
    ExactMatchMap(Vec<(String, OnMatchConfig<A>)>),
    /// O(k) longest-prefix lookup.
    PrefixMatchMap(Vec<(String, OnMatchConfig<A>)>),
}

/// Configuration for a [`FieldMatcher`](crate::FieldMatcher).
#[derive(Debug, Clone)]
pub struct FieldMatcherConfig<A> {
    /// The predicate that gates this field matcher.
    pub predicate: PredicateConfig,

    /// What to do when the predicate matches.
    pub on_match: OnMatchConfig<A>,
}

/// Configuration for a [`Predicate`](crate::Predicate).
///
/// Mirrors xDS `Matcher.MatcherList.Predicate`, whose `oneof match_type` is
/// these four arms.
#[derive(Debug, Clone)]
pub enum PredicateConfig {
    /// A single predicate: input + value match.
    Single(SinglePredicateConfig),

    /// All predicates must match (logical AND).
    And {
        /// Child predicates (all must match).
        predicates: Vec<PredicateConfig>,
    },

    /// Any predicate must match (logical OR).
    Or {
        /// Child predicates (any must match).
        predicates: Vec<PredicateConfig>,
    },

    /// Inverts the inner predicate (logical NOT).
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
    BuiltIn {
        /// The pattern and how to apply it.
        spec: StringMatchSpec,
        /// Case-insensitive matching — xDS `StringMatcher.ignore_case`.
        ///
        /// Carried here rather than inside `StringMatchSpec` because it is a
        /// property of the comparison, not of the pattern. The proto path used
        /// to drop it on the floor.
        ignore_case: bool,
    },
    /// Custom matcher resolved via the registry's matcher factories.
    Custom(TypedConfig),
}

/// Configuration for a [`SinglePredicate`](crate::SinglePredicate).
///
/// Combines a typed input reference (resolved via registry) with a value
/// matcher. Mirrors xDS `SinglePredicate`, whose `oneof matcher` is either a
/// built-in `StringMatcher` or a `TypedExtensionConfig`.
///
/// The exactly-one-of rule is enforced where the config is built: the proto
/// `oneof` makes both-set unrepresentable, and `rumi-proto`'s converter
/// rejects neither-set.
#[derive(Debug, Clone)]
pub struct SinglePredicateConfig {
    /// The input to extract data from context.
    /// Resolved at load time via the registry's `type_url` lookup.
    pub input: TypedConfig,

    /// How to match the extracted value.
    pub matcher: ValueMatchConfig,
}

/// Reference to a registered type with its configuration.
///
/// Maps to xDS `TypedExtensionConfig`:
/// - `type_url` identifies the registered type (input, matcher, or action)
/// - `config` carries the type-specific configuration payload
#[derive(Debug, Clone)]
pub struct TypedConfig {
    /// The type URL identifying the registered type.
    /// Must match a `type_url` registered in the [`Registry`](crate::Registry).
    pub type_url: String,

    /// Type-specific configuration payload.
    /// Deserialized as the `Config` associated type of the registered trait impl.
    pub config: serde_json::Value,
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
#[derive(Debug, Clone)]
pub enum OnMatchConfig<A> {
    /// Return this action when the predicate matches.
    Action {
        /// The action value.
        action: A,
    },

    /// Evaluate a nested matcher when the predicate matches.
    Matcher {
        /// The nested matcher configuration.
        matcher: Box<MatcherConfig<A>>,
    },
}

// The test module that lived here tested only the terse dialect's
// deserializer — `deserialize_simple_config`, `deserialize_and_predicate`,
// `deserialize_not_predicate`, `deserialize_nested_matcher`,
// `typed_config_defaults_to_empty_object`, `no_on_no_match_is_none`. With the
// impls gone their subject is gone; each requirement is now a protojson
// fixture in `spec/tests/07_protojson/`, asserted in all five implementations
// rather than one.
