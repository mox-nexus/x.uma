//! rumi - Rust implementation of xDS Unified Matcher API
//!
//! A matcher engine implementing the xDS Unified Matcher API specification.
//!
//! # Architecture (Envoy-inspired)
//!
//! The type system uses a hybrid erasure approach:
//!
//! - [`MatchingData`] — Erased data type (primitives + extensible Custom variant)
//! - [`DataInput<Ctx>`] — Domain-specific extraction, returns `MatchingData`
//! - [`InputMatcher`] — Domain-agnostic matching (non-generic, shareable!)
//! - [`SinglePredicate<Ctx>`] — Combines `DataInput` + `InputMatcher`
//! - [`Predicate<Ctx>`] — Boolean composition (And, Or, Not)
//! - [`Matcher<Ctx, A>`] — Top-level matcher with first-match-wins semantics
//!
//! # Key Design Insights
//!
//! 1. **Type erasure at data level**: `MatchingData` enables `InputMatcher` to be non-generic.
//!
//! 2. **Non-generic `InputMatcher`**: The same `ExactMatcher` works for HTTP, Claude,
//!    test contexts, etc. Matchers are domain-agnostic.
//!
//! 3. **`DataInput` None → false**: When a `DataInput` returns `MatchingData::None`,
//!    the predicate evaluates to `false`. This is a critical invariant.
//!
//! # Example
//!
//! ```
//! use rumi::prelude::*;
//!
//! // Define a context
//! #[derive(Debug)]
//! struct Request { path: String }
//!
//! // Define a DataInput
//! #[derive(Debug)]
//! struct PathInput;
//!
//! impl DataInput<Request> for PathInput {
//!     fn get(&self, ctx: &Request) -> MatchingData {
//!         MatchingData::String(ctx.path.clone())
//!     }
//! }
//!
//! // Build a matcher (OnMatch is an enum: Action or Matcher, per xDS proto)
//! let matcher: Matcher<Request, String> = Matcher::new(
//!     vec![
//!         FieldMatcher::new(
//!             Predicate::Single(SinglePredicate::new(
//!                 Box::new(PathInput),
//!                 Box::new(ExactMatcher::new("/api")),
//!             )),
//!             OnMatch::Action("api_backend".to_string()),
//!         ),
//!     ],
//!     Some(OnMatch::Action("default_backend".to_string())),
//! );
//!
//! // Evaluate
//! let result = matcher.evaluate(&Request { path: "/api".to_string() });
//! assert_eq!(result, Some("api_backend".to_string()));
//! ```
//!
//! # Extensions
//!
//! Domain-specific functionality:
//!
//! - [`claude`] — Claude Code hooks (feature = `"claude"`)
//! - [`rumi-http`](https://docs.rs/rumi-http) — HTTP request matching (separate crate)
//! - [`rumi-test`](https://docs.rs/rumi-test) — Test domain for conformance (internal)

// ═══════════════════════════════════════════════════════════════════════════════
// Modules
// ═══════════════════════════════════════════════════════════════════════════════

mod data_input;
mod field_matcher;
mod input_matcher;
mod matcher;
mod matcher_tree;
mod matching_data;
mod on_match;
mod predicate;
mod radix_tree;
mod string_match;
mod trace;

#[cfg(feature = "claude")]
pub mod claude;

#[cfg(feature = "registry")]
mod config;
#[cfg(feature = "registry")]
mod registry;

// ═══════════════════════════════════════════════════════════════════════════════
// Public API
// ═══════════════════════════════════════════════════════════════════════════════

// Core types
pub use data_input::DataInput;
pub use field_matcher::FieldMatcher;
pub use input_matcher::InputMatcher;
pub use matcher::Matcher;
pub use matcher_tree::MatcherTree;
pub use matching_data::{CustomMatchData, MatchingData};
pub use on_match::OnMatch;
pub use predicate::{Predicate, SinglePredicate};
pub use radix_tree::RadixTree;
pub use string_match::StringMatchSpec;

// Registry (feature-gated)
#[cfg(feature = "registry")]
pub use config::{
    FieldMatcherConfig, MatcherConfig, OnMatchConfig, PredicateConfig, SinglePredicateConfig,
    TypedConfig, UnitConfig, ValueMatchConfig,
};
#[cfg(feature = "registry")]
pub use registry::{
    register_core_matchers, ActionRegistry, ActionRegistryBuilder, IntoAction, IntoDataInput,
    IntoInputMatcher, Registry, RegistryBuilder,
};

// Trace types
pub use trace::{EvalStep, EvalTrace, OnMatchTrace, PredicateTrace};

// Concrete matchers
pub use input_matcher::{
    BoolMatcher, ContainsMatcher, ExactMatcher, PrefixMatcher, StringMatcher, SuffixMatcher,
};

// ═══════════════════════════════════════════════════════════════════════════════
// Prelude
// ═══════════════════════════════════════════════════════════════════════════════

/// Prelude module for convenient imports.
///
/// ```
/// use rumi::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        // Concrete matchers
        BoolMatcher,
        ContainsMatcher,
        // Traits
        CustomMatchData,
        DataInput,
        // Trace types
        EvalStep,
        EvalTrace,
        ExactMatcher,
        // Core types
        FieldMatcher,
        InputMatcher,
        Matcher,
        // Errors
        MatcherError,
        MatcherTree,
        MatchingData,
        OnMatch,
        OnMatchTrace,
        Predicate,
        PredicateTrace,
        PrefixMatcher,
        RadixTree,
        SinglePredicate,
        // Config types
        StringMatchSpec,
        StringMatcher,
        SuffixMatcher,
    };
}

// ═══════════════════════════════════════════════════════════════════════════════
// Constants
// ═══════════════════════════════════════════════════════════════════════════════

/// Maximum allowed depth for nested matchers.
///
/// This limit protects against stack overflow from deeply nested predicates.
/// Validate at config load time via [`Matcher::validate`].
pub const MAX_DEPTH: usize = 32;

/// Maximum number of field matchers in a single [`Matcher`].
///
/// Prevents width-based denial-of-service: a config with millions of field matchers at depth 1
/// bypasses [`MAX_DEPTH`] but still causes excessive resource consumption.
pub const MAX_FIELD_MATCHERS: usize = 256;

/// Maximum number of predicates in a single `And` or `Or` compound predicate.
///
/// Same width-based denial-of-service protection as [`MAX_FIELD_MATCHERS`], applied to
/// compound predicate children.
pub const MAX_PREDICATES_PER_COMPOUND: usize = 256;

/// Maximum length for non-regex string match patterns (exact, prefix, suffix, contains).
pub const MAX_PATTERN_LENGTH: usize = 8192;

/// Maximum length for regex patterns.
///
/// Regex compilation is expensive even with the linear-time Rust `regex` crate.
/// Shorter limit than [`MAX_PATTERN_LENGTH`] because regex complexity scales
/// faster than literal matching.
pub const MAX_REGEX_PATTERN_LENGTH: usize = 4096;

// ═══════════════════════════════════════════════════════════════════════════════
// Errors
// ═══════════════════════════════════════════════════════════════════════════════

/// Errors from matcher construction and validation.
///
/// These errors are caught at config load time, not evaluation time.
/// Fix the configuration and reconstruct the matcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatcherError {
    /// Matcher nesting exceeds [`MAX_DEPTH`].
    DepthExceeded {
        /// Actual depth of the matcher tree.
        depth: usize,
        /// Maximum allowed depth.
        max: usize,
    },
    /// A regex or string pattern is invalid.
    InvalidPattern {
        /// The pattern that failed to compile.
        pattern: String,
        /// The underlying error message.
        source: String,
    },
    /// Configuration deserialization or construction failed.
    InvalidConfig {
        /// The underlying error message.
        source: String,
    },
    /// A type URL was not found in the registry.
    UnknownTypeUrl {
        /// The unregistered type URL.
        type_url: String,
        /// Which registry was searched (`"input"`, `"matcher"`, `"action"`, or `"any_resolver"`).
        registry: &'static str,
        /// Type URLs that ARE registered (for self-correcting error messages).
        available: Vec<String>,
    },
    /// Input data type is incompatible with matcher's supported types.
    IncompatibleTypes {
        /// The data type produced by the input.
        input_type: String,
        /// The data types accepted by the matcher.
        matcher_types: Vec<String>,
    },
    /// Too many field matchers in a single `Matcher`.
    TooManyFieldMatchers {
        /// Actual count of field matchers.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Too many predicates in a compound `And` or `Or`.
    TooManyPredicates {
        /// Actual count of predicates.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// A string match pattern exceeds the maximum allowed length.
    PatternTooLong {
        /// Actual length of the pattern.
        len: usize,
        /// Maximum allowed length.
        max: usize,
    },
}

impl std::fmt::Display for MatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DepthExceeded { depth, max } => {
                write!(
                    f,
                    "matcher nesting depth is {depth}, but maximum allowed is {max} \
                     — reduce nesting or flatten your matcher tree"
                )
            }
            Self::InvalidPattern { pattern, source } => {
                write!(f, "invalid pattern \"{pattern}\": {source}")
            }
            Self::InvalidConfig { source } => {
                write!(f, "invalid config: {source}")
            }
            Self::UnknownTypeUrl {
                type_url,
                registry,
                available,
            } => {
                write!(f, "unknown {registry} type URL \"{type_url}\"")?;
                if available.is_empty() {
                    write!(f, " — no {registry} types are registered")
                } else {
                    write!(f, " — registered: {}", available.join(", "))
                }
            }
            Self::IncompatibleTypes {
                input_type,
                matcher_types,
            } => {
                write!(
                    f,
                    "input produces \"{input_type}\" data but matcher supports {matcher_types:?}"
                )
            }
            Self::TooManyFieldMatchers { count, max } => {
                write!(
                    f,
                    "matcher has {count} field matchers, but maximum allowed is {max}"
                )
            }
            Self::TooManyPredicates { count, max } => {
                write!(
                    f,
                    "compound predicate has {count} children, but maximum allowed is {max}"
                )
            }
            Self::PatternTooLong { len, max } => {
                write!(f, "pattern length is {len}, but maximum allowed is {max}")
            }
        }
    }
}

impl std::error::Error for MatcherError {}
