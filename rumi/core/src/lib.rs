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
//! Domain-specific functionality is provided by extension crates:
//!
//! - [`rumi-test`](https://docs.rs/rumi-test) — Test domain for conformance
//! - [`rumi-http`](https://docs.rs/rumi-http) — HTTP request matching
//! - [`rumi-claude`](https://docs.rs/rumi-claude) — Claude Code hooks

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
        }
    }
}

impl std::error::Error for MatcherError {}
