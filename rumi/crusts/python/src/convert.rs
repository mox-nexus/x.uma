//! Conversion from Python config types to Rust types with input validation.
//!
//! All validation happens at the FFI boundary (Vector security requirement).
//! Invalid inputs produce clear Python exceptions, never panics.

use crate::config::{PyHookMatch, PyStringMatch};
use pyo3::exceptions::PyValueError;
use pyo3::PyResult;
use rumi::claude::{ArgumentMatch, HookEvent, HookMatch, StringMatch};

/// Maximum length for any string pattern (8 KB).
const MAX_PATTERN_LENGTH: usize = 8192;

/// Maximum number of argument matchers per `HookMatch`.
const MAX_ARGUMENTS: usize = 64;

/// Maximum number of rules per `compile()` call.
pub const MAX_RULES: usize = 256;

/// Maximum regex pattern length (4 KB).
const MAX_REGEX_PATTERN_LENGTH: usize = 4096;

/// Convert a Python `PyHookMatch` to a Rust `HookMatch`.
///
/// # Errors
///
/// Returns `PyValueError` if:
/// - Empty match without `match_all=True` (V-BYPASS-1: fail-closed)
/// - Pattern exceeds length limits
/// - Argument count exceeds limit
/// - Event string is unrecognized
pub fn convert_hook_match(py_match: &PyHookMatch) -> PyResult<HookMatch> {
    // V-BYPASS-1: reject empty matches unless explicitly intended
    let is_empty = py_match.event.is_none()
        && py_match.tool_name.is_none()
        && py_match.arguments.is_empty()
        && py_match.session_id.is_none()
        && py_match.cwd.is_none()
        && py_match.git_branch.is_none();

    if is_empty && !py_match.match_all {
        return Err(PyValueError::new_err(
            "empty HookMatch matches everything — pass match_all=True to confirm, \
             or add at least one field to constrain the match",
        ));
    }

    // Validate argument count
    if py_match.arguments.len() > MAX_ARGUMENTS {
        return Err(PyValueError::new_err(format!(
            "too many arguments: {} exceeds limit of {MAX_ARGUMENTS}",
            py_match.arguments.len()
        )));
    }

    // Convert event
    let event = py_match
        .event
        .as_deref()
        .map(parse_hook_event)
        .transpose()?;

    // Convert string matches
    let tool_name = py_match
        .tool_name
        .as_ref()
        .map(convert_string_match)
        .transpose()?;
    let cwd = py_match
        .cwd
        .as_ref()
        .map(convert_string_match)
        .transpose()?;
    let git_branch = py_match
        .git_branch
        .as_ref()
        .map(convert_string_match)
        .transpose()?;

    // Convert arguments
    let arguments = if py_match.arguments.is_empty() {
        None
    } else {
        let mut args = Vec::with_capacity(py_match.arguments.len());
        for (name, value) in &py_match.arguments {
            validate_pattern_length(name, "argument name")?;
            args.push(ArgumentMatch {
                name: name.clone(),
                value: convert_string_match(value)?,
            });
        }
        Some(args)
    };

    Ok(HookMatch {
        event,
        tool_name,
        arguments,
        cwd,
        git_branch,
    })
}

/// Convert a Python `PyStringMatch` to a Rust `StringMatch`.
fn convert_string_match(py_sm: &PyStringMatch) -> PyResult<StringMatch> {
    match py_sm {
        PyStringMatch::Exact { value } => {
            validate_pattern_length(value, "exact pattern")?;
            Ok(StringMatch::Exact(value.clone()))
        }
        PyStringMatch::Prefix { value } => {
            validate_pattern_length(value, "prefix pattern")?;
            Ok(StringMatch::Prefix(value.clone()))
        }
        PyStringMatch::Suffix { value } => {
            validate_pattern_length(value, "suffix pattern")?;
            Ok(StringMatch::Suffix(value.clone()))
        }
        PyStringMatch::Contains { value } => {
            validate_pattern_length(value, "contains pattern")?;
            Ok(StringMatch::Contains(value.clone()))
        }
        PyStringMatch::Regex { pattern } => {
            if pattern.len() > MAX_REGEX_PATTERN_LENGTH {
                return Err(PyValueError::new_err(format!(
                    "regex pattern length {} exceeds limit of {MAX_REGEX_PATTERN_LENGTH}",
                    pattern.len()
                )));
            }
            Ok(StringMatch::Regex(pattern.clone()))
        }
    }
}

/// Validate that a pattern doesn't exceed the general length limit.
fn validate_pattern_length(value: &str, label: &str) -> PyResult<()> {
    if value.len() > MAX_PATTERN_LENGTH {
        return Err(PyValueError::new_err(format!(
            "{label} length {} exceeds limit of {MAX_PATTERN_LENGTH}",
            value.len()
        )));
    }
    Ok(())
}

/// Parse a hook event string into a `HookEvent` enum.
fn parse_hook_event(s: &str) -> PyResult<HookEvent> {
    match s {
        "PreToolUse" => Ok(HookEvent::PreToolUse),
        "PostToolUse" => Ok(HookEvent::PostToolUse),
        "Stop" => Ok(HookEvent::Stop),
        "SubagentStop" => Ok(HookEvent::SubagentStop),
        "UserPromptSubmit" => Ok(HookEvent::UserPromptSubmit),
        "SessionStart" => Ok(HookEvent::SessionStart),
        "SessionEnd" => Ok(HookEvent::SessionEnd),
        "PreCompact" => Ok(HookEvent::PreCompact),
        "Notification" => Ok(HookEvent::Notification),
        _ => Err(PyValueError::new_err(format!(
            "unknown hook event: {s:?}. Expected one of: PreToolUse, PostToolUse, Stop, \
             SubagentStop, UserPromptSubmit, SessionStart, SessionEnd, PreCompact, Notification"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_match_rejected_without_match_all() {
        let py_match = PyHookMatch {
            event: None,
            tool_name: None,
            arguments: vec![],
            session_id: None,
            cwd: None,
            git_branch: None,
            match_all: false,
        };
        let result = convert_hook_match(&py_match);
        assert!(result.is_err());
    }

    #[test]
    fn empty_match_allowed_with_match_all() {
        let py_match = PyHookMatch {
            event: None,
            tool_name: None,
            arguments: vec![],
            session_id: None,
            cwd: None,
            git_branch: None,
            match_all: true,
        };
        let result = convert_hook_match(&py_match);
        assert!(result.is_ok());
    }

    #[test]
    fn valid_event_parses() {
        assert!(parse_hook_event("PreToolUse").is_ok());
        assert!(parse_hook_event("Stop").is_ok());
        assert!(parse_hook_event("Notification").is_ok());
    }

    #[test]
    fn invalid_event_rejected() {
        let result = parse_hook_event("NotAnEvent");
        assert!(result.is_err());
    }

    #[test]
    fn oversized_pattern_rejected() {
        let big = "a".repeat(MAX_PATTERN_LENGTH + 1);
        let result = convert_string_match(&PyStringMatch::Exact { value: big });
        assert!(result.is_err());
    }

    #[test]
    fn oversized_regex_rejected() {
        let big = "a".repeat(MAX_REGEX_PATTERN_LENGTH + 1);
        let result = convert_string_match(&PyStringMatch::Regex { pattern: big });
        assert!(result.is_err());
    }

    #[test]
    fn too_many_arguments_rejected() {
        let py_match = PyHookMatch {
            event: Some("PreToolUse".into()),
            tool_name: None,
            arguments: (0..MAX_ARGUMENTS + 1)
                .map(|i| {
                    (
                        format!("arg{i}"),
                        PyStringMatch::Exact { value: "v".into() },
                    )
                })
                .collect(),
            session_id: None,
            cwd: None,
            git_branch: None,
            match_all: false,
        };
        let result = convert_hook_match(&py_match);
        assert!(result.is_err());
    }
}
