//! The boundary's floor, one no-op crossing per shape (D-066).
//!
//! Nothing here evaluates anything. Each function does the least work that
//! its return or argument shape allows, so a benchmark against it reads the
//! cost of the crossing itself, and a real path is judged against that floor
//! rather than against zero.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::PyString;

/// A string made once and handed back by reference: the shape `evaluate`
/// returns after D-056. Its distance from the `u32` floor is the reference's
/// own cost.
#[pyfunction]
pub fn boundary_floor_cached_string(py: Python<'_>) -> Py<PyString> {
    static CACHED: PyOnceLock<Py<PyString>> = PyOnceLock::new();
    CACHED
        .get_or_init(py, || PyString::new(py, "matched").unbind())
        .clone_ref(py)
}

/// A bare integer back across the boundary.
#[pyfunction]
pub fn boundary_floor_u32() -> u32 {
    1
}

/// An owned string back across the boundary, the shape `evaluate` returns today.
#[pyfunction]
pub fn boundary_floor_option_string() -> Option<String> {
    Some("matched".to_string())
}

/// One packed string in across the boundary: fields separated by U+001F,
/// split into owned Rust strings. The other candidate shape for the flat
/// context port (D-057).
#[pyfunction]
pub fn boundary_floor_packed(packed: &str) -> u32 {
    let fields: Vec<String> = packed.split('\u{1f}').map(str::to_owned).collect();
    u32::try_from(fields.len()).unwrap_or(u32::MAX)
}

/// A flat string map in across the boundary, marshalled into Rust strings,
/// the shape the flat context port would cross (D-057).
#[pyfunction]
pub fn boundary_floor_flat_map(map: HashMap<String, String>) -> u32 {
    u32::try_from(map.len()).unwrap_or(u32::MAX)
}
