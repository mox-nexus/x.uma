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
    ///
    /// # Errors
    ///
    /// [`MatcherError::EmptyIdentifier`](rumi::MatcherError::EmptyIdentifier)
    /// if `name` is empty. An empty header name reads no header, so the
    /// predicate is always false and a rule keyed on it silently stops firing
    /// — a deny rule that never denies. Rejecting it here rather than in the
    /// config loader means every route inherits the check: the proto config
    /// path, the compiler, the FFI bindings and direct construction.
    pub fn new(name: impl Into<String>) -> Result<Self, rumi::MatcherError> {
        let name = name.into();
        if name.is_empty() {
            return Err(rumi::MatcherError::EmptyIdentifier {
                what: "header name",
            });
        }
        Ok(Self { name })
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
    ///
    /// # Errors
    ///
    /// [`MatcherError::EmptyIdentifier`](rumi::MatcherError::EmptyIdentifier)
    /// if `name` is empty — see [`HeaderInput::new`] for why this is a
    /// constructor's job.
    pub fn new(name: impl Into<String>) -> Result<Self, rumi::MatcherError> {
        let name = name.into();
        if name.is_empty() {
            return Err(rumi::MatcherError::EmptyIdentifier {
                what: "query parameter name",
            });
        }
        Ok(Self { name })
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

// ═══════════════════════════════════════════════════════════════════════════════
// Registry support (feature = "registry")
// Hand-written config types — used when proto feature is not enabled.
// ═══════════════════════════════════════════════════════════════════════════════

/// Configuration for [`HeaderInput`].
#[cfg(all(feature = "registry", not(feature = "proto")))]
#[derive(serde::Deserialize)]
pub struct HeaderInputConfig {
    /// The header name to extract (case-insensitive).
    pub name: String,
}

/// Configuration for [`QueryParamInput`].
#[cfg(all(feature = "registry", not(feature = "proto")))]
#[derive(serde::Deserialize)]
pub struct QueryParamInputConfig {
    /// The query parameter name to extract.
    pub name: String,
}

#[cfg(all(feature = "registry", not(feature = "proto")))]
impl rumi::IntoDataInput<HttpMessage> for PathInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<HttpMessage>>, rumi::MatcherError> {
        Ok(Box::new(PathInput))
    }
}

#[cfg(all(feature = "registry", not(feature = "proto")))]
impl rumi::IntoDataInput<HttpMessage> for MethodInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<HttpMessage>>, rumi::MatcherError> {
        Ok(Box::new(MethodInput))
    }
}

#[cfg(all(feature = "registry", not(feature = "proto")))]
impl rumi::IntoDataInput<HttpMessage> for HeaderInput {
    type Config = HeaderInputConfig;

    fn from_config(
        config: Self::Config,
    ) -> Result<Box<dyn rumi::DataInput<HttpMessage>>, rumi::MatcherError> {
        Ok(Box::new(HeaderInput::new(config.name)?))
    }
}

#[cfg(all(feature = "registry", not(feature = "proto")))]
impl rumi::IntoDataInput<HttpMessage> for QueryParamInput {
    type Config = QueryParamInputConfig;

    fn from_config(
        config: Self::Config,
    ) -> Result<Box<dyn rumi::DataInput<HttpMessage>>, rumi::MatcherError> {
        Ok(Box::new(QueryParamInput::new(config.name)?))
    }
}

#[cfg(all(feature = "registry", not(feature = "proto")))]
impl rumi::IntoDataInput<HttpMessage> for SchemeInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<HttpMessage>>, rumi::MatcherError> {
        Ok(Box::new(SchemeInput))
    }
}

#[cfg(all(feature = "registry", not(feature = "proto")))]
impl rumi::IntoDataInput<HttpMessage> for AuthorityInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<HttpMessage>>, rumi::MatcherError> {
        Ok(Box::new(AuthorityInput))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Proto config types (feature = "proto")
// Uses proto-generated types as Config, enabling xDS control plane integration.
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "proto")]
mod proto_configs {
    use super::*;
    use rumi_proto::xuma::http::v1 as proto;

    impl rumi::IntoDataInput<HttpMessage> for PathInput {
        type Config = proto::PathInput;

        fn from_config(
            _: proto::PathInput,
        ) -> Result<Box<dyn rumi::DataInput<HttpMessage>>, rumi::MatcherError> {
            Ok(Box::new(PathInput))
        }
    }

    impl rumi::IntoDataInput<HttpMessage> for MethodInput {
        type Config = proto::MethodInput;

        fn from_config(
            _: proto::MethodInput,
        ) -> Result<Box<dyn rumi::DataInput<HttpMessage>>, rumi::MatcherError> {
            Ok(Box::new(MethodInput))
        }
    }

    impl rumi::IntoDataInput<HttpMessage> for HeaderInput {
        type Config = proto::HeaderInput;

        fn from_config(
            config: proto::HeaderInput,
        ) -> Result<Box<dyn rumi::DataInput<HttpMessage>>, rumi::MatcherError> {
            Ok(Box::new(HeaderInput::new(config.name)?))
        }
    }

    impl rumi::IntoDataInput<HttpMessage> for QueryParamInput {
        type Config = proto::QueryParamInput;

        fn from_config(
            config: proto::QueryParamInput,
        ) -> Result<Box<dyn rumi::DataInput<HttpMessage>>, rumi::MatcherError> {
            Ok(Box::new(QueryParamInput::new(config.name)?))
        }
    }

    impl rumi::IntoDataInput<HttpMessage> for SchemeInput {
        type Config = proto::SchemeInput;

        fn from_config(
            _: proto::SchemeInput,
        ) -> Result<Box<dyn rumi::DataInput<HttpMessage>>, rumi::MatcherError> {
            Ok(Box::new(SchemeInput))
        }
    }

    impl rumi::IntoDataInput<HttpMessage> for AuthorityInput {
        type Config = proto::HostInput;

        fn from_config(
            _: proto::HostInput,
        ) -> Result<Box<dyn rumi::DataInput<HttpMessage>>, rumi::MatcherError> {
            Ok(Box::new(AuthorityInput))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::HttpMessageBuilder;

    /// An empty header name reads no header, so every predicate keyed on it is
    /// false and a deny rule silently stops denying. Rejected at construction
    /// so the config path, the compiler, the FFI and direct callers all inherit
    /// it — a check in the loader alone would be advisory to the other three.
    #[test]
    fn an_empty_header_name_is_rejected() {
        let err = HeaderInput::new("").unwrap_err();
        assert!(
            matches!(err, rumi::MatcherError::EmptyIdentifier { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn an_empty_query_param_name_is_rejected() {
        let err = QueryParamInput::new("").unwrap_err();
        assert!(
            matches!(err, rumi::MatcherError::EmptyIdentifier { .. }),
            "{err:?}"
        );
    }

    // These test DataInput behaviour on HttpMessage, which is domain, not
    // transport — so they build via the public builder rather than assembling
    // an ext_proc ProcessingRequest. The ext_proc conversion has its own tests
    // in message.rs, including one asserting both paths agree.
    struct ProcessingRequestBuilder {
        inner: HttpMessageBuilder,
    }

    impl ProcessingRequestBuilder {
        fn new() -> Self {
            Self {
                inner: HttpMessageBuilder::new(),
            }
        }
        fn path(mut self, v: &str) -> Self {
            self.inner = self.inner.path(v);
            self
        }
        fn method(mut self, v: &str) -> Self {
            self.inner = self.inner.method(v);
            self
        }
        fn scheme(mut self, v: &str) -> Self {
            self.inner = self.inner.scheme(v);
            self
        }
        fn authority(mut self, v: &str) -> Self {
            self.inner = self.inner.authority(v);
            self
        }
        fn header(mut self, k: &str, v: &str) -> Self {
            self.inner = self.inner.header(k, v);
            self
        }
        fn build(self) -> HttpMessage {
            self.inner.build()
        }
    }

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
        let input = HeaderInput::new("content-type").unwrap();
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

        let input = HeaderInput::new("X-Custom-Header").unwrap();
        assert_eq!(input.get(&msg), MatchingData::String("value123".into()));
    }

    #[test]
    fn header_input_returns_none_when_missing() {
        let msg = ProcessingRequestBuilder::new()
            .header("content-type", "text/plain")
            .build();
        let input = HeaderInput::new("authorization").unwrap();
        assert_eq!(input.get(&msg), MatchingData::None);
    }

    #[test]
    fn header_input_extracts_authorization() {
        let msg = ProcessingRequestBuilder::new()
            .header("authorization", "Bearer token123")
            .build();
        let input = HeaderInput::new("authorization").unwrap();
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
        let input = QueryParamInput::new("q").unwrap();
        assert_eq!(input.get(&msg), MatchingData::String("rust".into()));
    }

    #[test]
    fn query_param_input_extracts_from_multiple() {
        let msg = ProcessingRequestBuilder::new()
            .path("/api?page=1&limit=10&sort=name")
            .build();

        assert_eq!(
            QueryParamInput::new("page").unwrap().get(&msg),
            MatchingData::String("1".into())
        );
        assert_eq!(
            QueryParamInput::new("limit").unwrap().get(&msg),
            MatchingData::String("10".into())
        );
        assert_eq!(
            QueryParamInput::new("sort").unwrap().get(&msg),
            MatchingData::String("name".into())
        );
    }

    #[test]
    fn query_param_input_returns_none_when_missing() {
        let msg = ProcessingRequestBuilder::new().path("/api?page=1").build();
        let input = QueryParamInput::new("limit").unwrap();
        assert_eq!(input.get(&msg), MatchingData::None);
    }

    #[test]
    fn query_param_input_returns_none_when_no_query_string() {
        let msg = ProcessingRequestBuilder::new().path("/api").build();
        let input = QueryParamInput::new("page").unwrap();
        assert_eq!(input.get(&msg), MatchingData::None);
    }

    #[test]
    fn query_param_input_returns_none_when_no_path() {
        let msg = ProcessingRequestBuilder::new().method("GET").build();
        let input = QueryParamInput::new("page").unwrap();
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
            QueryParamInput::new("page").unwrap().get(&msg),
            MatchingData::String("1".into())
        );
        assert_eq!(
            QueryParamInput::new("limit").unwrap().get(&msg),
            MatchingData::String("20".into())
        );
        assert_eq!(
            HeaderInput::new("content-type").unwrap().get(&msg),
            MatchingData::String("application/json".into())
        );
        assert_eq!(
            HeaderInput::new("authorization").unwrap().get(&msg),
            MatchingData::String("Bearer abc123".into())
        );
    }

    #[test]
    fn an_empty_message_yields_none_from_every_input() {
        // An empty message: no transport needed to assert INV-1.
        let msg = HttpMessageBuilder::new().build();

        assert_eq!(PathInput.get(&msg), MatchingData::None);
        assert_eq!(MethodInput.get(&msg), MatchingData::None);
        assert_eq!(SchemeInput.get(&msg), MatchingData::None);
        assert_eq!(AuthorityInput.get(&msg), MatchingData::None);
        assert_eq!(
            HeaderInput::new("any").unwrap().get(&msg),
            MatchingData::None
        );
        assert_eq!(
            QueryParamInput::new("any").unwrap().get(&msg),
            MatchingData::None
        );
    }
}
