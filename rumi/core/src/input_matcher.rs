//! `InputMatcher` — Domain-agnostic value matching
//!
//! The `InputMatcher` trait matches against type-erased `MatchingData`.
//! It is intentionally **non-generic** — matchers can be shared across
//! different context types. This is a key design insight from Envoy.
//!
//! # Available Matchers
//!
//! - [`ExactMatcher`] — Exact string equality
//! - [`PrefixMatcher`] — String prefix match
//! - [`SuffixMatcher`] — String suffix match
//! - [`ContainsMatcher`] — String contains match

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{boxed::Box, string::String};

#[cfg(feature = "std")]
use std::string::String;

use crate::MatchingData;
use core::fmt::Debug;

/// Matches against type-erased [`MatchingData`].
///
/// This trait is intentionally **non-generic**. `InputMatchers` operate on
/// erased data, which means the same matcher (e.g., `ExactMatcher`) can be
/// used across different context types (HTTP, Claude, test).
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to support concurrent evaluation
/// and FFI use cases.
///
/// # Example
///
/// ```
/// use rumi::{InputMatcher, MatchingData, ExactMatcher};
///
/// let matcher = ExactMatcher::new("hello");
/// assert!(matcher.matches(&MatchingData::String("hello".to_string())));
/// assert!(!matcher.matches(&MatchingData::String("world".to_string())));
/// ```
pub trait InputMatcher: Send + Sync + Debug {
    /// Check if the given value matches.
    ///
    /// Returns `false` if the value type is incompatible with this matcher.
    fn matches(&self, value: &MatchingData) -> bool;

    /// Returns the data types this matcher supports.
    ///
    /// Used for config-time validation. Default is `["string"]`.
    fn supported_types(&self) -> &[&'static str] {
        &["string"]
    }
}

// Blanket implementation for boxed InputMatchers
impl InputMatcher for Box<dyn InputMatcher> {
    fn matches(&self, value: &MatchingData) -> bool {
        (**self).matches(value)
    }

    fn supported_types(&self) -> &[&'static str] {
        (**self).supported_types()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// String Matchers
// ═══════════════════════════════════════════════════════════════════════════════

/// Exact string equality matcher.
///
/// Matches when the input string exactly equals the expected value.
///
/// # Example
///
/// ```
/// use rumi::{InputMatcher, MatchingData, ExactMatcher};
///
/// let matcher = ExactMatcher::new("hello");
/// assert!(matcher.matches(&"hello".into()));
/// assert!(!matcher.matches(&"Hello".into())); // case-sensitive
/// assert!(!matcher.matches(&"hello ".into())); // no trimming
/// ```
#[derive(Debug, Clone)]
pub struct ExactMatcher {
    expected: String,
}

impl ExactMatcher {
    /// Create a new exact matcher with the given expected value.
    pub fn new(expected: impl Into<String>) -> Self {
        Self {
            expected: expected.into(),
        }
    }

    /// Returns the expected value.
    #[must_use]
    pub fn expected(&self) -> &str {
        &self.expected
    }
}

impl InputMatcher for ExactMatcher {
    fn matches(&self, value: &MatchingData) -> bool {
        value.as_str().is_some_and(|s| s == self.expected)
    }
}

/// Prefix string matcher.
///
/// Matches when the input string starts with the specified prefix.
///
/// # Example
///
/// ```
/// use rumi::{InputMatcher, MatchingData, PrefixMatcher};
///
/// let matcher = PrefixMatcher::new("/api/");
/// assert!(matcher.matches(&"/api/users".into()));
/// assert!(matcher.matches(&"/api/".into()));
/// assert!(!matcher.matches(&"/users".into()));
/// ```
#[derive(Debug, Clone)]
pub struct PrefixMatcher {
    prefix: String,
}

impl PrefixMatcher {
    /// Create a new prefix matcher.
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    /// Returns the prefix being matched.
    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }
}

impl InputMatcher for PrefixMatcher {
    fn matches(&self, value: &MatchingData) -> bool {
        value.as_str().is_some_and(|s| s.starts_with(&self.prefix))
    }
}

/// Suffix string matcher.
///
/// Matches when the input string ends with the specified suffix.
///
/// # Example
///
/// ```
/// use rumi::{InputMatcher, MatchingData, SuffixMatcher};
///
/// let matcher = SuffixMatcher::new(".json");
/// assert!(matcher.matches(&"data.json".into()));
/// assert!(!matcher.matches(&"data.xml".into()));
/// ```
#[derive(Debug, Clone)]
pub struct SuffixMatcher {
    suffix: String,
}

impl SuffixMatcher {
    /// Create a new suffix matcher.
    pub fn new(suffix: impl Into<String>) -> Self {
        Self {
            suffix: suffix.into(),
        }
    }

    /// Returns the suffix being matched.
    #[must_use]
    pub fn suffix(&self) -> &str {
        &self.suffix
    }
}

impl InputMatcher for SuffixMatcher {
    fn matches(&self, value: &MatchingData) -> bool {
        value.as_str().is_some_and(|s| s.ends_with(&self.suffix))
    }
}

/// Contains string matcher.
///
/// Matches when the input string contains the specified substring.
///
/// # Example
///
/// ```
/// use rumi::{InputMatcher, MatchingData, ContainsMatcher};
///
/// let matcher = ContainsMatcher::new("error");
/// assert!(matcher.matches(&"connection error".into()));
/// assert!(matcher.matches(&"error: timeout".into()));
/// assert!(!matcher.matches(&"success".into()));
/// ```
#[derive(Debug, Clone)]
pub struct ContainsMatcher {
    substring: String,
}

impl ContainsMatcher {
    /// Create a new contains matcher.
    pub fn new(substring: impl Into<String>) -> Self {
        Self {
            substring: substring.into(),
        }
    }

    /// Returns the substring being searched for.
    #[must_use]
    pub fn substring(&self) -> &str {
        &self.substring
    }
}

impl InputMatcher for ContainsMatcher {
    fn matches(&self, value: &MatchingData) -> bool {
        value.as_str().is_some_and(|s| s.contains(&self.substring))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Boolean Matchers
// ═══════════════════════════════════════════════════════════════════════════════

/// Boolean equality matcher.
///
/// Matches when the input boolean equals the expected value.
///
/// # Example
///
/// ```
/// use rumi::{InputMatcher, MatchingData, BoolMatcher};
///
/// let matcher = BoolMatcher::new(true);
/// assert!(matcher.matches(&MatchingData::Bool(true)));
/// assert!(!matcher.matches(&MatchingData::Bool(false)));
/// ```
#[derive(Debug, Clone)]
pub struct BoolMatcher {
    expected: bool,
}

impl BoolMatcher {
    /// Create a new boolean matcher.
    #[must_use]
    pub fn new(expected: bool) -> Self {
        Self { expected }
    }
}

impl InputMatcher for BoolMatcher {
    fn matches(&self, value: &MatchingData) -> bool {
        value.as_bool().is_some_and(|b| b == self.expected)
    }

    fn supported_types(&self) -> &[&'static str] {
        &["bool"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_matcher() {
        let matcher = ExactMatcher::new("hello");
        assert!(matcher.matches(&"hello".into()));
        assert!(!matcher.matches(&"Hello".into()));
        assert!(!matcher.matches(&"hello world".into()));
        assert!(!matcher.matches(&MatchingData::None));
        assert!(!matcher.matches(&MatchingData::Int(42)));
    }

    #[test]
    fn test_prefix_matcher() {
        let matcher = PrefixMatcher::new("/api/");
        assert!(matcher.matches(&"/api/users".into()));
        assert!(matcher.matches(&"/api/".into()));
        assert!(!matcher.matches(&"/users".into()));
        assert!(!matcher.matches(&MatchingData::None));
    }

    #[test]
    fn test_suffix_matcher() {
        let matcher = SuffixMatcher::new(".json");
        assert!(matcher.matches(&"file.json".into()));
        assert!(matcher.matches(&".json".into()));
        assert!(!matcher.matches(&"file.xml".into()));
    }

    #[test]
    fn test_contains_matcher() {
        let matcher = ContainsMatcher::new("error");
        assert!(matcher.matches(&"an error occurred".into()));
        assert!(matcher.matches(&"error".into()));
        assert!(!matcher.matches(&"success".into()));
    }

    #[test]
    fn test_bool_matcher() {
        let matcher = BoolMatcher::new(true);
        assert!(matcher.matches(&MatchingData::Bool(true)));
        assert!(!matcher.matches(&MatchingData::Bool(false)));
        assert!(!matcher.matches(&MatchingData::String("true".into())));
    }

    #[test]
    fn test_matchers_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ExactMatcher>();
        assert_send_sync::<PrefixMatcher>();
        assert_send_sync::<SuffixMatcher>();
        assert_send_sync::<ContainsMatcher>();
        assert_send_sync::<BoolMatcher>();
        assert_send_sync::<Box<dyn InputMatcher>>();
    }
}
