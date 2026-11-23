//! The matcher IR: what a protojson reader builds and the registry loads.
//!
//! These mirror the runtime matcher types and are what
//! [`Registry::load_matcher()`] consumes. They are **built, not parsed**: the
//! config format is canonical protojson, read by `rumi-proto` into xDS proto
//! types and lowered to these.
//!
//! None of the tree-shape types below derives `Deserialize`, and that is the
//! enforcement rather than a style choice. While they did, a second, terser
//! JSON dialect remained authorable in Rust — one no fixture used, no other
//! implementation could read, and nothing documented. Removing the impl makes
//! writing that dialect a compile error instead of a convention.
//!
//! The payload types keep theirs: a `TypedExtensionConfig.config` body is deserialized
//! as the registered factory's `Config` associated type, which is the
//! extension seam working as designed.
//!
//! # Relationship to runtime types
//!
//! | IR type | Runtime type | Loader method |
//! |-------------|-------------|---------------|
//! | [`MatcherIr`] | [`Matcher`](crate::Matcher) | `Registry::load_matcher()` |
//! | [`FieldMatcherIr`] | [`FieldMatcher`](crate::FieldMatcher) | `Registry::load_field_matcher()` |
//! | [`PredicateIr`] | [`Predicate`](crate::Predicate) | `Registry::load_predicate()` |
//! | [`SinglePredicateIr`] | [`SinglePredicate`](crate::SinglePredicate) | `Registry::load_single()` |
//! | [`ValueMatchIr`] | `Box<dyn InputMatcher>` | built-in or via registry factory |
//! | [`OnMatchIr`] | [`OnMatch`](crate::OnMatch) | `Registry::load_on_match()` |
//! | [`TypedExtensionConfig`] | `Box<dyn DataInput<Ctx>>` or `Box<dyn InputMatcher>` | via registry factory |

use crate::StringMatchSpec;
use serde::Deserialize;

/// The IR of a [`Matcher`](crate::Matcher).
///
/// Built by a protojson reader, then loaded into a runtime `Matcher` via
/// [`Registry::load_matcher()`](crate::Registry::load_matcher).
#[derive(Debug, Clone)]
pub struct MatcherIr<A> {
    /// A list of field matchers, or a lookup tree. Never both.
    pub kind: MatcherKindIr<A>,

    /// Fallback when nothing matched.
    pub on_no_match: Option<OnMatchIr<A>>,
}

impl<A> MatcherIr<A> {
    /// A list matcher, which is what most configs are.
    #[must_use]
    pub fn list(matchers: Vec<FieldMatcherIr<A>>) -> Self {
        Self {
            kind: MatcherKindIr::List(matchers),
            on_no_match: None,
        }
    }

    /// Set the fallback.
    #[must_use]
    pub fn with_fallback(mut self, on_no_match: Option<OnMatchIr<A>>) -> Self {
        self.on_no_match = on_no_match;
        self
    }
}

/// The IR of [`MatcherKind`](crate::MatcherKind): xDS `oneof matcher_type`.
#[derive(Debug, Clone)]
pub enum MatcherKindIr<A> {
    /// Field matchers evaluated in order.
    List(Vec<FieldMatcherIr<A>>),
    /// A single map lookup.
    Tree(MatcherTreeIr<A>),
}

/// The IR of a [`MatcherTree`](crate::MatcherTree).
///
/// Carries no fallback: the proto `MatcherTree` has no such field and the
/// enclosing `Matcher` owns it. See `DECISIONS.md` D-044.
#[derive(Debug, Clone)]
pub struct MatcherTreeIr<A> {
    /// The input producing the lookup key, resolved via the registry.
    pub input: TypedExtensionConfig,
    /// Which lookup rule applies, and the entries it applies to.
    pub tree: TreeTypeIr<A>,
}

/// The IR of xDS `MatcherTree.tree_type`.
#[derive(Debug, Clone)]
pub enum TreeTypeIr<A> {
    /// O(1) exact key lookup.
    ExactMatchMap(Vec<(String, OnMatchIr<A>)>),
    /// O(k) longest-prefix lookup.
    PrefixMatchMap(Vec<(String, OnMatchIr<A>)>),
}

/// The IR of a [`FieldMatcher`](crate::FieldMatcher).
#[derive(Debug, Clone)]
pub struct FieldMatcherIr<A> {
    /// The predicate that gates this field matcher.
    pub predicate: PredicateIr,

    /// What to do when the predicate matches.
    pub on_match: OnMatchIr<A>,
}

/// The IR of a [`Predicate`](crate::Predicate).
///
/// Mirrors xDS `Matcher.MatcherList.Predicate`, whose `oneof match_type` is
/// these four arms.
#[derive(Debug, Clone)]
pub enum PredicateIr {
    /// A single predicate: input + value match.
    Single(SinglePredicateIr),

    /// All predicates must match (logical AND).
    And {
        /// Child predicates (all must match).
        predicates: Vec<PredicateIr>,
    },

    /// Any predicate must match (logical OR).
    Or {
        /// Child predicates (any must match).
        predicates: Vec<PredicateIr>,
    },

    /// Inverts the inner predicate (logical NOT).
    Not {
        /// The predicate to negate.
        predicate: Box<PredicateIr>,
    },
}

/// How to match the extracted value in a [`SinglePredicateIr`].
///
/// Mirrors Envoy's `oneof matcher` in `SinglePredicate`:
/// - `BuiltIn` — built-in string matching (Envoy: `StringMatcher value_match`)
/// - `Custom` — custom matcher via registry (Envoy: `TypedExtensionConfig custom_match`)
///
/// The enum makes illegal states unrepresentable: exactly one variant is active.
#[derive(Debug, Clone)]
pub enum ValueMatchIr {
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
    Custom(TypedExtensionConfig),
}

/// The IR of a [`SinglePredicate`](crate::SinglePredicate).
///
/// Combines a typed input reference (resolved via registry) with a value
/// matcher. Mirrors xDS `SinglePredicate`, whose `oneof matcher` is either a
/// built-in `StringMatcher` or a `TypedExtensionConfig`.
///
/// The exactly-one-of rule is enforced where the config is built: the proto
/// `oneof` makes both-set unrepresentable, and `rumi-proto`'s converter
/// rejects neither-set.
#[derive(Debug, Clone)]
pub struct SinglePredicateIr {
    /// The input to extract data from context.
    /// Resolved at load time via the registry's `type_url` lookup.
    pub input: TypedExtensionConfig,

    /// How to match the extracted value.
    pub matcher: ValueMatchIr,
}

/// Reference to a registered type with its configuration.
///
/// Maps to xDS `TypedExtensionConfig`:
/// - `type_url` identifies the registered type (input, matcher, or action)
/// - `config` carries the type-specific configuration payload
#[derive(Debug, Clone)]
pub struct TypedExtensionConfig {
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
/// Use as the `Config` associated type in [`DataInputFactory`](crate::DataInputFactory)
/// for inputs that are self-contained (no construction parameters).
#[derive(Debug, Clone, Copy)]
pub struct UnitConfig;

impl<'de> Deserialize<'de> for UnitConfig {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        serde::de::IgnoredAny::deserialize(deserializer)?;
        Ok(UnitConfig)
    }
}

/// The IR of an [`OnMatch`](crate::OnMatch).
///
/// Either an action (leaf) or a nested matcher (tree).
/// `OnMatch` exclusivity is enforced by the enum: action XOR matcher.
#[derive(Debug, Clone)]
pub enum OnMatchIr<A> {
    /// Return this action when the predicate matches.
    Action {
        /// The action value.
        action: A,
    },

    /// Evaluate a nested matcher when the predicate matches.
    Matcher {
        /// The nested matcher configuration.
        matcher: Box<MatcherIr<A>>,
    },
}
