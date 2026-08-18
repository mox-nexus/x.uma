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
    /// Load a matcher from a JSON config string.
    ///
    /// The config format is `MatcherConfig<String>` — the same JSON shape used
    /// by all x.uma implementations (rumi, puma, bumi).
    ///
    /// # Supported input type URLs
    ///
    /// - `xuma.test.v1.StringInput` — string lookup by key (config: `{"key": "..."}`)
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if:
    /// - JSON config is malformed
    /// - Unknown type URL (error lists available URLs)
    /// - Invalid regex pattern
    /// - Depth/width limits exceeded
    #[staticmethod]
    fn from_config(json_config: &str) -> PyResult<Self> {
        let config: rumi::MatcherConfig<String> = serde_json::from_str(json_config)
            .map_err(|e| PyValueError::new_err(format!("invalid config JSON: {e}")))?;

        let registry = build_test_registry();
        let matcher = registry
            .load_matcher(config)
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
    /// Used for running the `spec/tests/06_config/` conformance suite.
    #[staticmethod]
    fn run_fixtures(yaml_content: &str) -> PyResult<Vec<(String, String, bool, String)>> {
        let fixtures = rumi_test::config_fixture::ConfigFixture::from_yaml_multi(yaml_content)
            .map_err(|e| PyValueError::new_err(format!("invalid YAML: {e}")))?;

        let registry = build_test_registry();
        let mut results = Vec::new();

        for fixture in &fixtures {
            if fixture.expect_error {
                // Error fixtures: config should fail to load
                let config_result: Result<rumi::MatcherConfig<String>, _> =
                    serde_json::from_value(fixture.config.clone());
                match config_result {
                    Err(_) => {
                        results.push((
                            fixture.name.clone(),
                            "parse_error".into(),
                            true,
                            "correctly rejected at parse".into(),
                        ));
                    }
                    Ok(config) => match registry.load_matcher(config) {
                        Err(_) => {
                            results.push((
                                fixture.name.clone(),
                                "load_error".into(),
                                true,
                                "correctly rejected at load".into(),
                            ));
                        }
                        Ok(_) => {
                            results.push((
                                fixture.name.clone(),
                                "should_fail".into(),
                                false,
                                "expected error but config loaded successfully".into(),
                            ));
                        }
                    },
                }
                continue;
            }

            // Normal fixtures: load config and run cases
            let config: rumi::MatcherConfig<String> =
                match serde_json::from_value(fixture.config.clone()) {
                    Ok(c) => c,
                    Err(e) => {
                        results.push((
                            fixture.name.clone(),
                            "parse".into(),
                            false,
                            format!("config parse failed: {e}"),
                        ));
                        continue;
                    }
                };

            let matcher = match registry.load_matcher(config) {
                Ok(m) => m,
                Err(e) => {
                    results.push((
                        fixture.name.clone(),
                        "load".into(),
                        false,
                        format!("config load failed: {e}"),
                    ));
                    continue;
                }
            };

            for case in &fixture.cases {
                let ctx = case.build_context();
                let result = matcher.evaluate(&ctx);
                let passed = result == case.expect;
                let detail = if passed {
                    format!("got {result:?}")
                } else {
                    format!("expected {:?}, got {:?}", case.expect, result)
                };
                results.push((fixture.name.clone(), case.name.clone(), passed, detail));
            }
        }

        Ok(results)
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
