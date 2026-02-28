//! xuma-crust — Python bindings for rumi via `PyO3`.
//!
//! Exposes rumi's matcher engine to Python as opaque compiled matchers.
//! Config in → compile in Rust → evaluate in Rust → simple types out.
//!
//! # Matcher Types
//!
//! - [`HookMatcher`] — Claude Code hook matching (typed Python config)
//! - [`HttpMatcher`] — HTTP request matching (JSON config via registry)
//! - [`TestMatcher`] — Key-value matching for conformance testing (JSON config via registry)

mod config;
mod convert;
mod http_matcher;
pub(crate) mod matcher;
mod test_matcher;

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
    m.add_class::<test_matcher::TestMatcher>()?;

    // Trace types (shared across all matchers)
    m.add_class::<matcher::PyTraceResult>()?;
    m.add_class::<matcher::PyTraceStep>()?;

    Ok(())
}
