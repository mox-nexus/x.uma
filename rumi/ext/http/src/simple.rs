//! Simple `HttpRequest` for testing and basic use cases.
//!
//! This is a lightweight context for when you don't need full `ext_proc`.

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

    /// Get the request path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get a header value by name (case-insensitive).
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_lowercase()).map(String::as_str)
    }

    /// Get a query parameter by name.
    #[must_use]
    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.query_params.get(name).map(String::as_str)
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
        MatchingData::String(ctx.path.clone())
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
    fn test_case_insensitive_headers() {
        let req = HttpRequest::builder()
            .header("X-Custom-Header", "value")
            .build();

        assert_eq!(req.header("x-custom-header"), Some("value"));
        assert_eq!(req.header("X-CUSTOM-HEADER"), Some("value"));
    }
}
