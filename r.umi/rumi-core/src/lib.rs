//! rumi-core: Pure matcher engine for r.umi
//!
//! This crate provides the core types and traits for the xDS Unified Matcher API.
//! It is designed to be no_std compatible (with alloc) for embedded/WASM use cases.
//!
//! # Architecture (Envoy-inspired)
//!
//! The type system uses a hybrid erasure approach:
//!
//! - [`MatchingData`] — Erased data type (enum of String, Int, Bool, etc.)
//! - [`DataInput<Ctx>`] — Domain-specific extraction, returns `MatchingData`
//! - [`InputMatcher`] — Domain-agnostic matching (non-generic, shareable!)
//! - [`SinglePredicate<Ctx>`] — Combines DataInput + InputMatcher
//! - [`Predicate<Ctx>`] — Boolean composition (And, Or, Not)
//! - [`Matcher<Ctx, A>`] — Top-level matcher with first-match-wins semantics
//!
//! # Key Design Insights
//!
//! 1. **Type erasure at data level**: `MatchingData` is an enum, not a trait object.
//!    This allows `InputMatcher` to be non-generic.
//!
//! 2. **Non-generic InputMatcher**: The same `ExactMatcher` works for HTTP, Claude,
//!    test contexts, etc. Matchers are domain-agnostic.
//!
//! 3. **DataInput None → false**: When a DataInput returns `MatchingData::None`,
//!    the predicate evaluates to `false`. This is a critical invariant.
//!
//! # Example
//!
//! ```
//! use rumi_core::{
//!     DataInput, InputMatcher, MatchingData,
//!     ExactMatcher, SinglePredicate, Predicate,
//!     FieldMatcher, Matcher, OnMatch,
//! };
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

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

// ═══════════════════════════════════════════════════════════════════════════════
// Modules
// ═══════════════════════════════════════════════════════════════════════════════

mod matching_data;
mod data_input;
mod input_matcher;
mod predicate;
mod on_match;
mod field_matcher;
mod matcher;

// ═══════════════════════════════════════════════════════════════════════════════
// Public API
// ═══════════════════════════════════════════════════════════════════════════════

// Core types
pub use matching_data::MatchingData;
pub use data_input::DataInput;
pub use input_matcher::InputMatcher;
pub use predicate::{Predicate, SinglePredicate};
pub use on_match::OnMatch;
pub use field_matcher::FieldMatcher;
pub use matcher::Matcher;

// Concrete matchers
pub use input_matcher::{
    ExactMatcher,
    PrefixMatcher,
    SuffixMatcher,
    ContainsMatcher,
    BoolMatcher,
};

// ═══════════════════════════════════════════════════════════════════════════════
// Prelude
// ═══════════════════════════════════════════════════════════════════════════════

/// Prelude module for convenient imports.
///
/// ```
/// use rumi_core::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        // Traits
        DataInput,
        InputMatcher,
        // Core types
        MatchingData,
        Predicate,
        SinglePredicate,
        OnMatch,
        FieldMatcher,
        Matcher,
        // Concrete matchers
        ExactMatcher,
        PrefixMatcher,
        SuffixMatcher,
        ContainsMatcher,
        BoolMatcher,
    };
}

// ═══════════════════════════════════════════════════════════════════════════════
// Constants
// ═══════════════════════════════════════════════════════════════════════════════

/// Maximum allowed depth for nested matchers.
///
/// This limit protects against stack overflow from deeply nested predicates.
/// Depth should be validated at config load time, not evaluation time.
pub const MAX_DEPTH: usize = 32;
