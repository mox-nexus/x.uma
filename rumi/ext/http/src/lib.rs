//! rumi-http: HTTP domain for request matching
//!
//! This crate provides two layers:
//!
//! 1. **User API**: Gateway API `HttpRouteMatch` for configuration
//! 2. **Data Plane API**: `HttpMessage` (indexed from `ProcessingRequest`) for runtime
//!
//! # Architecture
//!
//! ```text
//! Gateway API HttpRouteMatch (config)
//!         ↓ compile()
//! rumi Matcher<HttpMessage, A>
//!         ↓ evaluate()
//! HttpMessage (indexed from ext_proc ProcessingRequest)
//! ```
//!
//! # Example
//!
//! ```ignore
//! use rumi_http::prelude::*;
//!
//! // Config time: compile Gateway API match to rumi matcher
//! let route_match = HttpRouteMatch {
//!     path: Some(HttpPathMatch::PathPrefix { value: "/api".into() }),
//!     ..Default::default()
//! };
//! let matcher = route_match.compile::<&str>("api_backend");
//!
//! // Runtime: index ProcessingRequest into HttpMessage, then evaluate
//! let msg = HttpMessage::from(&processing_request);
//! let result = matcher.evaluate(&msg);
//! ```

mod compiler;
mod context;
mod inputs;
mod message;
mod simple;

pub use compiler::*;
pub use inputs::*;
pub use message::HttpMessage;
pub use simple::{
    HttpRequest, HttpRequestBuilder, SimpleHeaderInput, SimpleMethodInput, SimplePathInput,
    SimpleQueryParamInput,
};

// Re-export ext_proc types for convenience
pub use envoy_grpc_ext_proc::envoy::service::ext_proc::v3::{
    ProcessingRequest, ProcessingResponse,
};

// Re-export Gateway API types for convenience
pub use k8s_gateway_api::{
    HttpHeaderMatch, HttpMethod, HttpPathMatch, HttpQueryParamMatch, HttpRouteMatch,
};

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::{
        // DataInputs for HttpMessage
        AuthorityInput,
        HeaderInput,
        // Re-exports: Gateway API
        HttpHeaderMatch,
        // Indexed context
        HttpMessage,
        HttpMethod,
        HttpPathMatch,
        HttpQueryParamMatch,
        // Simple context + inputs (for testing)
        HttpRequest,
        HttpRequestBuilder,
        HttpRouteMatch,
        // Compiler
        HttpRouteMatchExt,
        MethodInput,
        PathInput,
        // Re-exports: ext_proc
        ProcessingRequest,
        ProcessingResponse,
        QueryParamInput,
        SchemeInput,
        SimpleHeaderInput,
        SimpleMethodInput,
        SimplePathInput,
        SimpleQueryParamInput,
    };
    pub use rumi::prelude::*;
}

// Registry config types (hand-written, only without proto)
#[cfg(all(feature = "registry", not(feature = "proto")))]
pub use inputs::{HeaderInputConfig, QueryParamInputConfig};

/// Register all rumi-http types for [`HttpMessage`] with the given builder.
///
/// Registers core matchers (`BoolMatcher`, `StringMatcher`) and HTTP-domain inputs:
/// - `xuma.http.v1.PathInput` → [`PathInput`]
/// - `xuma.http.v1.MethodInput` → [`MethodInput`]
/// - `xuma.http.v1.HeaderInput` → [`HeaderInput`]
/// - `xuma.http.v1.QueryParamInput` → [`QueryParamInput`]
/// - `xuma.http.v1.SchemeInput` → [`SchemeInput`]
/// - `xuma.http.v1.AuthorityInput` → [`AuthorityInput`]
#[cfg(feature = "registry")]
#[must_use]
pub fn register(builder: rumi::RegistryBuilder<HttpMessage>) -> rumi::RegistryBuilder<HttpMessage> {
    rumi::register_core_matchers(builder)
        .input::<PathInput>("xuma.http.v1.PathInput")
        .input::<MethodInput>("xuma.http.v1.MethodInput")
        .input::<HeaderInput>("xuma.http.v1.HeaderInput")
        .input::<QueryParamInput>("xuma.http.v1.QueryParamInput")
        .input::<SchemeInput>("xuma.http.v1.SchemeInput")
        .input::<AuthorityInput>("xuma.http.v1.AuthorityInput")
}

// ═══════════════════════════════════════════════════════════════════════════════
// Proto registry integration tests
// Verifies: proto config → register() → load_matcher → evaluate on HttpMessage
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(all(test, feature = "proto"))]
mod proto_tests {
    use super::*;
    use envoy_grpc_ext_proc::envoy::{
        config::core::v3::{HeaderMap, HeaderValue},
        service::ext_proc::v3::{processing_request::Request, HttpHeaders},
    };
    use rumi::MatcherConfig;

    fn build_request(headers: Vec<(&str, &str)>) -> HttpMessage {
        let req = ProcessingRequest {
            request: Some(Request::RequestHeaders(HttpHeaders {
                headers: Some(HeaderMap {
                    headers: headers
                        .into_iter()
                        .map(|(k, v)| HeaderValue {
                            key: k.into(),
                            value: v.into(),
                            raw_value: vec![],
                        })
                        .collect(),
                }),
                ..Default::default()
            })),
            ..Default::default()
        };
        HttpMessage::from(&req)
    }

    #[test]
    fn register_builds_with_proto_configs() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        assert!(registry.contains_input("xuma.http.v1.PathInput"));
        assert!(registry.contains_input("xuma.http.v1.MethodInput"));
        assert!(registry.contains_input("xuma.http.v1.HeaderInput"));
        assert!(registry.contains_input("xuma.http.v1.QueryParamInput"));
        assert!(registry.contains_input("xuma.http.v1.SchemeInput"));
        assert!(registry.contains_input("xuma.http.v1.AuthorityInput"));
        assert!(registry.contains_matcher("xuma.core.v1.StringMatcher"));
    }

    #[test]
    fn load_matcher_with_proto_path_input() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        // PathInput is an empty proto message — no config fields
        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.http.v1.PathInput",
                        "config": {}
                    },
                    "value_match": { "Prefix": "/api" }
                },
                "on_match": { "type": "action", "action": "api_backend" }
            }],
            "on_no_match": { "type": "action", "action": "default" }
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        let matcher = registry.load_matcher(config).unwrap();

        let msg = build_request(vec![(":path", "/api/users"), (":method", "GET")]);
        assert_eq!(matcher.evaluate(&msg), Some("api_backend".to_string()));

        let msg = build_request(vec![(":path", "/health"), (":method", "GET")]);
        assert_eq!(matcher.evaluate(&msg), Some("default".to_string()));
    }

    #[test]
    fn load_matcher_with_proto_header_input() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        // HeaderInput config has "name" field (the header name to extract)
        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.http.v1.HeaderInput",
                        "config": { "name": "content-type" }
                    },
                    "value_match": { "Exact": "application/json" }
                },
                "on_match": { "type": "action", "action": "json_handler" }
            }]
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        let matcher = registry.load_matcher(config).unwrap();

        let msg = build_request(vec![("content-type", "application/json")]);
        assert_eq!(matcher.evaluate(&msg), Some("json_handler".to_string()));

        let msg = build_request(vec![("content-type", "text/html")]);
        assert_eq!(matcher.evaluate(&msg), None);
    }

    #[test]
    fn load_matcher_with_and_path_and_method() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "and",
                    "predicates": [
                        {
                            "type": "single",
                            "input": {
                                "type_url": "xuma.http.v1.PathInput",
                                "config": {}
                            },
                            "value_match": { "Prefix": "/api" }
                        },
                        {
                            "type": "single",
                            "input": {
                                "type_url": "xuma.http.v1.MethodInput",
                                "config": {}
                            },
                            "value_match": { "Exact": "POST" }
                        }
                    ]
                },
                "on_match": { "type": "action", "action": "api_write" }
            }]
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        let matcher = registry.load_matcher(config).unwrap();

        let msg = build_request(vec![(":path", "/api/users"), (":method", "POST")]);
        assert_eq!(matcher.evaluate(&msg), Some("api_write".to_string()));

        // GET doesn't match POST
        let msg = build_request(vec![(":path", "/api/users"), (":method", "GET")]);
        assert_eq!(matcher.evaluate(&msg), None);

        // Wrong path
        let msg = build_request(vec![(":path", "/health"), (":method", "POST")]);
        assert_eq!(matcher.evaluate(&msg), None);
    }
}
