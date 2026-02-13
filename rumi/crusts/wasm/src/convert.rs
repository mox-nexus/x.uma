//! Conversion from JS values to Rust types with input validation.
//!
//! All validation happens at the FFI boundary (Vector security requirement).
//! Invalid inputs produce clear JS errors, never panics.

use rumi_claude::{ArgumentMatch, HookEvent, HookMatch, StringMatch};
use wasm_bindgen::prelude::*;

/// Maximum length for any string pattern (8 KB).
const MAX_PATTERN_LENGTH: usize = 8192;

/// Maximum number of argument matchers per rule.
const MAX_ARGUMENTS: usize = 64;

/// Maximum number of rules per `compile()` call.
pub const MAX_RULES: u32 = 256;

/// Maximum regex pattern length (4 KB).
const MAX_REGEX_PATTERN_LENGTH: usize = 4096;

/// Extract a field from a JS object, returning `JsValue::UNDEFINED` if absent.
fn get_field(obj: &JsValue, key: &str) -> JsValue {
    js_sys::Reflect::get(obj, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED)
}

/// Convert a JS object to a Rust `HookMatch`.
///
/// Accepts plain JS objects with optional camelCase fields:
/// ```js
/// { event: "PreToolUse", toolName: "Bash", matchAll: false }
/// ```
pub fn convert_hook_match_from_js(val: &JsValue) -> Result<HookMatch, JsValue> {
    // Event
    let event_val = get_field(val, "event");
    let event = if event_val.is_undefined() || event_val.is_null() {
        None
    } else {
        let s = event_val
            .as_string()
            .ok_or_else(|| JsValue::from_str("event must be a string"))?;
        Some(parse_hook_event(&s)?)
    };

    // Tool name (string | StringMatch object)
    let tool_name_val = get_field(val, "toolName");
    let tool_name = convert_optional_string_match(&tool_name_val)?;

    // Arguments: Array<[string, string | StringMatch]>
    let args_val = get_field(val, "arguments");
    let arguments = if args_val.is_undefined() || args_val.is_null() {
        None
    } else {
        let args_array = js_sys::Array::from(&args_val);
        #[allow(clippy::cast_possible_truncation)] // MAX_ARGUMENTS (64) fits in u32
        if args_array.length() > MAX_ARGUMENTS as u32 {
            return Err(JsValue::from_str(&format!(
                "too many arguments: {} exceeds limit of {MAX_ARGUMENTS}",
                args_array.length()
            )));
        }
        let mut args = Vec::with_capacity(args_array.length() as usize);
        for item in args_array.iter() {
            let pair = js_sys::Array::from(&item);
            let name = pair
                .get(0)
                .as_string()
                .ok_or_else(|| JsValue::from_str("argument name must be a string"))?;
            validate_pattern_length(&name, "argument name")?;
            let value = convert_string_match_value(&pair.get(1))?;
            args.push(ArgumentMatch { name, value });
        }
        Some(args)
    };

    // sessionId (string | StringMatch object)
    let session_id_val = get_field(val, "sessionId");
    let session_id = convert_optional_string_match(&session_id_val)?;

    // cwd (string | StringMatch object)
    let cwd_val = get_field(val, "cwd");
    let cwd = convert_optional_string_match(&cwd_val)?;

    // gitBranch (string | StringMatch object)
    let git_branch_val = get_field(val, "gitBranch");
    let git_branch = convert_optional_string_match(&git_branch_val)?;

    // V-BYPASS-1: reject empty matches unless explicitly intended
    let match_all_val = get_field(val, "matchAll");
    let match_all = match_all_val.as_bool().unwrap_or(false);

    let is_empty = event.is_none()
        && tool_name.is_none()
        && arguments.is_none()
        && session_id.is_none()
        && cwd.is_none()
        && git_branch.is_none();

    if is_empty && !match_all {
        return Err(JsValue::from_str(
            "empty HookMatch matches everything \u{2014} pass matchAll: true to confirm, \
             or add at least one field to constrain the match",
        ));
    }

    Ok(HookMatch {
        event,
        tool_name,
        arguments,
        cwd,
        git_branch,
    })
}

/// Convert an optional string match field.
fn convert_optional_string_match(val: &JsValue) -> Result<Option<StringMatch>, JsValue> {
    if val.is_undefined() || val.is_null() {
        return Ok(None);
    }
    Ok(Some(convert_string_match_value(val)?))
}

/// Convert a JS value to a Rust `StringMatch`.
///
/// Accepts:
/// - Bare string → exact match (`"Bash"`)
/// - Object with `type` discriminant → other modes (`{ type: "prefix", value: "mcp__" }`)
pub fn convert_string_match_value(val: &JsValue) -> Result<StringMatch, JsValue> {
    // Bare string → Exact (Ace convenience: toolName: "Bash")
    // Note: as_string() handles JS string primitives; is_instance_of::<JsString>()
    // only matches `new String()` objects which are rare in practice.
    if let Some(s) = val.as_string() {
        validate_pattern_length(&s, "exact pattern")?;
        return Ok(StringMatch::Exact(s));
    }

    // Object with type discriminant: { type: "prefix", value: "mcp__" }
    let type_val = get_field(val, "type");
    let kind = type_val.as_string().ok_or_else(|| {
        JsValue::from_str(
            "expected string or object with 'type' field (exact, prefix, suffix, contains, regex)",
        )
    })?;

    match kind.as_str() {
        "exact" => {
            let v = get_string_field(val, "value", "exact: value must be a string")?;
            validate_pattern_length(&v, "exact pattern")?;
            Ok(StringMatch::Exact(v))
        }
        "prefix" => {
            let v = get_string_field(val, "value", "prefix: value must be a string")?;
            validate_pattern_length(&v, "prefix pattern")?;
            Ok(StringMatch::Prefix(v))
        }
        "suffix" => {
            let v = get_string_field(val, "value", "suffix: value must be a string")?;
            validate_pattern_length(&v, "suffix pattern")?;
            Ok(StringMatch::Suffix(v))
        }
        "contains" => {
            let v = get_string_field(val, "value", "contains: value must be a string")?;
            validate_pattern_length(&v, "contains pattern")?;
            Ok(StringMatch::Contains(v))
        }
        "regex" => {
            let p = get_string_field(val, "pattern", "regex: pattern must be a string")?;
            if p.len() > MAX_REGEX_PATTERN_LENGTH {
                return Err(JsValue::from_str(&format!(
                    "regex pattern length {} exceeds limit of {MAX_REGEX_PATTERN_LENGTH}",
                    p.len()
                )));
            }
            Ok(StringMatch::Regex(p))
        }
        _ => Err(JsValue::from_str(&format!(
            "unknown StringMatch type: \"{kind}\". \
             Expected: exact, prefix, suffix, contains, regex"
        ))),
    }
}

/// Extract a required string field from a JS object.
fn get_string_field(obj: &JsValue, key: &str, error_msg: &str) -> Result<String, JsValue> {
    get_field(obj, key)
        .as_string()
        .ok_or_else(|| JsValue::from_str(error_msg))
}

/// Validate that a pattern doesn't exceed the general length limit.
fn validate_pattern_length(value: &str, label: &str) -> Result<(), JsValue> {
    if value.len() > MAX_PATTERN_LENGTH {
        return Err(JsValue::from_str(&format!(
            "{label} length {} exceeds limit of {MAX_PATTERN_LENGTH}",
            value.len()
        )));
    }
    Ok(())
}

/// Parse a hook event string into a `HookEvent` enum.
pub fn parse_hook_event(s: &str) -> Result<HookEvent, JsValue> {
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
        _ => Err(JsValue::from_str(&format!(
            "unknown hook event: \"{s}\". Expected one of: PreToolUse, PostToolUse, Stop, \
             SubagentStop, UserPromptSubmit, SessionStart, SessionEnd, PreCompact, Notification"
        ))),
    }
}
