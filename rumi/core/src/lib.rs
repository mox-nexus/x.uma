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
//! 3. **`DataInput` None → false**: When a `DataInput` returns `None`, the
//!    predicate evaluates to `false` and the matcher is never asked (INV-1).
//!    The signature enforces it: absence is not a `MatchingData` value.
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
//!     fn get(&self, ctx: &Request) -> Option<MatchingData> {
//!         Some(MatchingData::String(ctx.path.clone()))
//!     }
//! }
//!
//! // Build a matcher (OnMatch is an enum: Action or Matcher, per xDS proto)
//! let matcher: Matcher<Request, String> = Matcher::list(
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
mod ir;
#[cfg(feature = "registry")]
mod registry;

pub use data_input::DataInput;
pub use field_matcher::FieldMatcher;
pub use input_matcher::InputMatcher;
pub use matcher::{Matcher, MatcherKind, Outcome};
pub use matcher_tree::MatcherTree;
pub use matching_data::{CustomMatchData, MatchingData};
pub use on_match::OnMatch;
pub use predicate::{MatchResult, Predicate, SinglePredicate};
pub use radix_tree::RadixTree;
pub use string_match::StringMatchSpec;

#[cfg(feature = "registry")]
pub use ir::{
    FieldMatcherIr, MatcherIr, MatcherKindIr, MatcherTreeIr, OnMatchIr, PredicateIr,
    SinglePredicateIr, TreeTypeIr, TypedExtensionConfig, UnitConfig, ValueMatchIr,
};
#[cfg(feature = "registry")]
pub use registry::{
    register_core_matchers, ActionFactory, ActionRegistry, ActionRegistryBuilder, DataInputFactory,
    InputMatcherFactory, Registry, RegistryBuilder,
};

pub use trace::{
    EvalStep, EvalSteps, EvalTrace, OnMatchTrace, PredicateTrace, TreeAttempt, TreeKind,
    TreeLookupTrace,
};

pub use input_matcher::{
    BoolMatcher, ContainsMatcher, ExactMatcher, PrefixMatcher, StringMatcher, SuffixMatcher,
};

/// Prelude module for convenient imports.
///
/// ```
/// use rumi::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        BoolMatcher, ContainsMatcher, CustomMatchData, DataInput, EvalStep, EvalTrace,
        ExactMatcher, FieldMatcher, InputMatcher, MatchResult, Matcher, MatcherError, MatcherTree,
        MatchingData, OnMatch, OnMatchTrace, Outcome, Predicate, PredicateTrace, PrefixMatcher,
        RadixTree, SinglePredicate, StringMatchSpec, StringMatcher, SuffixMatcher,
    };

    /// Registry types, so `use rumi::prelude::*` is enough to load a config.
    ///
    /// `MatcherIr` is deliberately absent: the config format is moving to
    /// protojson and the hand-written type is being retired, so exporting it
    /// here would add a name in one release and remove it in the next. Import
    /// it directly as `rumi::MatcherIr` until the replacement lands.
    #[cfg(feature = "registry")]
    pub use crate::{
        DataInputFactory, InputMatcherFactory, Registry, RegistryBuilder, TypedExtensionConfig,
        UnitConfig,
    };
}

/// Maximum allowed depth for nested matchers.
///
/// This limit protects against stack overflow from deeply nested predicates.
/// Validate at config load time via [`Matcher::validate`].
pub const MAX_DEPTH: usize = 32;

/// Predicates one evaluation can reach, along the worst path through the matcher.
///
/// Width is a denial-of-service axis that depth does not cover, and every
/// predicate a request can reach is a per-request cost: all of a list's rules,
/// since a nested miss moves on to the next; one entry of an exact tree; a
/// chain of prefixes, since a miss falls through to the next shorter one
/// (D-055). Entries a request never visits do not count, which is what lets a
/// tree scale where a list cannot (D-068).
pub const MAX_PREDICATES_PER_EVALUATION: usize = 256;

/// Maximum entries in a single [`MatcherTree`](crate::MatcherTree).
///
/// A tree is O(1) hash or O(k) in the key's length, so its entry count costs
/// nothing at evaluation time, and large routing tables are the entire reason
/// to reach for a tree. What its width does cost is memory at config load,
/// which is what this bounds, and why it is far larger than
/// [`MAX_PREDICATES_PER_EVALUATION`].
pub const MAX_TREE_ENTRIES: usize = 65_536;

/// Maximum length for non-regex string match patterns (exact, prefix, suffix, contains).
pub const MAX_PATTERN_LENGTH: usize = 8192;

/// Maximum length for regex patterns.
///
/// Regex compilation is expensive even with the linear-time Rust `regex` crate.
/// Shorter limit than [`MAX_PATTERN_LENGTH`] because regex complexity scales
/// faster than literal matching.
pub const MAX_REGEX_PATTERN_LENGTH: usize = 4096;

/// Predicates in one document, counted across every nested matcher and tree
/// entry.
///
/// A memory bound, not a per-request one: that is
/// [`MAX_PREDICATES_PER_EVALUATION`]. The registry counts as it loads and
/// refuses at the predicate that crosses, so a document whose size is not
/// bounded in bytes (IR deserialised from YAML with aliases, say) is refused
/// before it is materialised (D-068).
pub const MAX_DOCUMENT_PREDICATES: usize = 65_536;

/// Compiled regexes in one document.
///
/// Each regex holds up to [`REGEX_SIZE_LIMIT`] of program and
/// [`REGEX_DFA_SIZE_LIMIT`] of lazy DFA cache per thread, so the count is
/// what bounds the document's regex memory: 64 × 2 MiB. Counted by the
/// registry during load; the domain compilers are bounded by their own rule
/// counts (D-054).
pub const MAX_REGEXES: usize = 64;

/// Compiled program size per regex, in bytes.
pub const REGEX_SIZE_LIMIT: usize = 1 << 20;

/// Lazy DFA cache per regex per thread, in bytes.
pub const REGEX_DFA_SIZE_LIMIT: usize = 1 << 20;

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
    /// More predicates on one evaluation's worst path than
    /// [`MAX_PREDICATES_PER_EVALUATION`].
    TooManyPredicatesPerEvaluation {
        /// The worst path's predicate count.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// More predicates in the document than [`MAX_DOCUMENT_PREDICATES`].
    TooManyDocumentPredicates {
        /// The predicate that crossed the budget.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// More regexes in the document than [`MAX_REGEXES`].
    TooManyRegexes {
        /// The regex that crossed the budget.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// A config document larger than a reader accepts.
    DocumentTooLarge {
        /// The document's size.
        bytes: usize,
        /// Maximum accepted.
        max: usize,
    },
    /// An identifier that names what to look up was empty.
    ///
    /// Header names, query parameter names, tool argument names and map keys
    /// all identify *where* to read a value. An empty one reads nothing, which
    /// makes the predicate false and a deny rule silently stop firing. That is
    /// a fail-open, so it is rejected where the value is constructed rather
    /// than tolerated and discovered in production.
    EmptyIdentifier {
        /// What was being named, e.g. `"header name"`.
        what: &'static str,
    },
    /// Two entries of a `MatcherTree` were configured under the same key.
    ///
    /// A map silently keeps the last writer, so one of the two rules would
    /// vanish without a word. On a config path that is a rule the author
    /// believes is in force and is not.
    DuplicateTreeKey {
        /// The key configured more than once.
        key: String,
    },
    /// Too many entries in a single `MatcherTree`.
    TooManyTreeEntries {
        /// Actual count of entries.
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
            Self::TooManyPredicatesPerEvaluation { count, max } => {
                write!(
                    f,
                    "matcher can reach {count} predicates in one evaluation, but maximum \
                     allowed is {max} — key large rule sets on a field with a matcher tree"
                )
            }
            Self::TooManyDocumentPredicates { count, max } => {
                write!(
                    f,
                    "document has {count} predicates, but maximum allowed is {max}"
                )
            }
            Self::TooManyRegexes { count, max } => {
                write!(
                    f,
                    "matcher compiles {count} regexes, but maximum allowed is {max}"
                )
            }
            Self::DocumentTooLarge { bytes, max } => {
                write!(
                    f,
                    "config document is {bytes} bytes, but maximum accepted is {max}"
                )
            }
            Self::DuplicateTreeKey { key } => {
                write!(
                    f,
                    "matcher tree has more than one entry for key '{key}'; \
                     one of them would never be reachable"
                )
            }
            Self::TooManyTreeEntries { count, max } => {
                write!(f, "matcher tree has {count} entries, maximum is {max}")
            }
            Self::EmptyIdentifier { what } => {
                write!(f, "{what} must not be empty")
            }
            Self::PatternTooLong { len, max } => {
                write!(f, "pattern length is {len}, but maximum allowed is {max}")
            }
        }
    }
}

impl std::error::Error for MatcherError {}

/// The crate README's Rust blocks, compiled and run as doctests.
///
/// A crate README is a published artifact — it is the crates.io front page —
/// and until 2026-08-18 nothing checked one. `rumi-core`'s showed a config
/// dialect that had been retired, so the first thing a new reader would have
/// copied could not load. Nothing failed, because nothing looked.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;
