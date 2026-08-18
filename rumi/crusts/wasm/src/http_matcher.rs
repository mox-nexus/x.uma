//! Config-driven HTTP matcher exposed to TypeScript via wasm-bindgen.
//!
//! Takes a JSON config string, compiles it via Rust registry, and evaluates
//! against HTTP request contexts passed as plain JS objects.

use rumi::prelude::*;
use rumi_http::HttpRequest;
use wasm_bindgen::prelude::*;

use crate::matcher::{TraceResultSerde, TraceStepSerde};

/// An opaque compiled HTTP matcher.
///
/// Created via `HttpMatcher.fromConfig()`, immutable after construction.
/// Evaluates HTTP requests against compiled matcher trees.
#[wasm_bindgen]
pub struct HttpMatcher {
    inner: Matcher<HttpRequest, String>,
}

#[wasm_bindgen]
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
    #[wasm_bindgen(js_name = "fromConfig")]
    pub fn from_config(json_config: &str) -> Result<HttpMatcher, JsValue> {
        let config = crate::protojson::load(json_config).map_err(|e| JsValue::from_str(&e))?;

        let registry = build_http_registry();
        let matcher = registry
            .load_typed_matcher(config, &crate::protojson::actions())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        matcher
            .validate()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(Self { inner: matcher })
    }

    /// Evaluate an HTTP request against compiled matcher rules.
    ///
    /// Accepts a plain object:
    /// ```js
    /// matcher.evaluate({
    ///   method: "GET",
    ///   path: "/api/users",
    ///   headers: { "content-type": "application/json" },
    ///   queryParams: { "page": "1" },
    /// })
    /// ```
    ///
    /// Returns the action string if the request matched, or `undefined`.
    pub fn evaluate(&self, context: JsValue) -> Result<Option<String>, JsValue> {
        let req = build_request_from_js(&context)?;
        Ok(self.inner.evaluate(&req))
    }

    /// Trace evaluation for debugging.
    pub fn trace(&self, context: JsValue) -> Result<JsValue, JsValue> {
        let req = build_request_from_js(&context)?;
        let trace = self.inner.evaluate_with_trace(&req);

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
}

/// Build the HTTP registry for `HttpRequest`.
fn build_http_registry() -> rumi::Registry<HttpRequest> {
    rumi_http::register_simple(rumi::RegistryBuilder::new()).build()
}

/// Build an `HttpRequest` from a JS plain object.
fn build_request_from_js(val: &JsValue) -> Result<HttpRequest, JsValue> {
    let get =
        |key| js_sys::Reflect::get(val, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED);

    let method = get("method")
        .as_string()
        .ok_or_else(|| JsValue::from_str("method is required and must be a string"))?;

    let path = get("path")
        .as_string()
        .ok_or_else(|| JsValue::from_str("path is required and must be a string"))?;

    let mut builder = HttpRequest::builder().method(method).path(path);

    // Headers (optional, Record<string, string>)
    let headers_val = get("headers");
    if !headers_val.is_undefined() && !headers_val.is_null() {
        let entries = js_sys::Object::entries(&js_sys::Object::from(headers_val));
        for entry in entries.iter() {
            let pair = js_sys::Array::from(&entry);
            let key = pair
                .get(0)
                .as_string()
                .ok_or_else(|| JsValue::from_str("header key must be a string"))?;
            let value = pair
                .get(1)
                .as_string()
                .ok_or_else(|| JsValue::from_str("header value must be a string"))?;
            builder = builder.header(key, value);
        }
    }

    // Query params (optional, Record<string, string>)
    let params_val = get("queryParams");
    if !params_val.is_undefined() && !params_val.is_null() {
        let entries = js_sys::Object::entries(&js_sys::Object::from(params_val));
        for entry in entries.iter() {
            let pair = js_sys::Array::from(&entry);
            let key = pair
                .get(0)
                .as_string()
                .ok_or_else(|| JsValue::from_str("query param key must be a string"))?;
            let value = pair
                .get(1)
                .as_string()
                .ok_or_else(|| JsValue::from_str("query param value must be a string"))?;
            builder = builder.query_param(key, value);
        }
    }

    Ok(builder.build())
}
