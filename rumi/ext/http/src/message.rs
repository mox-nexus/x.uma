//! `HttpMessage` — Indexed view over HTTP request data.
//!
//! Provides O(1) header/path/method lookups by pre-indexing data from
//! `ext_proc` `ProcessingRequest` at construction time.
//!
//! This is the recommended context type for matcher evaluation. The raw
//! `ProcessingRequest` stores headers as a flat list requiring O(H) scans;
//! `HttpMessage` builds a `HashMap` index once, enabling O(1) lookups
//! for the lifetime of the evaluation.

#[cfg(feature = "ext-proc")]
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

/// Builds an [`HttpMessage`] from plain header pairs.
///
/// `HttpMessage` was previously constructible only via `From<ProcessingRequest>`,
/// which made the Gateway API compiler — whose output matches on `HttpMessage` —
/// unusable from anything that was not an `ext_proc` filter. The type itself is
/// six plain fields and has nothing to do with `ext_proc`; only the conversion did.
#[derive(Debug, Default, Clone)]
pub struct HttpMessageBuilder {
    headers: Vec<(String, String)>,
}

impl HttpMessageBuilder {
    /// Start an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a header. Pseudo-headers (`:path`, `:method`, …) are recognised.
    #[must_use]
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    /// Set `:path`, including any query string.
    #[must_use]
    pub fn path(self, path: impl Into<String>) -> Self {
        self.header(":path", path)
    }

    /// Set `:method`.
    #[must_use]
    pub fn method(self, method: impl Into<String>) -> Self {
        self.header(":method", method)
    }

    /// Set `:authority`.
    #[must_use]
    pub fn authority(self, authority: impl Into<String>) -> Self {
        self.header(":authority", authority)
    }

    /// Set `:scheme`.
    #[must_use]
    pub fn scheme(self, scheme: impl Into<String>) -> Self {
        self.header(":scheme", scheme)
    }

    /// Build the indexed message.
    #[must_use]
    pub fn build(self) -> HttpMessage {
        HttpMessage::from_header_pairs(self.headers)
    }
}

impl HttpMessage {
    /// Index a list of header pairs into a message.
    ///
    /// Pseudo-headers are lifted into their own fields. The `ext_proc` conversion
    /// delegates here rather than repeating the lift, so the two construction
    /// paths cannot drift — `builder_and_ext_proc_agree` in the tests holds it.
    #[must_use]
    pub fn from_header_pairs(pairs: impl IntoIterator<Item = (String, String)>) -> Self {
        let mut headers = HashMap::new();
        let mut path = None;
        let mut raw_path = None;
        let mut method = None;
        let mut authority = None;
        let mut scheme = None;

        for (key, value) in pairs {
            let key = key.to_ascii_lowercase();
            match key.as_str() {
                ":path" => {
                    raw_path = Some(value.clone());
                    path = Some(parse_path_only(&value).to_string());
                }
                ":method" => method = Some(value.clone()),
                ":authority" => authority = Some(value.clone()),
                ":scheme" => scheme = Some(value.clone()),
                _ => {}
            }
            headers.insert(key, value);
        }

        Self {
            headers,
            path,
            raw_path,
            method,
            authority,
            scheme,
        }
    }

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
#[cfg(feature = "ext-proc")]
impl From<ProcessingRequest> for HttpMessage {
    fn from(req: ProcessingRequest) -> Self {
        from_request_headers(req.request.as_ref())
    }
}

/// Borrowing conversion — clones strings from the proto.
#[cfg(feature = "ext-proc")]
impl From<&ProcessingRequest> for HttpMessage {
    fn from(req: &ProcessingRequest) -> Self {
        from_request_headers(req.request.as_ref())
    }
}

/// Decode ext_proc headers, then delegate the indexing.
///
/// This *calls* [`HttpMessage::from_header_pairs`] rather than reproducing it.
/// It used to hand-copy the pseudo-header lift, and a comment claimed the two
/// paths could not disagree — which was untrue, and worse than no comment,
/// because it told the next reader not to check. The only thing that belongs
/// here is the transport-shaped detail: `raw_value` takes precedence over
/// `value`, and is never stored.
#[cfg(feature = "ext-proc")]
fn from_request_headers(request: Option<&Request>) -> HttpMessage {
    let http_headers = match request {
        Some(Request::RequestHeaders(h) | Request::ResponseHeaders(h)) => h.headers.as_ref(),
        _ => None,
    };

    let Some(header_map) = http_headers else {
        return HttpMessage::from_header_pairs(Vec::new());
    };

    HttpMessage::from_header_pairs(header_map.headers.iter().map(|hv| {
        let value = if hv.raw_value.is_empty() {
            hv.value.clone()
        } else {
            String::from_utf8_lossy(&hv.raw_value).into_owned()
        };
        (hv.key.clone(), value)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "ext-proc")]
    use envoy_grpc_ext_proc::envoy::{
        config::core::v3::{HeaderMap, HeaderValue},
        service::ext_proc::v3::HttpHeaders,
    };

    #[cfg(feature = "ext-proc")]
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

    #[cfg(feature = "ext-proc")]
    #[test]
    fn test_path_extraction() {
        let req = build_request(vec![(":path", "/api/users?page=1")]);
        let msg = HttpMessage::from(&req);
        assert_eq!(msg.path(), Some("/api/users"));
        assert_eq!(msg.raw_path(), Some("/api/users?page=1"));
    }

    #[cfg(feature = "ext-proc")]
    #[test]
    fn test_method_extraction() {
        let req = build_request(vec![(":method", "POST")]);
        let msg = HttpMessage::from(&req);
        assert_eq!(msg.method(), Some("POST"));
    }

    #[cfg(feature = "ext-proc")]
    #[test]
    fn test_header_case_insensitive() {
        let req = build_request(vec![("Content-Type", "application/json")]);
        let msg = HttpMessage::from(&req);
        assert_eq!(msg.header("content-type"), Some("application/json"));
        assert_eq!(msg.header("CONTENT-TYPE"), Some("application/json"));
    }

    #[cfg(feature = "ext-proc")]
    #[test]
    fn test_query_param() {
        let req = build_request(vec![(":path", "/search?q=rust&limit=10")]);
        let msg = HttpMessage::from(&req);
        assert_eq!(msg.query_param("q"), Some("rust"));
        assert_eq!(msg.query_param("limit"), Some("10"));
        assert_eq!(msg.query_param("missing"), None);
    }

    #[cfg(feature = "ext-proc")]
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

    #[cfg(feature = "ext-proc")]
    #[test]
    fn test_empty_request() {
        let req = ProcessingRequest::default();
        let msg = HttpMessage::from(&req);
        assert_eq!(msg.path(), None);
        assert_eq!(msg.method(), None);
        assert_eq!(msg.header("any"), None);
    }

    #[cfg(feature = "ext-proc")]
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

    #[cfg(feature = "ext-proc")]
    #[test]
    fn test_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<HttpMessage>();
    }

    // ── The builder path, which had no coverage at all ──────────────────────

    #[test]
    fn builder_lifts_pseudo_headers() {
        let msg = HttpMessageBuilder::new()
            .method("GET")
            .path("/api/users?page=1")
            .authority("example.com")
            .scheme("https")
            .header("X-Trace", "abc")
            .build();

        assert_eq!(msg.method(), Some("GET"));
        assert_eq!(
            msg.path(),
            Some("/api/users"),
            "path drops the query string"
        );
        assert_eq!(
            msg.raw_path(),
            Some("/api/users?page=1"),
            "raw_path keeps it"
        );
        assert_eq!(msg.authority(), Some("example.com"));
        assert_eq!(msg.scheme(), Some("https"));
        assert_eq!(
            msg.header("x-trace"),
            Some("abc"),
            "header lookup is case-insensitive"
        );
    }

    #[test]
    fn builder_produces_none_for_absent_fields() {
        let msg = HttpMessageBuilder::new().build();
        assert_eq!(msg.path(), None);
        assert_eq!(msg.method(), None);
        assert_eq!(msg.header("anything"), None);
    }

    // ── The invariant the doc comment claims ────────────────────────────────
    //
    // `from_request_headers` delegates to `from_header_pairs` rather than
    // repeating the pseudo-header lift. Before 2026-08-17 it duplicated it, and
    // a comment asserted the two could not disagree — which was untrue, and
    // worse than no comment, because it told the reader not to check.
    //
    // This is that check.

    #[cfg(feature = "ext-proc")]
    #[test]
    fn builder_and_ext_proc_agree() {
        let headers = vec![
            (":method", "POST"),
            (":path", "/api/items?q=1&sort=asc"),
            (":authority", "svc.internal"),
            (":scheme", "http"),
            ("Content-Type", "application/json"),
            ("X-Mixed-Case", "Value"),
        ];

        let from_proto = HttpMessage::from(&build_request(headers.clone()));
        let from_builder = headers
            .iter()
            .fold(HttpMessageBuilder::new(), |b, (k, v)| b.header(*k, *v))
            .build();

        assert_eq!(from_proto.path(), from_builder.path());
        assert_eq!(from_proto.raw_path(), from_builder.raw_path());
        assert_eq!(from_proto.method(), from_builder.method());
        assert_eq!(from_proto.authority(), from_builder.authority());
        assert_eq!(from_proto.scheme(), from_builder.scheme());
        for (k, _) in &headers {
            assert_eq!(
                from_proto.header(&k.to_ascii_lowercase()),
                from_builder.header(&k.to_ascii_lowercase()),
                "header {k} differs between construction paths"
            );
        }
    }
}
