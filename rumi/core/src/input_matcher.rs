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

use crate::MatchingData;
use std::fmt::Debug;

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
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `InputMatcher`",
    label = "this type cannot match against MatchingData",
    note = "InputMatcher is domain-agnostic — use built-in matchers (ExactMatcher, PrefixMatcher, StringMatcher, etc.) or implement the `matches(&self, &MatchingData) -> bool` method"
)]
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
#[diagnostic::do_not_recommend]
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

// ═══════════════════════════════════════════════════════════════════════════════
// StringMatcher (xDS Unified)
// ═══════════════════════════════════════════════════════════════════════════════

/// Unified string matcher matching xDS `StringMatcher` proto.
///
/// Combines all string matching strategies with optional case-insensitivity.
/// This is the xDS-native way to express string matching.
///
/// # Example
///
/// ```
/// use rumi::{InputMatcher, MatchingData, StringMatcher};
///
/// // Case-insensitive prefix match
/// let matcher = StringMatcher::prefix("/API/", true);
/// assert!(matcher.matches(&"/api/users".into()));
/// assert!(matcher.matches(&"/API/users".into()));
///
/// // Regex match
/// let matcher = StringMatcher::regex(r"^user-\d+$").unwrap();
/// assert!(matcher.matches(&"user-123".into()));
/// assert!(!matcher.matches(&"user-abc".into()));
/// ```
#[derive(Debug, Clone)]
pub enum StringMatcher {
    /// Exact string equality.
    Exact { value: String, ignore_case: bool },
    /// String prefix match.
    Prefix { value: String, ignore_case: bool },
    /// String suffix match.
    Suffix { value: String, ignore_case: bool },
    /// Substring contains match.
    Contains { value: String, ignore_case: bool },
    /// Regular expression match (RE2 semantics, linear time).
    Regex(regex::Regex),
}

impl StringMatcher {
    /// Create an exact match.
    #[must_use]
    pub fn exact(value: impl Into<String>, ignore_case: bool) -> Self {
        Self::Exact {
            value: value.into(),
            ignore_case,
        }
    }

    /// Create a prefix match.
    #[must_use]
    pub fn prefix(value: impl Into<String>, ignore_case: bool) -> Self {
        Self::Prefix {
            value: value.into(),
            ignore_case,
        }
    }

    /// Create a suffix match.
    #[must_use]
    pub fn suffix(value: impl Into<String>, ignore_case: bool) -> Self {
        Self::Suffix {
            value: value.into(),
            ignore_case,
        }
    }

    /// Create a contains match.
    ///
    /// When `ignore_case` is true, the pattern is pre-lowercased at construction
    /// to avoid redundant allocation per match call.
    #[must_use]
    pub fn contains(value: impl Into<String>, ignore_case: bool) -> Self {
        let value = value.into();
        Self::Contains {
            value: if ignore_case {
                value.to_ascii_lowercase()
            } else {
                value
            },
            ignore_case,
        }
    }

    /// Create a regex match.
    ///
    /// Uses Rust's `regex` crate which guarantees linear time matching (no `ReDoS`).
    ///
    /// # Errors
    ///
    /// Returns `Err` if the regex pattern is invalid.
    pub fn regex(pattern: &str) -> Result<Self, regex::Error> {
        regex::Regex::new(pattern).map(Self::Regex)
    }

    /// Create a case-insensitive regex match.
    ///
    /// Prepends `(?i)` to the pattern for case-insensitivity.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the regex pattern is invalid.
    pub fn regex_ignore_case(pattern: &str) -> Result<Self, regex::Error> {
        regex::Regex::new(&format!("(?i){pattern}")).map(Self::Regex)
    }
}

impl InputMatcher for StringMatcher {
    fn matches(&self, value: &MatchingData) -> bool {
        let Some(input) = value.as_str() else {
            return false;
        };

        match self {
            Self::Exact { value, ignore_case } => {
                if *ignore_case {
                    input.eq_ignore_ascii_case(value)
                } else {
                    input == value
                }
            }
            Self::Prefix { value, ignore_case } => {
                if *ignore_case {
                    input
                        .get(..value.len())
                        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(value))
                } else {
                    input.starts_with(value.as_str())
                }
            }
            Self::Suffix { value, ignore_case } => {
                if *ignore_case {
                    input
                        .len()
                        .checked_sub(value.len())
                        .and_then(|start| input.get(start..))
                        .is_some_and(|suffix| suffix.eq_ignore_ascii_case(value))
                } else {
                    input.ends_with(value.as_str())
                }
            }
            Self::Contains { value, ignore_case } => {
                if *ignore_case {
                    // value is pre-lowercased at construction time
                    input.to_ascii_lowercase().contains(value.as_str())
                } else {
                    input.contains(value.as_str())
                }
            }
            Self::Regex(re) => re.is_match(input),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// IntoInputMatcher impls (feature = "registry")
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "registry")]
mod into_input_matcher {
    use super::{BoolMatcher, InputMatcher, StringMatcher};
    use crate::registry::IntoInputMatcher;
    use crate::MatcherError;
    use serde::Deserialize;

    // ── BoolMatcher ──────────────────────────────────────────────────────────

    /// Configuration for constructing a [`BoolMatcher`] via the registry.
    #[derive(Debug, Clone, Deserialize)]
    pub struct BoolMatcherConfig {
        /// The boolean value to match against.
        pub expected: bool,
    }

    impl IntoInputMatcher for BoolMatcher {
        type Config = BoolMatcherConfig;

        fn from_config(config: Self::Config) -> Result<Box<dyn InputMatcher>, MatcherError> {
            Ok(Box::new(BoolMatcher::new(config.expected)))
        }
    }

    // ── StringMatcher ────────────────────────────────────────────────────────

    /// How to match the pattern in a [`StringMatcherConfig`].
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum StringMatchType {
        /// Exact string equality.
        Exact,
        /// String prefix match.
        Prefix,
        /// String suffix match.
        Suffix,
        /// Substring contains match.
        Contains,
        /// Regular expression match.
        Regex,
    }

    /// Configuration for constructing a [`StringMatcher`] via the registry.
    ///
    /// JSON example:
    /// ```json
    /// { "value": "/api", "match_type": "prefix", "ignore_case": true }
    /// ```
    #[derive(Debug, Clone, Deserialize)]
    pub struct StringMatcherConfig {
        /// The pattern to match against.
        pub value: String,
        /// How to match the pattern.
        pub match_type: StringMatchType,
        /// Case-insensitive matching (default: false).
        #[serde(default)]
        pub ignore_case: bool,
    }

    impl IntoInputMatcher for StringMatcher {
        type Config = StringMatcherConfig;

        fn from_config(config: Self::Config) -> Result<Box<dyn InputMatcher>, MatcherError> {
            let matcher = match config.match_type {
                StringMatchType::Exact => StringMatcher::exact(config.value, config.ignore_case),
                StringMatchType::Prefix => StringMatcher::prefix(config.value, config.ignore_case),
                StringMatchType::Suffix => StringMatcher::suffix(config.value, config.ignore_case),
                StringMatchType::Contains => {
                    StringMatcher::contains(config.value, config.ignore_case)
                }
                StringMatchType::Regex => if config.ignore_case {
                    StringMatcher::regex_ignore_case(&config.value)
                } else {
                    StringMatcher::regex(&config.value)
                }
                .map_err(|e| MatcherError::InvalidPattern {
                    pattern: config.value.clone(),
                    source: e.to_string(),
                })?,
            };
            Ok(Box::new(matcher))
        }
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
    fn test_string_matcher_exact() {
        let m = StringMatcher::exact("hello", false);
        assert!(m.matches(&"hello".into()));
        assert!(!m.matches(&"Hello".into()));

        let m = StringMatcher::exact("hello", true);
        assert!(m.matches(&"hello".into()));
        assert!(m.matches(&"HELLO".into()));
    }

    #[test]
    fn test_string_matcher_prefix() {
        let m = StringMatcher::prefix("/api/", false);
        assert!(m.matches(&"/api/users".into()));
        assert!(!m.matches(&"/API/users".into()));

        let m = StringMatcher::prefix("/api/", true);
        assert!(m.matches(&"/API/users".into()));
    }

    #[test]
    fn test_string_matcher_suffix() {
        let m = StringMatcher::suffix(".JSON", true);
        assert!(m.matches(&"file.json".into()));
        assert!(m.matches(&"file.JSON".into()));
    }

    #[test]
    fn test_string_matcher_contains() {
        let m = StringMatcher::contains("error", true);
        assert!(m.matches(&"an ERROR occurred".into()));
    }

    #[test]
    fn test_string_matcher_regex() {
        let m = StringMatcher::regex(r"^user-\d+$").unwrap();
        assert!(m.matches(&"user-123".into()));
        assert!(!m.matches(&"user-abc".into()));
        assert!(!m.matches(&"USER-123".into()));

        let m = StringMatcher::regex_ignore_case(r"^user-\d+$").unwrap();
        assert!(m.matches(&"USER-123".into()));
    }
}
