//! rumi-http: HTTP domain for request matching
//!
//! Provides context and `DataInput` implementations for HTTP requests.
//!
//! # Example
//!
//! ```ignore
//! use rumi_http::prelude::*;
//!
//! let request = HttpRequest::builder()
//!     .method("GET")
//!     .path("/api/users")
//!     .header("Authorization", "Bearer token123")
//!     .build();
//!
//! let input = HeaderInput::new("Authorization");
//! assert_eq!(input.get(&request), MatchingData::String("Bearer token123".into()));
//! ```

use rumi::prelude::*;
use std::collections::HashMap;

/// HTTP request context for matching.
#[derive(Debug, Clone, Default)]
pub struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    query_params: HashMap<String, String>,
}

impl HttpRequest {
    /// Create a builder for HttpRequest.
    #[must_use]
    pub fn builder() -> HttpRequestBuilder {
        HttpRequestBuilder::default()
    }

    /// Get the HTTP method.
    #[must_use]
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Get the request path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get a header value by name (case-insensitive).
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_lowercase())
            .map(String::as_str)
    }

    /// Get a query parameter by name.
    #[must_use]
    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.query_params.get(name).map(String::as_str)
    }
}

/// Builder for HttpRequest.
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
        self.request.headers.insert(name.into().to_lowercase(), value.into());
        self
    }

    /// Add a query parameter.
    #[must_use]
    pub fn query_param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.request.query_params.insert(name.into(), value.into());
        self
    }

    /// Build the HttpRequest.
    #[must_use]
    pub fn build(self) -> HttpRequest {
        self.request
    }
}

/// Extracts the HTTP method from the request.
#[derive(Debug, Clone)]
pub struct MethodInput;

impl DataInput<HttpRequest> for MethodInput {
    fn get(&self, ctx: &HttpRequest) -> MatchingData {
        MatchingData::String(ctx.method.clone())
    }
}

/// Extracts the request path.
#[derive(Debug, Clone)]
pub struct PathInput;

impl DataInput<HttpRequest> for PathInput {
    fn get(&self, ctx: &HttpRequest) -> MatchingData {
        MatchingData::String(ctx.path.clone())
    }
}

/// Extracts a header value by name.
#[derive(Debug, Clone)]
pub struct HeaderInput {
    name: String,
}

impl HeaderInput {
    /// Create a new header input extractor.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into().to_lowercase() }
    }
}

impl DataInput<HttpRequest> for HeaderInput {
    fn get(&self, ctx: &HttpRequest) -> MatchingData {
        ctx.header(&self.name)
            .map(|s| MatchingData::String(s.to_string()))
            .unwrap_or(MatchingData::None)
    }
}

/// Extracts a query parameter value by name.
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

impl DataInput<HttpRequest> for QueryParamInput {
    fn get(&self, ctx: &HttpRequest) -> MatchingData {
        ctx.query_param(&self.name)
            .map(|s| MatchingData::String(s.to_string()))
            .unwrap_or(MatchingData::None)
    }
}

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::{HeaderInput, HttpRequest, HttpRequestBuilder, MethodInput, PathInput, QueryParamInput};
    pub use rumi::prelude::*;
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_method_input() {
        let req = HttpRequest::builder().method("GET").build();
        assert_eq!(MethodInput.get(&req), MatchingData::String("GET".into()));
    }

    #[test]
    fn test_path_input() {
        let req = HttpRequest::builder().path("/foo/bar").build();
        assert_eq!(PathInput.get(&req), MatchingData::String("/foo/bar".into()));
    }

    #[test]
    fn test_header_input() {
        let req = HttpRequest::builder()
            .header("X-Custom", "value")
            .build();

        let input = HeaderInput::new("x-custom"); // case-insensitive
        assert_eq!(input.get(&req), MatchingData::String("value".into()));
    }

    #[test]
    fn test_header_input_missing() {
        let req = HttpRequest::builder().build();
        let input = HeaderInput::new("missing");
        assert_eq!(input.get(&req), MatchingData::None);
    }

    #[test]
    fn test_full_http_matcher() {
        let req = HttpRequest::builder()
            .method("GET")
            .path("/api/admin")
            .header("Authorization", "Bearer admin-token")
            .build();

        let matcher: Matcher<HttpRequest, &str> = Matcher::new(
            vec![
                FieldMatcher::new(
                    Predicate::And(vec![
                        Predicate::Single(SinglePredicate::new(
                            Box::new(PathInput),
                            Box::new(PrefixMatcher::new("/api/admin")),
                        )),
                        Predicate::Single(SinglePredicate::new(
                            Box::new(HeaderInput::new("Authorization")),
                            Box::new(PrefixMatcher::new("Bearer ")),
                        )),
                    ]),
                    OnMatch::Action("admin_backend"),
                ),
            ],
            Some(OnMatch::Action("public_backend")),
        );

        assert_eq!(matcher.evaluate(&req), Some("admin_backend"));
    }
}
