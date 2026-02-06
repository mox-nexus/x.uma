//! `DataInput` implementations for `HttpMessage`.
//!
//! These extract HTTP data from the indexed `HttpMessage` for matching.
//! All lookups are O(1) thanks to the pre-built index.

use crate::message::HttpMessage;
use rumi::prelude::*;

/// Extracts the request path (without query string) from `HttpMessage`.
///
/// Maps to the `:path` pseudo-header, with query string stripped.
#[derive(Debug, Clone, Default)]
pub struct PathInput;

impl DataInput<HttpMessage> for PathInput {
    fn get(&self, ctx: &HttpMessage) -> MatchingData {
        ctx.path()
            .map_or(MatchingData::None, |p| MatchingData::String(p.to_string()))
    }
}

/// Extracts the HTTP method from `HttpMessage`.
///
/// Maps to the `:method` pseudo-header.
#[derive(Debug, Clone, Default)]
pub struct MethodInput;

impl DataInput<HttpMessage> for MethodInput {
    fn get(&self, ctx: &HttpMessage) -> MatchingData {
        ctx.method()
            .map_or(MatchingData::None, |m| MatchingData::String(m.to_string()))
    }
}

/// Extracts a header value from `HttpMessage`.
///
/// Header names are matched case-insensitively.
#[derive(Debug, Clone)]
pub struct HeaderInput {
    name: String,
}

impl HeaderInput {
    /// Create a new header input extractor.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl DataInput<HttpMessage> for HeaderInput {
    fn get(&self, ctx: &HttpMessage) -> MatchingData {
        ctx.header(&self.name)
            .map_or(MatchingData::None, |v| MatchingData::String(v.to_string()))
    }
}

/// Extracts a query parameter value from `HttpMessage`.
///
/// Parses the query string from the `:path` pseudo-header.
#[derive(Debug, Clone)]
pub struct QueryParamInput {
    name: String,
}

impl QueryParamInput {
    /// Create a new query parameter input extractor.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl DataInput<HttpMessage> for QueryParamInput {
    fn get(&self, ctx: &HttpMessage) -> MatchingData {
        ctx.query_param(&self.name)
            .map_or(MatchingData::None, |v| MatchingData::String(v.to_string()))
    }
}

/// Extracts the :scheme pseudo-header from `HttpMessage`.
#[derive(Debug, Clone, Default)]
pub struct SchemeInput;

impl DataInput<HttpMessage> for SchemeInput {
    fn get(&self, ctx: &HttpMessage) -> MatchingData {
        ctx.scheme()
            .map_or(MatchingData::None, |s| MatchingData::String(s.to_string()))
    }
}

/// Extracts the :authority pseudo-header from `HttpMessage`.
#[derive(Debug, Clone, Default)]
pub struct AuthorityInput;

impl DataInput<HttpMessage> for AuthorityInput {
    fn get(&self, ctx: &HttpMessage) -> MatchingData {
        ctx.authority()
            .map_or(MatchingData::None, |a| MatchingData::String(a.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use envoy_grpc_ext_proc::envoy::{
        config::core::v3::{HeaderMap, HeaderValue},
        service::ext_proc::v3::{processing_request::Request, HttpHeaders, ProcessingRequest},
    };

    /// Builder for constructing test `ProcessingRequest` instances.
    struct ProcessingRequestBuilder {
        headers: Vec<HeaderValue>,
    }

    impl ProcessingRequestBuilder {
        fn new() -> Self {
            Self { headers: vec![] }
        }

        fn path(mut self, path: &str) -> Self {
            self.headers.push(HeaderValue {
                key: ":path".into(),
                value: path.into(),
                raw_value: vec![],
            });
            self
        }

        fn method(mut self, method: &str) -> Self {
            self.headers.push(HeaderValue {
                key: ":method".into(),
                value: method.into(),
                raw_value: vec![],
            });
            self
        }

        fn scheme(mut self, scheme: &str) -> Self {
            self.headers.push(HeaderValue {
                key: ":scheme".into(),
                value: scheme.into(),
                raw_value: vec![],
            });
            self
        }

        fn authority(mut self, authority: &str) -> Self {
            self.headers.push(HeaderValue {
                key: ":authority".into(),
                value: authority.into(),
                raw_value: vec![],
            });
            self
        }

        fn header(mut self, name: &str, value: &str) -> Self {
            self.headers.push(HeaderValue {
                key: name.to_lowercase(),
                value: value.into(),
                raw_value: vec![],
            });
            self
        }

        fn build(self) -> HttpMessage {
            let req = ProcessingRequest {
                request: Some(Request::RequestHeaders(HttpHeaders {
                    headers: Some(HeaderMap {
                        headers: self.headers,
                    }),
                    ..Default::default()
                })),
                ..Default::default()
            };
            HttpMessage::from(&req)
        }
    }

    // ========== PathInput Tests ==========

    #[test]
    fn path_input_extracts_simple_path() {
        let msg = ProcessingRequestBuilder::new().path("/api/users").build();
        let input = PathInput;
        assert_eq!(input.get(&msg), MatchingData::String("/api/users".into()));
    }

    #[test]
    fn path_input_strips_query_string() {
        let msg = ProcessingRequestBuilder::new()
            .path("/api/users?page=1&limit=10")
            .build();
        let input = PathInput;
        assert_eq!(input.get(&msg), MatchingData::String("/api/users".into()));
    }

    #[test]
    fn path_input_returns_none_when_missing() {
        let msg = ProcessingRequestBuilder::new().method("GET").build();
        let input = PathInput;
        assert_eq!(input.get(&msg), MatchingData::None);
    }

    #[test]
    fn path_input_handles_root_path() {
        let msg = ProcessingRequestBuilder::new().path("/").build();
        let input = PathInput;
        assert_eq!(input.get(&msg), MatchingData::String("/".into()));
    }

    #[test]
    fn path_input_handles_empty_query_string() {
        let msg = ProcessingRequestBuilder::new().path("/api?").build();
        let input = PathInput;
        assert_eq!(input.get(&msg), MatchingData::String("/api".into()));
    }

    // ========== MethodInput Tests ==========

    #[test]
    fn method_input_extracts_get() {
        let msg = ProcessingRequestBuilder::new().method("GET").build();
        let input = MethodInput;
        assert_eq!(input.get(&msg), MatchingData::String("GET".into()));
    }

    #[test]
    fn method_input_extracts_post() {
        let msg = ProcessingRequestBuilder::new().method("POST").build();
        let input = MethodInput;
        assert_eq!(input.get(&msg), MatchingData::String("POST".into()));
    }

    #[test]
    fn method_input_returns_none_when_missing() {
        let msg = ProcessingRequestBuilder::new().path("/").build();
        let input = MethodInput;
        assert_eq!(input.get(&msg), MatchingData::None);
    }

    // ========== HeaderInput Tests ==========

    #[test]
    fn header_input_extracts_content_type() {
        let msg = ProcessingRequestBuilder::new()
            .header("content-type", "application/json")
            .build();
        let input = HeaderInput::new("content-type");
        assert_eq!(
            input.get(&msg),
            MatchingData::String("application/json".into())
        );
    }

    #[test]
    fn header_input_case_insensitive() {
        let msg = ProcessingRequestBuilder::new()
            .header("x-custom-header", "value123")
            .build();

        let input = HeaderInput::new("X-Custom-Header");
        assert_eq!(input.get(&msg), MatchingData::String("value123".into()));
    }

    #[test]
    fn header_input_returns_none_when_missing() {
        let msg = ProcessingRequestBuilder::new()
            .header("content-type", "text/plain")
            .build();
        let input = HeaderInput::new("authorization");
        assert_eq!(input.get(&msg), MatchingData::None);
    }

    #[test]
    fn header_input_extracts_authorization() {
        let msg = ProcessingRequestBuilder::new()
            .header("authorization", "Bearer token123")
            .build();
        let input = HeaderInput::new("authorization");
        assert_eq!(
            input.get(&msg),
            MatchingData::String("Bearer token123".into())
        );
    }

    // ========== QueryParamInput Tests ==========

    #[test]
    fn query_param_input_extracts_single_param() {
        let msg = ProcessingRequestBuilder::new()
            .path("/search?q=rust")
            .build();
        let input = QueryParamInput::new("q");
        assert_eq!(input.get(&msg), MatchingData::String("rust".into()));
    }

    #[test]
    fn query_param_input_extracts_from_multiple() {
        let msg = ProcessingRequestBuilder::new()
            .path("/api?page=1&limit=10&sort=name")
            .build();

        assert_eq!(
            QueryParamInput::new("page").get(&msg),
            MatchingData::String("1".into())
        );
        assert_eq!(
            QueryParamInput::new("limit").get(&msg),
            MatchingData::String("10".into())
        );
        assert_eq!(
            QueryParamInput::new("sort").get(&msg),
            MatchingData::String("name".into())
        );
    }

    #[test]
    fn query_param_input_returns_none_when_missing() {
        let msg = ProcessingRequestBuilder::new().path("/api?page=1").build();
        let input = QueryParamInput::new("limit");
        assert_eq!(input.get(&msg), MatchingData::None);
    }

    #[test]
    fn query_param_input_returns_none_when_no_query_string() {
        let msg = ProcessingRequestBuilder::new().path("/api").build();
        let input = QueryParamInput::new("page");
        assert_eq!(input.get(&msg), MatchingData::None);
    }

    #[test]
    fn query_param_input_returns_none_when_no_path() {
        let msg = ProcessingRequestBuilder::new().method("GET").build();
        let input = QueryParamInput::new("page");
        assert_eq!(input.get(&msg), MatchingData::None);
    }

    // ========== SchemeInput Tests ==========

    #[test]
    fn scheme_input_extracts_https() {
        let msg = ProcessingRequestBuilder::new().scheme("https").build();
        let input = SchemeInput;
        assert_eq!(input.get(&msg), MatchingData::String("https".into()));
    }

    #[test]
    fn scheme_input_extracts_http() {
        let msg = ProcessingRequestBuilder::new().scheme("http").build();
        let input = SchemeInput;
        assert_eq!(input.get(&msg), MatchingData::String("http".into()));
    }

    #[test]
    fn scheme_input_returns_none_when_missing() {
        let msg = ProcessingRequestBuilder::new().path("/").build();
        let input = SchemeInput;
        assert_eq!(input.get(&msg), MatchingData::None);
    }

    // ========== AuthorityInput Tests ==========

    #[test]
    fn authority_input_extracts_host() {
        let msg = ProcessingRequestBuilder::new()
            .authority("example.com")
            .build();
        let input = AuthorityInput;
        assert_eq!(input.get(&msg), MatchingData::String("example.com".into()));
    }

    #[test]
    fn authority_input_extracts_host_with_port() {
        let msg = ProcessingRequestBuilder::new()
            .authority("example.com:8080")
            .build();
        let input = AuthorityInput;
        assert_eq!(
            input.get(&msg),
            MatchingData::String("example.com:8080".into())
        );
    }

    #[test]
    fn authority_input_returns_none_when_missing() {
        let msg = ProcessingRequestBuilder::new().path("/").build();
        let input = AuthorityInput;
        assert_eq!(input.get(&msg), MatchingData::None);
    }

    // ========== Combined/Integration Tests ==========

    #[test]
    fn full_request_all_inputs() {
        let msg = ProcessingRequestBuilder::new()
            .method("POST")
            .path("/api/v1/users?page=1&limit=20")
            .scheme("https")
            .authority("api.example.com")
            .header("content-type", "application/json")
            .header("authorization", "Bearer abc123")
            .build();

        assert_eq!(
            PathInput.get(&msg),
            MatchingData::String("/api/v1/users".into())
        );
        assert_eq!(MethodInput.get(&msg), MatchingData::String("POST".into()));
        assert_eq!(SchemeInput.get(&msg), MatchingData::String("https".into()));
        assert_eq!(
            AuthorityInput.get(&msg),
            MatchingData::String("api.example.com".into())
        );
        assert_eq!(
            QueryParamInput::new("page").get(&msg),
            MatchingData::String("1".into())
        );
        assert_eq!(
            QueryParamInput::new("limit").get(&msg),
            MatchingData::String("20".into())
        );
        assert_eq!(
            HeaderInput::new("content-type").get(&msg),
            MatchingData::String("application/json".into())
        );
        assert_eq!(
            HeaderInput::new("authorization").get(&msg),
            MatchingData::String("Bearer abc123".into())
        );
    }

    #[test]
    fn empty_processing_request_returns_none() {
        let msg = HttpMessage::from(&ProcessingRequest::default());

        assert_eq!(PathInput.get(&msg), MatchingData::None);
        assert_eq!(MethodInput.get(&msg), MatchingData::None);
        assert_eq!(SchemeInput.get(&msg), MatchingData::None);
        assert_eq!(AuthorityInput.get(&msg), MatchingData::None);
        assert_eq!(HeaderInput::new("any").get(&msg), MatchingData::None);
        assert_eq!(QueryParamInput::new("any").get(&msg), MatchingData::None);
    }
}
