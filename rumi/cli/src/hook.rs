//! Claude Code hook mode: JSON on stdin, decision as an exit code.
//!
//! Real hooks are invoked by Claude Code with a JSON payload on stdin and are
//! expected to answer with an exit code. Before this existed the CLI only took
//! `--event/--tool/--arg` flags, so the README could demo blocking `rm -rf` on
//! its front page with no path from that demo to a working gate.
//!
//! # The exit-code contract
//!
//! | Exit | Meaning to Claude Code |
//! |---|---|
//! | 0 | allow — the tool call proceeds |
//! | 2 | block — the call is stopped and stderr is shown to the model |
//! | other | non-blocking error: surfaced to the user, **and the call proceeds** |
//!
//! That last row is why every failure here exits 2 rather than 1. A malformed
//! payload, an unreadable config, an unknown event: all of them are cases where
//! this program does not know whether the call is safe, and the only honest
//! answer to "should this tool run?" when you cannot tell is no.
//!
//! # Which actions block
//!
//! The matched action decides. `deny` and `block` exit 2; anything else exits 0.
//! Use `--deny-action NAME` to nominate a different one. No match at all exits
//! 0 — an allowlist is expressed by matching what is allowed and setting
//! `on_no_match` to `deny`, not by inverting the default.

use std::collections::HashMap;

/// Exit code meaning "allow" to Claude Code.
pub const EXIT_ALLOW: i32 = 0;

/// Exit code meaning "block". Also used for every error, deliberately.
pub const EXIT_BLOCK: i32 = 2;

/// Actions that block when matched, unless `--deny-action` overrides.
pub const DEFAULT_DENY_ACTIONS: &[&str] = &["deny", "block"];

/// The parts of a Claude Code hook payload this engine can match on.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct HookPayload {
    /// `hook_event_name`, e.g. `PreToolUse`.
    pub event: String,
    /// `tool_name`, absent for non-tool events.
    pub tool_name: String,
    /// Flattened `tool_input`, string values only.
    pub arguments: HashMap<String, String>,
    /// `session_id`.
    pub session_id: String,
    /// `cwd`.
    pub cwd: String,
}

/// Parse a Claude Code hook payload.
///
/// # Errors
///
/// Returns a message if the input is not JSON, is not an object, or omits
/// `hook_event_name`. Callers must treat any error as a block — see the module
/// docs.
pub fn parse_payload(raw: &str) -> Result<HookPayload, String> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("hook payload is not valid JSON: {e}"))?;

    let obj = value
        .as_object()
        .ok_or_else(|| "hook payload must be a JSON object".to_string())?;

    let event = obj
        .get("hook_event_name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "hook payload has no `hook_event_name`".to_string())?
        .to_string();

    let str_field = |key: &str| {
        obj.get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string()
    };

    // tool_input values that are not strings are rendered compactly rather than
    // dropped: a rule matching on a `command` should still see a number or a
    // nested object as *something*, instead of the field silently vanishing.
    let mut arguments = HashMap::new();
    if let Some(input) = obj.get("tool_input").and_then(serde_json::Value::as_object) {
        for (k, v) in input {
            let rendered = match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            arguments.insert(k.clone(), rendered);
        }
    }

    Ok(HookPayload {
        event,
        tool_name: str_field("tool_name"),
        arguments,
        session_id: str_field("session_id"),
        cwd: str_field("cwd"),
    })
}

/// Map a matched action to an exit code.
#[must_use]
pub fn exit_code_for(action: Option<&str>, deny_actions: &[String]) -> i32 {
    match action {
        Some(a) if deny_actions.iter().any(|d| d == a) => EXIT_BLOCK,
        _ => EXIT_ALLOW,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deny_defaults() -> Vec<String> {
        DEFAULT_DENY_ACTIONS
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    }

    #[test]
    fn parses_a_real_pretooluse_payload() {
        let raw = r#"{
            "session_id": "abc123",
            "cwd": "/home/user/project",
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": { "command": "rm -rf /", "description": "cleanup" }
        }"#;
        let p = parse_payload(raw).expect("must parse");
        assert_eq!(p.event, "PreToolUse");
        assert_eq!(p.tool_name, "Bash");
        assert_eq!(p.session_id, "abc123");
        assert_eq!(p.cwd, "/home/user/project");
        assert_eq!(
            p.arguments.get("command").map(String::as_str),
            Some("rm -rf /")
        );
    }

    #[test]
    fn non_string_tool_input_is_rendered_not_dropped() {
        let raw = r#"{"hook_event_name":"PreToolUse","tool_name":"X",
                      "tool_input":{"timeout":30,"nested":{"a":1}}}"#;
        let p = parse_payload(raw).unwrap();
        assert_eq!(p.arguments.get("timeout").map(String::as_str), Some("30"));
        assert!(
            p.arguments.contains_key("nested"),
            "must not silently vanish"
        );
    }

    #[test]
    fn missing_optional_fields_are_empty_not_errors() {
        let p = parse_payload(r#"{"hook_event_name":"Stop"}"#).unwrap();
        assert_eq!(p.event, "Stop");
        assert!(p.tool_name.is_empty());
        assert!(p.arguments.is_empty());
    }

    // ── The safety property ─────────────────────────────────────────────────
    //
    // Claude Code treats exit 2 as "block" and every other non-zero code as a
    // non-blocking error, which lets the tool call proceed. So a gate that
    // cannot tell whether a call is safe must exit 2, not 1.

    #[test]
    fn malformed_payloads_are_rejected() {
        for raw in [
            "",
            "not json",
            "[1,2,3]",
            "\"a string\"",
            "{}",
            r#"{"tool_name":"Bash"}"#,
            r#"{"hook_event_name": 42}"#,
        ] {
            assert!(parse_payload(raw).is_err(), "must reject: {raw:?}");
        }
    }

    #[test]
    fn deny_actions_block_and_others_allow() {
        let d = deny_defaults();
        assert_eq!(exit_code_for(Some("deny"), &d), EXIT_BLOCK);
        assert_eq!(exit_code_for(Some("block"), &d), EXIT_BLOCK);
        assert_eq!(exit_code_for(Some("allow"), &d), EXIT_ALLOW);
        assert_eq!(exit_code_for(Some("audit"), &d), EXIT_ALLOW);
    }

    #[test]
    fn no_match_allows() {
        assert_eq!(exit_code_for(None, &deny_defaults()), EXIT_ALLOW);
    }

    #[test]
    fn a_custom_deny_action_is_honoured() {
        let d = vec!["reject".to_string()];
        assert_eq!(exit_code_for(Some("reject"), &d), EXIT_BLOCK);
        // and the defaults no longer apply once overridden
        assert_eq!(exit_code_for(Some("deny"), &d), EXIT_ALLOW);
    }

    #[test]
    fn the_block_code_is_two_and_that_matters() {
        // Guards the contract itself. Claude Code treats 2 as block and any
        // other non-zero as a non-blocking error that lets the call through, so
        // changing this constant silently converts every deny into an allow.
        assert_eq!(EXIT_BLOCK, 2);
        assert_eq!(EXIT_ALLOW, 0);
    }
}
