//! Simple `HttpRequest` for testing and basic use cases.
//!
//! This is a lightweight context for when you don't need full `ext_proc`.

use crate::context::{get_query_param, parse_path_only, parse_query_string};
use rumi::prelude::*;
use std::collections::HashMap;

/// Simple HTTP request context for matching.
///
/// Use this for testing or simple use cases. For production `ext_proc`
/// integration, use [`HttpMessage`](crate::HttpMessage) instead.
#[derive(Debug, Clone, Default)]
pub struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    query_params: HashMap<String, String>,
}

impl HttpRequest {
    /// Create a builder for `HttpRequest`.
    #[must_use]
    pub fn builder() -> HttpRequestBuilder {
        HttpRequestBuilder::default()
    }

    /// Get the HTTP method.
    #[must_use]
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Get the request path, without any query string.
    ///
    /// Splits at the first `?`, the same way [`HttpMessage`](crate::HttpMessage)
    /// does, and through the same function — so the two HTTP contexts cannot
    /// disagree about what a path is. They did: this returned the raw value, so
    /// `/admin?x=1` matched `Exact("/admin")` on one context and not the other,
    /// under the same `xuma.http.v1.PathInput` type URL. Both crusts use this
    /// one, so a path gate behaved differently in the wheel and the npm package
    /// than it did natively.
    #[must_use]
    pub fn path(&self) -> &str {
        parse_path_only(&self.path)
    }

    /// Get the path exactly as it was set, query string included.
    #[must_use]
    pub fn raw_path(&self) -> &str {
        &self.path
    }

    /// Get a header value by name (case-insensitive).
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_lowercase()).map(String::as_str)
    }

    /// Get a query parameter by name.
    ///
    /// Parameters set explicitly on the builder win; otherwise the path's own
    /// query string is parsed, so a request built as `path("/a?x=1")` answers
    /// the same question as one built with `query_param("x", "1")`.
    #[must_use]
    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.query_params
            .get(name)
            .map(String::as_str)
            .or_else(|| parse_query_string(&self.path).and_then(|q| get_query_param(q, name)))
    }
}

/// Builder for `HttpRequest`.
#[derive(Debug, Default)]
pub struct HttpRequestBuilder {
    request: HttpRequest,
}

impl HttpRequestBuilder {
    /// Set the HTTP method.
    #[must_use]
    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.request.method = method.into();
        self
    }

    /// Set the request path.
    #[must_use]
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.request.path = path.into();
        self
    }

    /// Add a header (name is lowercased for case-insensitive lookup).
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.request
            .headers
            .insert(name.into().to_lowercase(), value.into());
        self
    }

    /// Add a query parameter.
    #[must_use]
    pub fn query_param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.request.query_params.insert(name.into(), value.into());
        self
    }

    /// Build the `HttpRequest`.
    #[must_use]
    pub fn build(self) -> HttpRequest {
        self.request
    }
}

// DataInputs for simple HttpRequest

/// Extracts the HTTP method from simple `HttpRequest`.
#[derive(Debug, Clone)]
pub struct SimpleMethodInput;

impl DataInput<HttpRequest> for SimpleMethodInput {
    fn get(&self, ctx: &HttpRequest) -> MatchingData {
        MatchingData::String(ctx.method.clone())
    }
}

/// Extracts the path from simple `HttpRequest`.
#[derive(Debug, Clone)]
pub struct SimplePathInput;

impl DataInput<HttpRequest> for SimplePathInput {
    fn get(&self, ctx: &HttpRequest) -> MatchingData {
        MatchingData::String(ctx.path().to_string())
    }
}

/// Extracts the `:authority` pseudo-header from simple `HttpRequest`.
///
/// `HttpMessage` reads authority and scheme from the `:authority` and
/// `:scheme` pseudo-headers, and `HttpRequest` carries headers, so it reads
/// them from the same place. Without these two, `register_simple` bound four of
/// the six `xuma.http.v1.*` type URLs and `register` bound all six — the same
/// one-type-URL-two-behaviours class as F26, in the shape of a type URL that
/// resolves in one context and not the other.
#[derive(Debug, Clone, Default)]
pub struct SimpleAuthorityInput;

impl DataInput<HttpRequest> for SimpleAuthorityInput {
    fn get(&self, ctx: &HttpRequest) -> MatchingData {
        ctx.header(":authority")
            .map_or(MatchingData::None, |a| MatchingData::String(a.to_string()))
    }
}

/// Extracts the `:scheme` pseudo-header from simple `HttpRequest`.
#[derive(Debug, Clone, Default)]
pub struct SimpleSchemeInput;

impl DataInput<HttpRequest> for SimpleSchemeInput {
    fn get(&self, ctx: &HttpRequest) -> MatchingData {
        ctx.header(":scheme")
            .map_or(MatchingData::None, |s| MatchingData::String(s.to_string()))
    }
}

/// Extracts a header from simple `HttpRequest`.
#[derive(Debug, Clone)]
pub struct SimpleHeaderInput {
    name: String,
}

impl SimpleHeaderInput {
    /// Create a header input for the given name (case-insensitive).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into().to_lowercase(),
        }
    }
}

impl DataInput<HttpRequest> for SimpleHeaderInput {
    fn get(&self, ctx: &HttpRequest) -> MatchingData {
        ctx.header(&self.name)
            .map_or(MatchingData::None, |s| MatchingData::String(s.to_string()))
    }
}

/// Extracts a query parameter from simple `HttpRequest`.
#[derive(Debug, Clone)]
pub struct SimpleQueryParamInput {
    name: String,
}

impl SimpleQueryParamInput {
    /// Create a query param input for the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl DataInput<HttpRequest> for SimpleQueryParamInput {
    fn get(&self, ctx: &HttpRequest) -> MatchingData {
        ctx.query_param(&self.name)
            .map_or(MatchingData::None, |s| MatchingData::String(s.to_string()))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Registry support for HttpRequest (feature = "registry")
// Mirrors the HttpMessage registry but for the simpler HttpRequest context.
// ═══════════════════════════════════════════════════════════════════════════════

/// Configuration for [`SimpleHeaderInput`].
#[cfg(feature = "registry")]
#[derive(serde::Deserialize)]
pub struct SimpleHeaderInputConfig {
    /// The header name to extract (case-insensitive).
    pub name: String,
}

/// Configuration for [`SimpleQueryParamInput`].
#[cfg(feature = "registry")]
#[derive(serde::Deserialize)]
pub struct SimpleQueryParamInputConfig {
    /// The query parameter name to extract.
    pub name: String,
}

#[cfg(feature = "registry")]
impl rumi::IntoDataInput<HttpRequest> for SimplePathInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<HttpRequest>>, rumi::MatcherError> {
        Ok(Box::new(SimplePathInput))
    }
}

#[cfg(feature = "registry")]
impl rumi::IntoDataInput<HttpRequest> for SimpleMethodInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<HttpRequest>>, rumi::MatcherError> {
        Ok(Box::new(SimpleMethodInput))
    }
}

#[cfg(feature = "registry")]
impl rumi::IntoDataInput<HttpRequest> for SimpleAuthorityInput {
    type Config = rumi_proto::xuma::http::v1::AuthorityInput;

    fn from_config(
        _: Self::Config,
    ) -> Result<Box<dyn rumi::DataInput<HttpRequest>>, rumi::MatcherError> {
        Ok(Box::new(SimpleAuthorityInput))
    }
}

#[cfg(feature = "registry")]
impl rumi::IntoDataInput<HttpRequest> for SimpleSchemeInput {
    type Config = rumi_proto::xuma::http::v1::SchemeInput;

    fn from_config(
        _: Self::Config,
    ) -> Result<Box<dyn rumi::DataInput<HttpRequest>>, rumi::MatcherError> {
        Ok(Box::new(SimpleSchemeInput))
    }
}

#[cfg(feature = "registry")]
impl rumi::IntoDataInput<HttpRequest> for SimpleHeaderInput {
    type Config = SimpleHeaderInputConfig;

    fn from_config(
        config: Self::Config,
    ) -> Result<Box<dyn rumi::DataInput<HttpRequest>>, rumi::MatcherError> {
        Ok(Box::new(SimpleHeaderInput::new(config.name)))
    }
}

#[cfg(feature = "registry")]
impl rumi::IntoDataInput<HttpRequest> for SimpleQueryParamInput {
    type Config = SimpleQueryParamInputConfig;

    fn from_config(
        config: Self::Config,
    ) -> Result<Box<dyn rumi::DataInput<HttpRequest>>, rumi::MatcherError> {
        Ok(Box::new(SimpleQueryParamInput::new(config.name)))
    }
}

/// Register all rumi-http types for [`HttpRequest`] with the given builder.
///
/// Uses the same type URLs as the full [`HttpMessage`](crate::HttpMessage) registry,
/// but with simpler inputs suitable for testing and Python/WASM bindings.
///
/// Registers:
/// - `xuma.http.v1.PathInput` → [`SimplePathInput`]
/// - `xuma.http.v1.MethodInput` → [`SimpleMethodInput`]
/// - `xuma.http.v1.HeaderInput` → [`SimpleHeaderInput`]
/// - `xuma.http.v1.QueryParamInput` → [`SimpleQueryParamInput`]
#[cfg(feature = "registry")]
#[must_use]
pub fn register_simple(
    builder: rumi::RegistryBuilder<HttpRequest>,
) -> rumi::RegistryBuilder<HttpRequest> {
    rumi::register_core_matchers(builder)
        .input::<SimplePathInput>("xuma.http.v1.PathInput")
        .input::<SimpleMethodInput>("xuma.http.v1.MethodInput")
        .input::<SimpleHeaderInput>("xuma.http.v1.HeaderInput")
        .input::<SimpleQueryParamInput>("xuma.http.v1.QueryParamInput")
        .input::<SimpleAuthorityInput>("xuma.http.v1.AuthorityInput")
        .input::<SimpleSchemeInput>("xuma.http.v1.SchemeInput")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One type URL, one meaning.
    ///
    /// `xuma.http.v1.PathInput` resolves to `PathInput` through `register` and
    /// to `SimplePathInput` through `register_simple`. They disagreed about the
    /// query string, and both crusts use the second — so a path gate that held
    /// natively did not hold through the wheel or the npm package.
    ///
    /// This compares the two directly rather than asserting each against a
    /// literal, so it fails if either drifts, whichever way it drifts.
    #[test]
    #[cfg(feature = "message")]
    fn both_http_contexts_agree_on_what_a_path_is() {
        use crate::inputs::PathInput;
        use crate::message::HttpMessageBuilder;

        for raw in ["/admin", "/admin?x=1", "/admin?", "/a/b/c?q=1&r=2", "/", ""] {
            let simple = HttpRequest::builder().path(raw).build();
            let message = HttpMessageBuilder::new().path(raw).build();

            assert_eq!(
                SimplePathInput.get(&simple),
                PathInput.get(&message),
                "the two HTTP contexts disagree about {raw:?}"
            );
        }
    }

    /// The query string is reachable from a path that carries one, so the two
    /// contexts also agree about parameters, not only about the path.
    #[test]
    fn a_query_string_in_the_path_is_parsed() {
        let req = HttpRequest::builder().path("/api?page=2&limit=10").build();
        assert_eq!(req.query_param("page"), Some("2"));
        assert_eq!(req.query_param("limit"), Some("10"));
        assert_eq!(req.query_param("missing"), None);
        assert_eq!(req.path(), "/api");
        assert_eq!(req.raw_path(), "/api?page=2&limit=10");
    }

    #[test]
    fn test_http_request_builder() {
        let req = HttpRequest::builder()
            .method("POST")
            .path("/api/users")
            .header("Content-Type", "application/json")
            .query_param("page", "1")
            .build();

        assert_eq!(req.method(), "POST");
        assert_eq!(req.path(), "/api/users");
        assert_eq!(req.header("content-type"), Some("application/json"));
        assert_eq!(req.query_param("page"), Some("1"));
    }

    #[test]
    fn test_case_insensitive_headers() {
        let req = HttpRequest::builder()
            .header("X-Custom-Header", "value")
            .build();

        assert_eq!(req.header("x-custom-header"), Some("value"));
        assert_eq!(req.header("X-CUSTOM-HEADER"), Some("value"));
    }
}
