//! The boundary's floor, one no-op crossing per shape (D-066).
//!
//! Nothing here evaluates anything. Each function does the least work that
//! its return or argument shape allows, so a benchmark against it reads the
//! cost of the crossing itself, and a real path is judged against that floor
//! rather than against zero.

use std::collections::HashMap;

use wasm_bindgen::prelude::*;

/// A bare integer back across the boundary.
#[wasm_bindgen(js_name = "boundaryFloorU32")]
#[must_use]
pub fn boundary_floor_u32() -> u32 {
    1
}

/// An owned string back across the boundary, the shape `evaluate` returns today.
#[wasm_bindgen(js_name = "boundaryFloorOptionString")]
#[must_use]
pub fn boundary_floor_option_string() -> Option<String> {
    Some("matched".to_string())
}

/// A flat string map in across the boundary, marshalled into Rust strings,
/// the shape the flat context port would cross (D-057).
///
/// # Errors
///
/// If a key or value is not a string.
#[wasm_bindgen(js_name = "boundaryFloorFlatMap")]
pub fn boundary_floor_flat_map(map: JsValue) -> Result<u32, JsValue> {
    let entries = js_sys::Object::entries(&js_sys::Object::from(map));
    let mut values: HashMap<String, String> = HashMap::with_capacity(entries.length() as usize);
    for entry in entries.iter() {
        let pair = js_sys::Array::from(&entry);
        let key = pair
            .get(0)
            .as_string()
            .ok_or_else(|| JsValue::from_str("key must be a string"))?;
        let value = pair
            .get(1)
            .as_string()
            .ok_or_else(|| JsValue::from_str("value must be a string"))?;
        values.insert(key, value);
    }
    Ok(u32::try_from(values.len()).unwrap_or(u32::MAX))
}
