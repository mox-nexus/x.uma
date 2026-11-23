//! rumi-http: the HTTP matching domain.
//!
//! One context, [`HttpMessage`]: a request as it arrives on the wire, with the
//! query string inside `:path` and `:authority` / `:scheme` as pseudo-headers.
//! Three adapters build or target it, each behind its own feature:
//!
//! ```text
//! protojson config  --registry-->  register() + load_matcher()  --+
//! Gateway API       --gateway--->  compile_route_matches()     ---+--> Matcher<HttpMessage, A>
//! ext_proc          --ext-proc-->  From<ProcessingRequest> for HttpMessage
//! ```
//!
//! There was a second context, `HttpRequest`, with its own registry over the
//! same `xuma.http.v1.*` type URLs; that pair once gave `PathInput` two meanings
//! depending on the caller. One context removes the class (D-059).

// Path and query string parsing, used by the one context.
mod context;
mod inputs;
mod message;

// The Gateway API compiler. Needs Gateway API *config types*, not a data
// plane: only `From<ProcessingRequest>` ever touched ext_proc.
#[cfg(feature = "gateway")]
mod compiler;

pub use inputs::*;
pub use message::{HttpMessage, HttpMessageBuilder};

#[cfg(feature = "gateway")]
pub use compiler::*;

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
    // The domain: the context and its DataInputs.
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

#[cfg(all(test, feature = "registry"))]
mod proto_tests {
    use super::*;
    use rumi::{
        FieldMatcherIr, MatcherIr, OnMatchIr, PredicateIr, SinglePredicateIr, StringMatchSpec,
        TypedExtensionConfig, ValueMatchIr,
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

    // `MatcherIr` and friends are built, never parsed — see rumi's
    // `config` module.

    fn input(type_url: &str, config: serde_json::Value) -> TypedExtensionConfig {
        TypedExtensionConfig {
            type_url: type_url.into(),
            config,
        }
    }

    fn built_in(input: TypedExtensionConfig, spec: StringMatchSpec) -> PredicateIr {
        PredicateIr::Single(SinglePredicateIr {
            input,
            matcher: ValueMatchIr::BuiltIn {
                spec,
                ignore_case: false,
            },
        })
    }

    fn act(action: &str) -> OnMatchIr<String> {
        OnMatchIr::Action {
            action: action.into(),
        }
    }

    fn one(predicate: PredicateIr, on_match: OnMatchIr<String>) -> MatcherIr<String> {
        MatcherIr::list(vec![FieldMatcherIr {
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
        let config = MatcherIr::list(vec![FieldMatcherIr {
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
            PredicateIr::And {
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

/// The crate README's Rust blocks, compiled and run as doctests: the README is the
/// crates.io front page, so its examples must load.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;
