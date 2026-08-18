//! Config-driven test matcher exposed to TypeScript via wasm-bindgen.
//!
//! Takes a JSON config string, compiles it via Rust registry, and evaluates
//! against key-value contexts passed as plain JS objects.

use rumi::prelude::*;
use rumi_kv::KvContext;
use wasm_bindgen::prelude::*;

use crate::matcher::{TraceResultSerde, TraceStepSerde};

/// An opaque compiled test matcher.
///
/// Created via `TestMatcher.fromConfig()`, immutable after construction.
/// Evaluates key-value contexts against compiled matcher trees.
#[wasm_bindgen]
pub struct TestMatcher {
    inner: Matcher<KvContext, String>,
}

#[wasm_bindgen]
impl TestMatcher {
    /// Load a matcher from a JSON config string.
    ///
    /// The config format is `MatcherConfig<String>` — the same JSON shape used
    /// by all x.uma implementations (rumi, puma, bumi).
    ///
    /// # Supported input type URLs
    ///
    /// - `xuma.kv.v1.MapInput` — string lookup by key (config: `{"key": "..."}`)
    #[wasm_bindgen(js_name = "fromConfig")]
    pub fn from_config(json_config: &str) -> Result<TestMatcher, JsValue> {
        let config: rumi::MatcherConfig<String> = serde_json::from_str(json_config)
            .map_err(|e| JsValue::from_str(&format!("invalid config JSON: {e}")))?;

        let registry = build_test_registry();
        let matcher = registry
            .load_matcher(config)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        matcher
            .validate()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(Self { inner: matcher })
    }

    /// Evaluate a key-value context against compiled matcher rules.
    ///
    /// Accepts a plain `Record<string, string>` object:
    /// ```js
    /// matcher.evaluate({ role: "admin", org: "acme" })
    /// ```
    ///
    /// Returns the action string if the context matched, or `undefined`.
    pub fn evaluate(&self, context: JsValue) -> Result<Option<String>, JsValue> {
        let ctx = build_context_from_js(&context)?;
        Ok(self.inner.evaluate(&ctx))
    }

    /// Trace evaluation for debugging.
    pub fn trace(&self, context: JsValue) -> Result<JsValue, JsValue> {
        let ctx = build_context_from_js(&context)?;
        let trace = self.inner.evaluate_with_trace(&ctx);

        let steps: Vec<TraceStepSerde> = trace
            .steps
            .iter()
            .map(|step| TraceStepSerde {
                index: step.index,
                matched: step.matched,
                predicate: format!("{:?}", step.predicate_trace),
            })
            .collect();

        let result = TraceResultSerde {
            result: trace.result,
            steps,
            used_fallback: trace.used_fallback,
        };

        serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[cfg(feature = "fixtures")]
    /// Load and run conformance fixtures from a YAML string.
    ///
    /// Returns an array of `{ fixture, caseName, passed, detail }` objects.
    ///
    /// Runs `spec/tests/07_protojson/` — canonical protojson, the same fixtures
    /// and the same reader rumi, puma and bumi use. The crusts are
    /// implementations four and five; reading a different config format from
    /// the other three would turn "all five agree" into a claim about five
    /// different questions.
    #[wasm_bindgen(js_name = "runFixtures")]
    pub fn run_fixtures(yaml_content: &str) -> Result<JsValue, JsValue> {
        let results: Vec<FixtureResultSerde> = crate::fixtures::run_protojson(yaml_content)
            .map_err(|e| JsValue::from_str(&e))?
            .into_iter()
            .map(|(fixture, case_name, passed, detail)| FixtureResultSerde {
                fixture,
                case_name,
                passed,
                detail,
            })
            .collect();
        serde_wasm_bindgen::to_value(&results).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

/// Build the test registry for `KvContext`.
fn build_test_registry() -> rumi::Registry<KvContext> {
    rumi_kv::register(rumi::RegistryBuilder::new()).build()
}

/// Build a `KvContext` from a JS plain object (Record<string, string>).
fn build_context_from_js(val: &JsValue) -> Result<KvContext, JsValue> {
    let entries = js_sys::Object::entries(&js_sys::Object::from(val.clone()));
    let mut ctx = KvContext::new();
    for entry in entries.iter() {
        let pair = js_sys::Array::from(&entry);
        let key = pair
            .get(0)
            .as_string()
            .ok_or_else(|| JsValue::from_str("context key must be a string"))?;
        let value = pair
            .get(1)
            .as_string()
            .ok_or_else(|| JsValue::from_str("context value must be a string"))?;
        ctx = ctx.with(key, value);
    }
    Ok(ctx)
}

/// Fixture result for serde-wasm-bindgen serialization.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg(feature = "fixtures")]
struct FixtureResultSerde {
    fixture: String,
    case_name: String,
    passed: bool,
    detail: String,
}
