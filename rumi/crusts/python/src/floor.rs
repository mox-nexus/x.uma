//! The boundary's floor, one no-op crossing per shape (D-066).
//!
//! Nothing here evaluates anything. Each function does the least work that
//! its return or argument shape allows, so a benchmark against it reads the
//! cost of the crossing itself, and a real path is judged against that floor
//! rather than against zero.

use std::collections::HashMap;

use pyo3::prelude::*;

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

/// A flat string map in across the boundary, marshalled into Rust strings,
/// the shape the flat context port would cross (D-057).
#[pyfunction]
pub fn boundary_floor_flat_map(map: HashMap<String, String>) -> u32 {
    u32::try_from(map.len()).unwrap_or(u32::MAX)
}
