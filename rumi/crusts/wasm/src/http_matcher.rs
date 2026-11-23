//! Config-driven HTTP matcher exposed to TypeScript via wasm-bindgen.
//!
//! Takes a JSON config string, compiles it via Rust registry, and evaluates
//! against HTTP request contexts passed as plain JS objects.

use std::sync::OnceLock;

use rumi::prelude::*;
use rumi_http::{HttpMessage, HttpMessageBuilder};
use wasm_bindgen::prelude::*;

use rumi_proto::index::ActionIndex;

use crate::matcher::Actions;

/// An opaque compiled HTTP matcher.
///
/// Created via `HttpMatcher.fromConfig()`, immutable after construction.
/// Evaluates HTTP requests against compiled matcher trees.
#[wasm_bindgen]
pub struct HttpMatcher {
    inner: Matcher<HttpMessage, ActionIndex>,
    actions: Actions,
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

        let loaded = crate::protojson::load_indexed(registry(), config)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let (inner, actions) = Actions::split(loaded);

        Ok(Self { inner, actions })
    }

    /// Evaluate an HTTP request against compiled matcher rules.
    ///
    /// Accepts a plain object:
    /// ```js
    /// matcher.evaluate({
    ///   method: "GET",
    ///   path: "/api/users",
    ///   headers: { "content-type": "application/json" },
    /// })
    /// ```
    ///
    /// The query string rides in `path`, as it does on the wire, and
    /// pseudo-headers such as `:authority` in `headers` (D-059).
    ///
    /// Returns the action string if the request matched, or `undefined`.
    #[wasm_bindgen(unchecked_return_type = "string | undefined")]
    pub fn evaluate(&self, context: JsValue) -> Result<JsValue, JsValue> {
        let req = build_request_from_js(&context)?;
        Ok(self.actions.action(self.inner.evaluate(&req)))
    }

    /// The decision as an object: `{ matched, action, index, fallback, undecided }`.
    pub fn decide(&self, context: JsValue) -> Result<JsValue, JsValue> {
        let req = build_request_from_js(&context)?;
        Ok(self.actions.decide(self.inner.outcome(&req)))
    }

    /// Every action this matcher can return, in index order.
    #[must_use]
    pub fn actions(&self) -> Vec<String> {
        self.actions.names()
    }

    /// The metadata carried by the action at `index`.
    #[wasm_bindgen(js_name = "actionMetadata")]
    pub fn action_metadata(&self, index: u32) -> Result<JsValue, JsValue> {
        self.actions.metadata(index)
    }

    /// Trace evaluation for debugging.
    pub fn trace(&self, context: JsValue) -> Result<JsValue, JsValue> {
        let req = build_request_from_js(&context)?;
        let trace = self.inner.evaluate_with_trace(&req);
        self.actions.trace(&trace)
    }
}

/// The registry every `HttpMatcher` loads through. Built on first use; every
/// call after that returns the same instance.
pub(crate) fn registry() -> &'static rumi::Registry<HttpMessage> {
    static REGISTRY: OnceLock<rumi::Registry<HttpMessage>> = OnceLock::new();
    REGISTRY.get_or_init(|| rumi_http::register(rumi::RegistryBuilder::new()).build())
}

/// Build an `HttpMessage` from a JS plain object.
fn build_request_from_js(val: &JsValue) -> Result<HttpMessage, JsValue> {
    let get =
        |key| js_sys::Reflect::get(val, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED);

    let method = get("method")
        .as_string()
        .ok_or_else(|| JsValue::from_str("method is required and must be a string"))?;

    let path = get("path")
        .as_string()
        .ok_or_else(|| JsValue::from_str("path is required and must be a string"))?;

    let mut builder = HttpMessageBuilder::new().method(method).path(path);

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

    Ok(builder.build())
}
