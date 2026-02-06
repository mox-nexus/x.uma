//! Path and query string parsing utilities.
//!
//! Used internally by [`HttpMessage`](crate::HttpMessage) to parse
//! the `:path` pseudo-header at construction time.

/// Parse query string from path.
///
/// Returns the query string portion after '?' or None if no query string.
#[must_use]
pub(crate) fn parse_query_string(path: &str) -> Option<&str> {
    path.split_once('?').map(|(_, query)| query)
}

/// Parse path without query string.
///
/// Returns the path portion before '?' or the full path if no query string.
#[must_use]
pub(crate) fn parse_path_only(path: &str) -> &str {
    path.split_once('?').map_or(path, |(p, _)| p)
}

/// Get a query parameter value from a query string.
#[must_use]
pub(crate) fn get_query_param<'a>(query: &'a str, name: &str) -> Option<&'a str> {
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
