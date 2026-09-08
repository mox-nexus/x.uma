//! The key-value matcher exposed to TypeScript via wasm-bindgen.
//!
//! Compiles a protojson document over `xuma.kv.v1` inputs and evaluates
//! key-value contexts: the domain the conformance suite runs in.

use std::sync::OnceLock;

use rumi::prelude::*;
use rumi_kv::KvContext;
use wasm_bindgen::prelude::*;

use crate::matcher::TraceResultSerde;

/// An opaque compiled key-value matcher.
///
/// Created via `KvMatcher.fromConfig()`, immutable after construction.
/// Evaluates key-value contexts against compiled matcher trees.
#[wasm_bindgen]
pub struct KvMatcher {
    inner: Matcher<KvContext, String>,
}

#[wasm_bindgen]
impl KvMatcher {
    /// Load a matcher from a canonical protojson config string.
    ///
    /// protojson is the format all x.uma implementations use — protobuf's own
    /// JSON mapping of `xds.type.matcher.v3.Matcher`. See DECISIONS.md D-026.
    ///
    /// # Supported input type URLs
    ///
    /// - `xuma.kv.v1.MapInput` — string lookup by key (`{"key": "..."}`)
    #[wasm_bindgen(js_name = "fromConfig")]
    pub fn from_config(json_config: &str) -> Result<KvMatcher, JsValue> {
        let config = crate::protojson::load(json_config).map_err(|e| JsValue::from_str(&e))?;

        let matcher = registry()
            .load_typed_matcher(config, crate::protojson::actions())
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

        let steps = super::matcher::trace_steps(&trace.steps);

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
    /// Runs `spec/tests/07_protojson/` — canonical protojson, through the same
    /// reader the engine uses, so a passing suite is a claim about this
    /// binding's door and not about a second parser.
    #[wasm_bindgen(js_name = "runFixtures")]
    pub fn run_fixtures(yaml_content: &str) -> Result<JsValue, JsValue> {
        let results: Vec<FixtureResultSerde> = crate::fixtures::run_protojson(
            yaml_content,
            rumi_test::implementations::Implementation::Typescript,
        )
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

/// The registry every `KvMatcher` loads through. Built on first use; every
/// call after that returns the same instance.
pub(crate) fn registry() -> &'static rumi::Registry<KvContext> {
    static REGISTRY: OnceLock<rumi::Registry<KvContext>> = OnceLock::new();
    REGISTRY.get_or_init(|| rumi_kv::register(rumi::RegistryBuilder::new()).build())
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
