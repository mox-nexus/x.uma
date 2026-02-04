//! `ProcessingRequest` context helpers.
//!
//! Provides ergonomic access to HTTP data within `ext_proc` `ProcessingRequest`.

use envoy_grpc_ext_proc::envoy::service::ext_proc::v3::{
    processing_request::Request, HttpHeaders, ProcessingRequest,
};

/// Extension trait for `ProcessingRequest` to extract HTTP data.
pub trait ProcessingRequestExt {
    /// Get the request headers if this is a `request_headers` message.
    fn request_headers(&self) -> Option<&HttpHeaders>;

    /// Get the response headers if this is a `response_headers` message.
    fn response_headers(&self) -> Option<&HttpHeaders>;

    /// Get a header value by name from request headers.
    /// Header names are matched case-insensitively (all stored lowercase).
    fn get_request_header(&self, name: &str) -> Option<&str>;

    /// Get the :path pseudo-header from request headers.
    fn get_path(&self) -> Option<&str>;

    /// Get the :method pseudo-header from request headers.
    fn get_method(&self) -> Option<&str>;

    /// Get the :authority pseudo-header from request headers.
    fn get_authority(&self) -> Option<&str>;

    /// Get the :scheme pseudo-header from request headers.
    fn get_scheme(&self) -> Option<&str>;
}

impl ProcessingRequestExt for ProcessingRequest {
    fn request_headers(&self) -> Option<&HttpHeaders> {
        match &self.request {
            Some(Request::RequestHeaders(h)) => Some(h),
            _ => None,
        }
    }

    fn response_headers(&self) -> Option<&HttpHeaders> {
        match &self.request {
            Some(Request::ResponseHeaders(h)) => Some(h),
            _ => None,
        }
    }

    fn get_request_header(&self, name: &str) -> Option<&str> {
        let headers = self.request_headers()?;
        let header_map = headers.headers.as_ref()?;
        let name_lower = name.to_lowercase();

        header_map
            .headers
            .iter()
            .find(|h| h.key.to_lowercase() == name_lower)
            .and_then(|h| {
                // Prefer raw_value, fall back to value
                if h.raw_value.is_empty() {
                    Some(h.value.as_str())
                } else {
                    std::str::from_utf8(&h.raw_value).ok()
                }
            })
    }

    fn get_path(&self) -> Option<&str> {
        self.get_request_header(":path")
    }

    fn get_method(&self) -> Option<&str> {
        self.get_request_header(":method")
    }

    fn get_authority(&self) -> Option<&str> {
        self.get_request_header(":authority")
    }

    fn get_scheme(&self) -> Option<&str> {
        self.get_request_header(":scheme")
    }
}

/// Parse query string from path.
///
/// Returns the query string portion after '?' or None if no query string.
#[must_use]
pub fn parse_query_string(path: &str) -> Option<&str> {
    path.split_once('?').map(|(_, query)| query)
}

/// Parse path without query string.
///
/// Returns the path portion before '?' or the full path if no query string.
#[must_use]
pub fn parse_path_only(path: &str) -> &str {
    path.split_once('?').map_or(path, |(p, _)| p)
}

/// Get a query parameter value from a query string.
#[must_use]
pub fn get_query_param<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        if key == name {
            Some(value)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_string() {
        assert_eq!(
            parse_query_string("/api/users?page=1&limit=10"),
            Some("page=1&limit=10")
        );
        assert_eq!(parse_query_string("/api/users"), None);
        assert_eq!(parse_query_string("/api?"), Some(""));
    }

    #[test]
    fn test_parse_path_only() {
        assert_eq!(parse_path_only("/api/users?page=1"), "/api/users");
        assert_eq!(parse_path_only("/api/users"), "/api/users");
    }

    #[test]
    fn test_get_query_param() {
        let query = "page=1&limit=10&sort=name";
        assert_eq!(get_query_param(query, "page"), Some("1"));
        assert_eq!(get_query_param(query, "limit"), Some("10"));
        assert_eq!(get_query_param(query, "sort"), Some("name"));
        assert_eq!(get_query_param(query, "missing"), None);
    }
}
