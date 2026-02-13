//! `StringMatch` factory functions exposed to TypeScript.
//!
//! Factory methods return plain JS objects with a `type` discriminant — the
//! natural TypeScript pattern (discriminated unions). No opaque Rust struct
//! needed for config types; only `HookMatcher` is opaque.
//!
//! ```ts
//! // Bare string → exact match
//! { toolName: "Bash" }
//!
//! // Factory methods
//! { toolName: StringMatch.prefix("mcp__") }
//! { toolName: StringMatch.regex("^(Write|Edit)$") }
//! ```

use js_sys::JsString;
use wasm_bindgen::prelude::*;

/// Factory methods for string matching configuration.
///
/// Each method returns a plain JS object with a `type` discriminant:
/// - `StringMatch.exact("Bash")` → `{ type: "exact", value: "Bash" }`
/// - `StringMatch.prefix("mcp__")` → `{ type: "prefix", value: "mcp__" }`
/// - `StringMatch.regex("^Bash$")` → `{ type: "regex", pattern: "^Bash$" }`
///
/// Bare strings passed directly to `HookMatcher.compile()` fields are also
/// treated as exact matches (Ace convenience).
#[wasm_bindgen]
pub struct StringMatch {
    // Zero-size: exists only as a namespace for static methods.
    _private: u8,
}

/// Build a `{ type, value }` JS object.
fn make_match(kind: &str, key: &str, val: &str) -> JsValue {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &JsString::from("type"), &JsString::from(kind)).unwrap();
    js_sys::Reflect::set(&obj, &JsString::from(key), &JsString::from(val)).unwrap();
    obj.into()
}

#[wasm_bindgen]
#[allow(clippy::needless_pass_by_value)] // wasm-bindgen requires owned String across FFI
impl StringMatch {
    /// Exact equality: `{ type: "exact", value }`.
    pub fn exact(value: String) -> JsValue {
        make_match("exact", "value", &value)
    }

    /// Starts with prefix: `{ type: "prefix", value }`.
    pub fn prefix(value: String) -> JsValue {
        make_match("prefix", "value", &value)
    }

    /// Ends with suffix: `{ type: "suffix", value }`.
    pub fn suffix(value: String) -> JsValue {
        make_match("suffix", "value", &value)
    }

    /// Contains substring: `{ type: "contains", value }`.
    pub fn contains(value: String) -> JsValue {
        make_match("contains", "value", &value)
    }

    /// Matches regex (Rust `regex` crate — guaranteed linear time): `{ type: "regex", pattern }`.
    pub fn regex(pattern: String) -> JsValue {
        make_match("regex", "pattern", &pattern)
    }
}
