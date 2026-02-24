//! User-facing configuration types for Claude Code hook matching.
//!
//! These types provide a declarative API for specifying match rules.
//! The [compiler](super::compiler) translates them into runtime `Matcher` trees.

use super::context::HookEvent;

/// How to match a string value.
///
/// This is a re-export of [`crate::StringMatchSpec`] for backward compatibility.
/// Domain-agnostic string matching specification that compiles to runtime matchers.
pub use crate::StringMatchSpec as StringMatch;

/// User-friendly configuration for matching Claude Code hook events.
///
/// All fields are optional. Omitted fields match anything.
/// All present fields are `ANDed` (every condition must match).
///
/// # Example
///
/// ```ignore
/// use rumi::claude::prelude::*;
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
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArgumentMatch {
    /// The argument name to extract.
    pub name: String,
    /// How to match the argument's value.
    pub value: StringMatch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

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
        assert!(err.to_string().contains("invalid pattern"));
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
