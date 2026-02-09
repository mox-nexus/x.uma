//! User-facing configuration types for Claude Code hook matching.
//!
//! These types provide a declarative API for specifying match rules.
//! The [compiler](crate::compiler) translates them into runtime `Matcher` trees.

use crate::context::HookEvent;
use rumi::prelude::*;
use std::fmt;

/// How to match a string value.
#[derive(Debug, Clone)]
pub enum StringMatch {
    /// Exact equality.
    Exact(String),
    /// Starts with prefix.
    Prefix(String),
    /// Ends with suffix.
    Suffix(String),
    /// Contains substring.
    Contains(String),
    /// Matches regular expression (Rust `regex` crate syntax).
    Regex(String),
}

impl StringMatch {
    /// Compile this match config into a runtime `InputMatcher`.
    ///
    /// # Errors
    ///
    /// Returns `CompileError` if the pattern is an invalid regular expression.
    pub fn to_input_matcher(&self) -> Result<Box<dyn InputMatcher>, CompileError> {
        match self {
            Self::Exact(v) => Ok(Box::new(ExactMatcher::new(v.as_str()))),
            Self::Prefix(v) => Ok(Box::new(PrefixMatcher::new(v.as_str()))),
            Self::Suffix(v) => Ok(Box::new(SuffixMatcher::new(v.as_str()))),
            Self::Contains(v) => Ok(Box::new(ContainsMatcher::new(v.as_str()))),
            Self::Regex(v) => StringMatcher::regex(v)
                .map(|sm| Box::new(sm) as Box<dyn InputMatcher>)
                .map_err(|e| CompileError(format!("invalid regex `{v}`: {e}"))),
        }
    }
}

impl fmt::Display for StringMatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(v) => write!(f, "Exact(\"{v}\")"),
            Self::Prefix(v) => write!(f, "Prefix(\"{v}\")"),
            Self::Suffix(v) => write!(f, "Suffix(\"{v}\")"),
            Self::Contains(v) => write!(f, "Contains(\"{v}\")"),
            Self::Regex(v) => write!(f, "Regex(\"{v}\")"),
        }
    }
}

/// User-friendly configuration for matching Claude Code hook events.
///
/// All fields are optional. Omitted fields match anything.
/// All present fields are `ANDed` (every condition must match).
///
/// # Example
///
/// ```ignore
/// use rumi_claude::prelude::*;
///
/// let rule = HookMatch {
///     event: Some(HookEvent::PreToolUse),
///     tool_name: Some(StringMatch::Exact("Bash".into())),
///     arguments: Some(vec![
///         ArgumentMatch {
///             name: "command".into(),
///             value: StringMatch::Contains("rm -rf".into()),
///         },
///     ]),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct HookMatch {
    /// Match on hook event type (exact match).
    pub event: Option<HookEvent>,
    /// Match on tool name.
    pub tool_name: Option<StringMatch>,
    /// Match on specific tool arguments (all `ANDed`).
    pub arguments: Option<Vec<ArgumentMatch>>,
    /// Match on current working directory.
    pub cwd: Option<StringMatch>,
    /// Match on git branch.
    pub git_branch: Option<StringMatch>,
}

/// Match a specific tool argument by name.
#[derive(Debug, Clone)]
pub struct ArgumentMatch {
    /// The argument name to extract.
    pub name: String,
    /// How to match the argument's value.
    pub value: StringMatch,
}

/// Error when compiling a `HookMatch` into a `Matcher`.
#[derive(Debug)]
pub struct CompileError(pub(crate) String);

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hook match compile error: {}", self.0)
    }
}

impl std::error::Error for CompileError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_match_exact_compiles() {
        let m = StringMatch::Exact("hello".into());
        let matcher = m.to_input_matcher().unwrap();
        assert!(matcher.matches(&MatchingData::String("hello".into())));
        assert!(!matcher.matches(&MatchingData::String("world".into())));
    }

    #[test]
    fn string_match_prefix_compiles() {
        let m = StringMatch::Prefix("/api".into());
        let matcher = m.to_input_matcher().unwrap();
        assert!(matcher.matches(&MatchingData::String("/api/users".into())));
        assert!(!matcher.matches(&MatchingData::String("/other".into())));
    }

    #[test]
    fn string_match_suffix_compiles() {
        let m = StringMatch::Suffix(".rs".into());
        let matcher = m.to_input_matcher().unwrap();
        assert!(matcher.matches(&MatchingData::String("main.rs".into())));
        assert!(!matcher.matches(&MatchingData::String("main.py".into())));
    }

    #[test]
    fn string_match_contains_compiles() {
        let m = StringMatch::Contains("rm -rf".into());
        let matcher = m.to_input_matcher().unwrap();
        assert!(matcher.matches(&MatchingData::String("sudo rm -rf /".into())));
        assert!(!matcher.matches(&MatchingData::String("ls -la".into())));
    }

    #[test]
    fn string_match_regex_compiles() {
        let m = StringMatch::Regex(r"^mcp__.*__delete".into());
        let matcher = m.to_input_matcher().unwrap();
        assert!(matcher.matches(&MatchingData::String("mcp__db__delete_row".into())));
        assert!(!matcher.matches(&MatchingData::String("Write".into())));
    }

    #[test]
    fn string_match_invalid_regex_returns_error() {
        let m = StringMatch::Regex("[invalid".into());
        let err = m.to_input_matcher().unwrap_err();
        assert!(err.to_string().contains("invalid regex"));
    }

    #[test]
    fn hook_match_default_is_empty() {
        let m = HookMatch::default();
        assert!(m.event.is_none());
        assert!(m.tool_name.is_none());
        assert!(m.arguments.is_none());
        assert!(m.cwd.is_none());
        assert!(m.git_branch.is_none());
    }

    #[test]
    fn string_match_display() {
        assert_eq!(
            StringMatch::Exact("Bash".into()).to_string(),
            r#"Exact("Bash")"#
        );
        assert_eq!(
            StringMatch::Regex("^mcp".into()).to_string(),
            r#"Regex("^mcp")"#
        );
    }
}
