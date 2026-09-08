//! The key-value matcher exposed to Python.
//!
//! Compiles a protojson document over `xuma.kv.v1` inputs and evaluates
//! key-value contexts: the domain the conformance suite runs in.

use std::sync::OnceLock;

use std::collections::HashMap;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyString;
use rumi_proto::index::ActionIndex;

use crate::matcher::{Actions, PyDecision};
use rumi::prelude::*;
use rumi_kv::KvContext;

/// An opaque compiled key-value matcher.
///
/// Created via `KvMatcher.from_config()`, immutable after construction.
/// Evaluates key-value contexts against compiled matcher trees.
///
/// # Thread Safety
///
/// `KvMatcher` is immutable and safe to share across threads.
#[pyclass(frozen)]
pub struct KvMatcher {
    inner: Matcher<KvContext, ActionIndex>,
    actions: Actions,
}

#[pymethods]
impl KvMatcher {
    /// Load a matcher from a canonical protojson config string.
    ///
    /// protojson is the format all x.uma implementations use — protobuf's own
    /// JSON mapping of `xds.type.matcher.v3.Matcher`. See DECISIONS.md D-026.
    ///
    /// # Supported input type URLs
    ///
    /// - `xuma.kv.v1.MapInput` — string lookup by key (`{"key": "..."}`)
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if:
    /// - JSON config is malformed, or not a valid `Matcher`
    /// - Unknown type URL (error lists available URLs)
    /// - Invalid regex pattern
    /// - Depth/width limits exceeded
    #[staticmethod]
    fn from_config(py: Python<'_>, json_config: &str) -> PyResult<Self> {
        let config = crate::protojson::load(json_config).map_err(PyValueError::new_err)?;

        let loaded = crate::protojson::load_indexed(registry(), config)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let (inner, actions) = Actions::split(py, loaded);

        Ok(Self { inner, actions })
    }

    /// Evaluate a key-value context against compiled matcher rules.
    ///
    /// # Arguments
    ///
    /// * `context` — A dictionary of string key-value pairs.
    ///
    /// # Returns
    ///
    /// The action string if the context matched, or `None`.
    fn evaluate(&self, py: Python<'_>, context: HashMap<String, String>) -> Option<Py<PyString>> {
        let ctx = build_context(context);
        self.actions.action(py, self.inner.evaluate(&ctx))
    }

    /// The decision as an object: `matched`, `action`, `index`, `fallback`.
    fn decide(&self, py: Python<'_>, context: HashMap<String, String>) -> PyDecision {
        let ctx = build_context(context);
        self.actions.decide(py, self.inner.evaluate(&ctx))
    }

    /// Every action this matcher can return, in index order.
    fn actions(&self) -> Vec<String> {
        self.actions.names()
    }

    /// The metadata carried by the action at `index`.
    fn action_metadata(&self, index: u32) -> PyResult<HashMap<String, String>> {
        self.actions.metadata(index)
    }

    /// Trace evaluation for debugging.
    ///
    /// Returns the same result as `evaluate()` plus a detailed trace.
    fn trace(
        &self,
        py: Python<'_>,
        context: HashMap<String, String>,
    ) -> super::matcher::PyTraceResult {
        let ctx = build_context(context);
        let trace = self.inner.evaluate_with_trace(&ctx);
        self.actions.trace(py, &trace)
    }

    #[cfg(feature = "fixtures")]
    /// Load and run conformance fixtures from a YAML file.
    ///
    /// Returns a list of `(fixture_name, case_name, passed, detail)` tuples.
    /// Runs `spec/tests/07_protojson/` — canonical protojson, through the same
    /// reader the engine uses, so a passing suite is a claim about this
    /// binding's door and not about a second parser.
    #[staticmethod]
    fn run_fixtures(yaml_content: &str) -> PyResult<Vec<(String, String, bool, String)>> {
        crate::fixtures::run_protojson(
            yaml_content,
            rumi_test::implementations::Implementation::Python,
        )
        .map_err(PyValueError::new_err)
    }

    #[allow(clippy::unused_self)]
    fn __repr__(&self) -> String {
        "KvMatcher(<compiled>)".to_string()
    }
}

/// The registry every `KvMatcher` loads through. Built on first use; every
/// call after that returns the same instance.
pub(crate) fn registry() -> &'static rumi::Registry<KvContext> {
    static REGISTRY: OnceLock<rumi::Registry<KvContext>> = OnceLock::new();
    REGISTRY.get_or_init(|| rumi_kv::register(rumi::RegistryBuilder::new()).build())
}

/// Build a `KvContext` from a Python dict.
fn build_context(values: HashMap<String, String>) -> KvContext {
    let mut ctx = KvContext::new();
    for (k, v) in values {
        ctx = ctx.with(k, v);
    }
    ctx
}
