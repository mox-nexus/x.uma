//! Config-driven HTTP matcher exposed to Python.
//!
//! Takes a JSON config string, compiles it via Rust registry, and evaluates
//! against HTTP request contexts. Same `MatcherConfig<String>` format used
//! by all implementations.

use std::collections::HashMap;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
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
    inner: Matcher<HttpRequest, String>,
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
    fn from_config(json_config: &str) -> PyResult<Self> {
        let config = crate::protojson::load(json_config).map_err(PyValueError::new_err)?;

        let registry = build_http_registry();
        let matcher = registry
            .load_typed_matcher(config, &crate::protojson::actions())
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        matcher
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Ok(Self { inner: matcher })
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
        method: &str,
        path: &str,
        headers: Option<HashMap<String, String>>,
        query_params: Option<HashMap<String, String>>,
    ) -> Option<String> {
        let req = build_request(method, path, headers, query_params);
        self.inner.evaluate(&req)
    }

    /// Trace evaluation for debugging.
    ///
    /// Returns the same result as `evaluate()` plus a detailed trace.
    #[pyo3(signature = (method, path, headers = None, query_params = None))]
    fn trace(
        &self,
        method: &str,
        path: &str,
        headers: Option<HashMap<String, String>>,
        query_params: Option<HashMap<String, String>>,
    ) -> super::matcher::PyTraceResult {
        let req = build_request(method, path, headers, query_params);
        let trace = self.inner.evaluate_with_trace(&req);

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

    #[allow(clippy::unused_self)]
    fn __repr__(&self) -> String {
        "HttpMatcher(<compiled>)".to_string()
    }
}

/// Build the HTTP registry for `HttpRequest`.
fn build_http_registry() -> rumi::Registry<HttpRequest> {
    rumi_http::register_simple(rumi::RegistryBuilder::new()).build()
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
