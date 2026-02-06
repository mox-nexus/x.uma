//! `HttpMessage` — Indexed view over HTTP request data.
//!
//! Provides O(1) header/path/method lookups by pre-indexing data from
//! `ext_proc` `ProcessingRequest` at construction time.
//!
//! This is the recommended context type for matcher evaluation. The raw
//! `ProcessingRequest` stores headers as a flat list requiring O(H) scans;
//! `HttpMessage` builds a `HashMap` index once, enabling O(1) lookups
//! for the lifetime of the evaluation.

use envoy_grpc_ext_proc::envoy::service::ext_proc::v3::{
    processing_request::Request, ProcessingRequest,
};
use std::collections::HashMap;

use crate::context::{get_query_param, parse_path_only, parse_query_string};

/// Indexed view over HTTP request data for efficient matching.
///
/// Built from a `ProcessingRequest`, pre-indexing all headers into a `HashMap`
/// with lowercased keys and parsing pseudo-headers (`:path`, `:method`, etc.)
/// into dedicated fields.
///
/// # Performance
///
/// - Construction: O(H) where H = number of headers (one-time cost)
/// - All lookups: O(1) via `HashMap`
///
/// # Example
///
/// ```ignore
/// let msg = HttpMessage::from(processing_request);
/// assert_eq!(msg.method(), Some("GET"));
/// assert_eq!(msg.path(), Some("/api/users"));
/// assert_eq!(msg.header("content-type"), Some("application/json"));
/// ```
#[derive(Debug, Clone)]
pub struct HttpMessage {
    /// All headers indexed by lowercased name.
    headers: HashMap<String, String>,
    /// Parsed path (without query string), from `:path` pseudo-header.
    path: Option<String>,
    /// Full raw path (with query string), from `:path` pseudo-header.
    raw_path: Option<String>,
    /// HTTP method, from `:method` pseudo-header.
    method: Option<String>,
    /// Authority, from `:authority` pseudo-header.
    authority: Option<String>,
    /// Scheme, from `:scheme` pseudo-header.
    scheme: Option<String>,
}

impl HttpMessage {
    /// Get the request path (without query string).
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Get the full raw path (with query string).
    #[must_use]
    pub fn raw_path(&self) -> Option<&str> {
        self.raw_path.as_deref()
    }

    /// Get the HTTP method.
    #[must_use]
    pub fn method(&self) -> Option<&str> {
        self.method.as_deref()
    }

    /// Get the authority (host).
    #[must_use]
    pub fn authority(&self) -> Option<&str> {
        self.authority.as_deref()
    }

    /// Get the scheme.
    #[must_use]
    pub fn scheme(&self) -> Option<&str> {
        self.scheme.as_deref()
    }

    /// Get a header value by name. Names are case-insensitive.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    /// Get a query parameter value by name.
    #[must_use]
    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.raw_path
            .as_deref()
            .and_then(parse_query_string)
            .and_then(|q| get_query_param(q, name))
    }
}

/// Consuming conversion — takes ownership of the proto (clones strings internally).
impl From<ProcessingRequest> for HttpMessage {
    fn from(req: ProcessingRequest) -> Self {
        from_request_headers(req.request.as_ref())
    }
}

/// Borrowing conversion — clones strings from the proto.
impl From<&ProcessingRequest> for HttpMessage {
    fn from(req: &ProcessingRequest) -> Self {
        from_request_headers(req.request.as_ref())
    }
}

/// Shared construction logic.
fn from_request_headers(request: Option<&Request>) -> HttpMessage {
    let mut headers = HashMap::new();
    let mut path = None;
    let mut raw_path = None;
    let mut method = None;
    let mut authority = None;
    let mut scheme = None;

    let http_headers = match request {
        Some(Request::RequestHeaders(h) | Request::ResponseHeaders(h)) => h.headers.as_ref(),
        _ => None,
    };

    if let Some(header_map) = http_headers {
        headers.reserve(header_map.headers.len());

        for hv in &header_map.headers {
            let key = hv.key.to_ascii_lowercase();
            let value = if hv.raw_value.is_empty() {
                hv.value.clone()
            } else {
                String::from_utf8_lossy(&hv.raw_value).into_owned()
            };

            // Extract pseudo-headers into dedicated fields
            match key.as_str() {
                ":path" => {
                    path = Some(parse_path_only(&value).to_string());
                    raw_path = Some(value.clone());
                }
                ":method" => method = Some(value.clone()),
                ":authority" => authority = Some(value.clone()),
                ":scheme" => scheme = Some(value.clone()),
                _ => {}
            }

            headers.insert(key, value);
        }
    }

    HttpMessage {
        headers,
        path,
        raw_path,
        method,
        authority,
        scheme,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use envoy_grpc_ext_proc::envoy::{
        config::core::v3::{HeaderMap, HeaderValue},
        service::ext_proc::v3::HttpHeaders,
    };

    fn build_request(headers: Vec<(&str, &str)>) -> ProcessingRequest {
        ProcessingRequest {
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
        }
    }

    #[test]
    fn test_path_extraction() {
        let req = build_request(vec![(":path", "/api/users?page=1")]);
        let msg = HttpMessage::from(&req);
        assert_eq!(msg.path(), Some("/api/users"));
        assert_eq!(msg.raw_path(), Some("/api/users?page=1"));
    }

    #[test]
    fn test_method_extraction() {
        let req = build_request(vec![(":method", "POST")]);
        let msg = HttpMessage::from(&req);
        assert_eq!(msg.method(), Some("POST"));
    }

    #[test]
    fn test_header_case_insensitive() {
        let req = build_request(vec![("Content-Type", "application/json")]);
        let msg = HttpMessage::from(&req);
        assert_eq!(msg.header("content-type"), Some("application/json"));
        assert_eq!(msg.header("CONTENT-TYPE"), Some("application/json"));
    }

    #[test]
    fn test_query_param() {
        let req = build_request(vec![(":path", "/search?q=rust&limit=10")]);
        let msg = HttpMessage::from(&req);
        assert_eq!(msg.query_param("q"), Some("rust"));
        assert_eq!(msg.query_param("limit"), Some("10"));
        assert_eq!(msg.query_param("missing"), None);
    }

    #[test]
    fn test_authority_and_scheme() {
        let req = build_request(vec![
            (":authority", "example.com:8080"),
            (":scheme", "https"),
        ]);
        let msg = HttpMessage::from(&req);
        assert_eq!(msg.authority(), Some("example.com:8080"));
        assert_eq!(msg.scheme(), Some("https"));
    }

    #[test]
    fn test_empty_request() {
        let req = ProcessingRequest::default();
        let msg = HttpMessage::from(&req);
        assert_eq!(msg.path(), None);
        assert_eq!(msg.method(), None);
        assert_eq!(msg.header("any"), None);
    }

    #[test]
    fn test_full_request() {
        let req = build_request(vec![
            (":method", "PUT"),
            (":path", "/api/v2/resource?dry-run=true"),
            (":scheme", "https"),
            (":authority", "api.example.com"),
            ("content-type", "application/json"),
            ("authorization", "Bearer token123"),
        ]);
        let msg = HttpMessage::from(&req);

        assert_eq!(msg.method(), Some("PUT"));
        assert_eq!(msg.path(), Some("/api/v2/resource"));
        assert_eq!(msg.raw_path(), Some("/api/v2/resource?dry-run=true"));
        assert_eq!(msg.scheme(), Some("https"));
        assert_eq!(msg.authority(), Some("api.example.com"));
        assert_eq!(msg.header("content-type"), Some("application/json"));
        assert_eq!(msg.header("authorization"), Some("Bearer token123"));
        assert_eq!(msg.query_param("dry-run"), Some("true"));
    }

    #[test]
    fn test_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<HttpMessage>();
    }
}
