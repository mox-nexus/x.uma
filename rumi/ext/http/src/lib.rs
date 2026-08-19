//! rumi-http: HTTP domain for request matching
//!
//! This crate provides two layers:
//!
//! 1. **Simple API**: `HttpRequest` for testing and lightweight bindings
//! 2. **Data Plane API**: `HttpMessage` (indexed from `ProcessingRequest`) for production `ext_proc`
//!
//! The simple API is always available. The data plane API requires the `ext-proc` feature
//! (enabled by default).
//!
//! # Architecture
//!
//! ```text
//! Gateway API HttpRouteMatch (config)        [ext-proc feature]
//!         ↓ compile()
//! rumi Matcher<HttpMessage, A>               [ext-proc feature]
//!         ↓ evaluate()
//! HttpMessage (indexed from ext_proc ProcessingRequest)
//!
//! JSON MatcherConfig (config)                [registry feature]
//!         ↓ register_simple() + load_matcher()
//! rumi Matcher<HttpRequest, A>               [always available]
//!         ↓ evaluate()
//! HttpRequest (simple builder pattern)
//! ```

// Modules always available
mod simple;

// Three pure string functions and nothing else. They are ungated because both
// HTTP contexts must split a path the same way, and `simple` is ungated while
// `message` is not — a copy in each would be two answers to one question, which
// is the defect this shares them to remove.
mod context;

// The Gateway API compiler, the indexed context and its inputs. These need
// Gateway API *config types*, not a data plane — `HttpMessage` is six plain
// fields, and only its `From<ProcessingRequest>` conversions ever touched
// ext_proc. Conflating the two is what put tonic on the default path.
#[cfg(feature = "gateway")]
mod compiler;
#[cfg(feature = "message")]
mod inputs;
#[cfg(feature = "message")]
mod message;

// Simple types (always available)
pub use simple::{
    HttpRequest, HttpRequestBuilder, SimpleAuthorityInput, SimpleHeaderInput, SimpleMethodInput,
    SimplePathInput, SimpleQueryParamInput, SimpleSchemeInput,
};

// Registry for simple HttpRequest context (always available with registry)
#[cfg(feature = "registry")]
pub use simple::register_simple;

// Compiler, inputs and the indexed context — available with `gateway` alone.
#[cfg(feature = "gateway")]
pub use compiler::*;
#[cfg(feature = "message")]
pub use inputs::*;
#[cfg(feature = "message")]
pub use message::{HttpMessage, HttpMessageBuilder};

// Gateway API config types.
#[cfg(feature = "gateway")]
pub use k8s_gateway_api::{
    HttpHeaderMatch, HttpMethod, HttpPathMatch, HttpQueryParamMatch, HttpRouteMatch,
};

// The data plane. Only these actually need envoy-grpc-ext-proc.
#[cfg(feature = "ext-proc")]
pub use envoy_grpc_ext_proc::envoy::service::ext_proc::v3::{
    ProcessingRequest, ProcessingResponse,
};

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::{
        // Simple context + inputs (always available)
        HttpRequest,
        HttpRequestBuilder,
        SimpleAuthorityInput,
        SimpleHeaderInput,
        SimpleMethodInput,
        SimplePathInput,
        SimpleQueryParamInput,
        SimpleSchemeInput,
    };

    // The domain: indexed context and its DataInputs.
    #[cfg(feature = "message")]
    pub use super::{
        AuthorityInput, HeaderInput, HttpMessage, HttpMessageBuilder, MethodInput, PathInput,
        QueryParamInput, SchemeInput,
    };

    // Gateway API config types and the compiler.
    #[cfg(feature = "gateway")]
    pub use super::{
        HttpHeaderMatch, HttpMethod, HttpPathMatch, HttpQueryParamMatch, HttpRouteMatchExt,
    };

    // The data-plane types are the only things that need ext_proc.
    #[cfg(feature = "ext-proc")]
    pub use super::{ProcessingRequest, ProcessingResponse};

    pub use rumi::prelude::*;
}

/// Register all rumi-http types for [`HttpMessage`] with the given builder.
///
/// Registers core matchers (`BoolMatcher`, `StringMatcher`) and HTTP-domain inputs:
/// - `xuma.http.v1.PathInput` → [`PathInput`]
/// - `xuma.http.v1.MethodInput` → [`MethodInput`]
/// - `xuma.http.v1.HeaderInput` → [`HeaderInput`]
/// - `xuma.http.v1.QueryParamInput` → [`QueryParamInput`]
/// - `xuma.http.v1.SchemeInput` → [`SchemeInput`]
/// - `xuma.http.v1.AuthorityInput` → [`AuthorityInput`]
#[cfg(all(feature = "message", feature = "registry"))]
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

#[cfg(all(test, feature = "registry"))]
mod proto_tests {
    use super::*;
    use rumi::{
        FieldMatcherConfig, MatcherConfig, OnMatchConfig, PredicateConfig, SinglePredicateConfig,
        StringMatchSpec, TypedConfig, ValueMatchConfig,
    };

    // Built via HttpMessageBuilder rather than an ext_proc ProcessingRequest.
    // `proto` is a control-plane feature and has no business requiring the data
    // plane: it was only reaching `gateway` transitively through `ext-proc`,
    // which dragged tonic and tokio into a config-loading build.
    fn build_request(headers: Vec<(&str, &str)>) -> HttpMessage {
        headers
            .into_iter()
            .fold(HttpMessageBuilder::new(), |b, (k, v)| b.header(k, v))
            .build()
    }

    // `MatcherConfig` and friends are built, never parsed — see rumi's
    // `config` module.

    fn input(type_url: &str, config: serde_json::Value) -> TypedConfig {
        TypedConfig {
            type_url: type_url.into(),
            config,
        }
    }

    fn built_in(input: TypedConfig, spec: StringMatchSpec) -> PredicateConfig {
        PredicateConfig::Single(SinglePredicateConfig {
            input,
            matcher: ValueMatchConfig::BuiltIn {
                spec,
                ignore_case: false,
            },
        })
    }

    fn act(action: &str) -> OnMatchConfig<String> {
        OnMatchConfig::Action {
            action: action.into(),
        }
    }

    fn one(predicate: PredicateConfig, on_match: OnMatchConfig<String>) -> MatcherConfig<String> {
        MatcherConfig::list(vec![FieldMatcherConfig {
            predicate,
            on_match,
        }])
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
    }

    #[test]
    fn load_matcher_with_proto_path_input() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        // PathInput is an empty proto message — no config fields
        let config = MatcherConfig::list(vec![FieldMatcherConfig {
            predicate: built_in(
                input("xuma.http.v1.PathInput", serde_json::json!({})),
                StringMatchSpec::Prefix("/api".into()),
            ),
            on_match: act("api_backend"),
        }])
        .with_fallback(Some(act("default")));
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
        let config = one(
            built_in(
                input(
                    "xuma.http.v1.HeaderInput",
                    serde_json::json!({ "name": "content-type" }),
                ),
                StringMatchSpec::Exact("application/json".into()),
            ),
            act("json_handler"),
        );
        let matcher = registry.load_matcher(config).unwrap();

        let msg = build_request(vec![("content-type", "application/json")]);
        assert_eq!(matcher.evaluate(&msg), Some("json_handler".to_string()));

        let msg = build_request(vec![("content-type", "text/html")]);
        assert_eq!(matcher.evaluate(&msg), None);
    }

    #[test]
    fn load_matcher_with_and_path_and_method() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        let config = one(
            PredicateConfig::And {
                predicates: vec![
                    built_in(
                        input("xuma.http.v1.PathInput", serde_json::json!({})),
                        StringMatchSpec::Prefix("/api".into()),
                    ),
                    built_in(
                        input("xuma.http.v1.MethodInput", serde_json::json!({})),
                        StringMatchSpec::Exact("POST".into()),
                    ),
                ],
            },
            act("api_write"),
        );
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

/// The crate README's Rust blocks, compiled and run as doctests.
///
/// A crate README is a published artifact — it is the crates.io front page —
/// and until 2026-08-18 nothing checked one. `rumi-core`'s showed a config
/// dialect that had been retired, so the first thing a new reader would have
/// copied could not load. Nothing failed, because nothing looked.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;
