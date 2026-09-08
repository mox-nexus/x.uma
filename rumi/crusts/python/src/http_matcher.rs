//! Config-driven HTTP matcher exposed to Python.
//!
//! Takes a JSON config string, compiles it via Rust registry, and evaluates
//! against HTTP request contexts. Same `MatcherIr<String>` format used
//! by all implementations.

use std::sync::OnceLock;

use std::collections::HashMap;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyString;
use rumi_proto::index::ActionIndex;

use crate::matcher::{Actions, PyDecision};
use rumi::prelude::*;
use rumi_http::HttpRequest;

/// An opaque compiled HTTP matcher.
///
/// Created via `HttpMatcher.from_config()`, immutable after construction.
/// Evaluates HTTP requests against compiled matcher trees.
///
/// # Thread Safety
///
/// `HttpMatcher` is immutable and safe to share across threads.
#[pyclass(frozen)]
pub struct HttpMatcher {
    inner: Matcher<HttpRequest, ActionIndex>,
    actions: Actions,
}

#[pymethods]
impl HttpMatcher {
    /// Load a matcher from a canonical protojson config string.
    ///
    /// protojson is the format all x.uma implementations use — protobuf's own
    /// JSON mapping of `xds.type.matcher.v3.Matcher`. See DECISIONS.md D-026.
    ///
    /// # Supported input type URLs
    ///
    /// - `xuma.http.v1.PathInput` / `MethodInput` / `AuthorityInput` / `SchemeInput` — no config
    /// - `xuma.http.v1.HeaderInput` — header value (`{"name": "..."}`)
    /// - `xuma.http.v1.QueryParamInput` — query parameter (`{"name": "..."}`)
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

    /// Evaluate an HTTP request against compiled matcher rules.
    ///
    /// # Arguments
    ///
    /// * `method` — HTTP method (e.g., "GET", "POST").
    /// * `path` — Request path (e.g., "/api/users").
    /// * `headers` — Request headers as key-value pairs (keys are case-insensitive).
    /// * `query_params` — Query parameters as key-value pairs.
    ///
    /// # Returns
    ///
    /// The action string if the request matched, or `None`.
    #[pyo3(signature = (method, path, headers = None, query_params = None))]
    fn evaluate(
        &self,
        py: Python<'_>,
        method: &str,
        path: &str,
        headers: Option<HashMap<String, String>>,
        query_params: Option<HashMap<String, String>>,
    ) -> Option<Py<PyString>> {
        let req = build_request(method, path, headers, query_params);
        self.actions.action(py, self.inner.evaluate(&req))
    }

    /// The decision as an object: `matched`, `action`, `index`, `fallback`.
    #[pyo3(signature = (method, path, headers = None, query_params = None))]
    fn decide(
        &self,
        py: Python<'_>,
        method: &str,
        path: &str,
        headers: Option<HashMap<String, String>>,
        query_params: Option<HashMap<String, String>>,
    ) -> PyDecision {
        let req = build_request(method, path, headers, query_params);
        self.actions.decide(py, self.inner.evaluate(&req))
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
    #[pyo3(signature = (method, path, headers = None, query_params = None))]
    fn trace(
        &self,
        py: Python<'_>,
        method: &str,
        path: &str,
        headers: Option<HashMap<String, String>>,
        query_params: Option<HashMap<String, String>>,
    ) -> super::matcher::PyTraceResult {
        let req = build_request(method, path, headers, query_params);
        let trace = self.inner.evaluate_with_trace(&req);
        self.actions.trace(py, &trace)
    }

    #[allow(clippy::unused_self)]
    fn __repr__(&self) -> String {
        "HttpMatcher(<compiled>)".to_string()
    }
}

/// The registry every `HttpMatcher` loads through. Built on first use; every
/// call after that returns the same instance.
pub(crate) fn registry() -> &'static rumi::Registry<HttpRequest> {
    static REGISTRY: OnceLock<rumi::Registry<HttpRequest>> = OnceLock::new();
    REGISTRY.get_or_init(|| rumi_http::register_simple(rumi::RegistryBuilder::new()).build())
}

/// Build an `HttpRequest` from Python arguments.
fn build_request(
    method: &str,
    path: &str,
    headers: Option<HashMap<String, String>>,
    query_params: Option<HashMap<String, String>>,
) -> HttpRequest {
    let mut builder = HttpRequest::builder().method(method).path(path);

    if let Some(hdrs) = headers {
        for (k, v) in hdrs {
            builder = builder.header(k, v);
        }
    }

    if let Some(params) = query_params {
        for (k, v) in params {
            builder = builder.query_param(k, v);
        }
    }

    builder.build()
}
