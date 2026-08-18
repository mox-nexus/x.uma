//! Config-driven test matcher exposed to Python.
//!
//! Takes a JSON config string, compiles it via Rust registry, and evaluates
//! against simple key-value contexts. Used for conformance testing.

use std::collections::HashMap;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rumi::prelude::*;
use rumi_kv::KvContext;

/// An opaque compiled test matcher.
///
/// Created via `TestMatcher.from_config()`, immutable after construction.
/// Evaluates key-value contexts against compiled matcher trees.
///
/// # Thread Safety
///
/// `TestMatcher` is immutable and safe to share across threads.
#[pyclass(frozen)]
pub struct TestMatcher {
    inner: Matcher<KvContext, String>,
}

#[pymethods]
impl TestMatcher {
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
    fn from_config(json_config: &str) -> PyResult<Self> {
        let config = crate::protojson::load(json_config).map_err(PyValueError::new_err)?;

        let registry = build_test_registry();
        let matcher = registry
            .load_typed_matcher(config, &crate::protojson::actions())
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        matcher
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Ok(Self { inner: matcher })
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
    fn evaluate(&self, context: HashMap<String, String>) -> Option<String> {
        let ctx = build_context(context);
        self.inner.evaluate(&ctx)
    }

    /// Trace evaluation for debugging.
    ///
    /// Returns the same result as `evaluate()` plus a detailed trace.
    fn trace(&self, context: HashMap<String, String>) -> super::matcher::PyTraceResult {
        let ctx = build_context(context);
        let trace = self.inner.evaluate_with_trace(&ctx);

        let steps: Vec<super::matcher::PyTraceStep> = trace
            .steps
            .iter()
            .map(|step| super::matcher::PyTraceStep {
                index: step.index,
                matched: step.matched,
                predicate: format!("{:?}", step.predicate_trace),
            })
            .collect();

        super::matcher::PyTraceResult {
            result: trace.result,
            steps,
            used_fallback: trace.used_fallback,
        }
    }

    #[cfg(feature = "fixtures")]
    /// Load and run conformance fixtures from a YAML file.
    ///
    /// Returns a list of `(fixture_name, case_name, passed, detail)` tuples.
    /// Runs `spec/tests/07_protojson/` — canonical protojson, the same fixtures
    /// and the same reader rumi, puma and bumi use.
    ///
    /// The crusts are implementations four and five. Reading a different config
    /// format from the other three would turn "all five agree" into a claim
    /// about five different questions.
    #[staticmethod]
    fn run_fixtures(yaml_content: &str) -> PyResult<Vec<(String, String, bool, String)>> {
        crate::fixtures::run_protojson(yaml_content).map_err(PyValueError::new_err)
    }

    #[allow(clippy::unused_self)]
    fn __repr__(&self) -> String {
        "TestMatcher(<compiled>)".to_string()
    }
}

/// Build the test registry for `KvContext`.
fn build_test_registry() -> rumi::Registry<KvContext> {
    rumi_kv::register(rumi::RegistryBuilder::new()).build()
}

/// Build a `KvContext` from a Python dict.
fn build_context(values: HashMap<String, String>) -> KvContext {
    let mut ctx = KvContext::new();
    for (k, v) in values {
        ctx = ctx.with(k, v);
    }
    ctx
}
