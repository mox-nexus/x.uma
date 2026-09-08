//! xuma-crust — Python bindings for rumi via `PyO3`.
//!
//! Exposes rumi's matcher engine to Python as opaque compiled matchers.
//! Config in → compile in Rust → evaluate in Rust → simple types out.
//!
//! # Matcher Types
//!
//! - [`HookMatcher`] — Claude Code hook matching (typed Python config)
//! - [`HttpMatcher`] — HTTP request matching (JSON config via registry)
//! - [`KvMatcher`] — key-value matching over `xuma.kv.v1` inputs; the conformance domain

mod config;
mod convert;
#[cfg(feature = "fixtures")]
mod fixtures;
#[cfg(feature = "bench")]
mod floor;
mod http_matcher;
mod kv_matcher;
pub(crate) mod matcher;
mod protojson;

use pyo3::prelude::*;

/// Python module: `xuma_crust`
#[pymodule]
fn xuma_crust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Config types (HookMatcher typed API)
    m.add_class::<config::PyStringMatch>()?;
    m.add_class::<config::PyHookMatch>()?;

    // Compiled matchers
    m.add_class::<matcher::HookMatcher>()?;
    m.add_class::<http_matcher::HttpMatcher>()?;
    m.add_class::<kv_matcher::KvMatcher>()?;

    // Trace types (shared across all matchers)
    m.add_class::<matcher::PyTraceResult>()?;
    m.add_class::<matcher::PyTraceStep>()?;
    m.add_class::<matcher::PyDecision>()?;

    #[cfg(feature = "bench")]
    {
        m.add_function(wrap_pyfunction!(floor::boundary_floor_u32, m)?)?;
        m.add_function(wrap_pyfunction!(floor::boundary_floor_option_string, m)?)?;
        m.add_function(wrap_pyfunction!(floor::boundary_floor_flat_map, m)?)?;
    }

    Ok(())
}
