//! Compiler: Gateway API `HttpRouteMatch` -> rumi Matcher
//!
//! Translates user-friendly Gateway API configuration into efficient
//! runtime matchers operating on `HttpMessage`.

use crate::inputs::{HeaderInput, MethodInput, PathInput, QueryParamInput};
use crate::message::HttpMessage;
use k8s_gateway_api::{HttpHeaderMatch, HttpPathMatch, HttpQueryParamMatch, HttpRouteMatch};
use rumi::prelude::*;

/// Extension trait for compiling `HttpRouteMatch` to rumi Matcher.
pub trait HttpRouteMatchExt {
    /// Compile this `HttpRouteMatch` into a rumi Matcher.
    ///
    /// The resulting matcher operates on `HttpMessage` and returns
    /// the provided action when all conditions match.
    fn compile<A: Clone + Send + Sync + 'static>(&self, action: A) -> Matcher<HttpMessage, A>;

    /// Compile this `HttpRouteMatch` into a Predicate (without action).
    fn to_predicate(&self) -> Predicate<HttpMessage>;
}

impl HttpRouteMatchExt for HttpRouteMatch {
    fn compile<A: Clone + Send + Sync + 'static>(&self, action: A) -> Matcher<HttpMessage, A> {
        let predicate = self.to_predicate();

        Matcher::new(
            vec![FieldMatcher::new(predicate, OnMatch::Action(action))],
            None,
        )
    }

    fn to_predicate(&self) -> Predicate<HttpMessage> {
        let mut predicates: Vec<Predicate<HttpMessage>> = Vec::new();

        // Path matching
        if let Some(path_match) = &self.path {
            predicates.push(compile_path_match(path_match));
        }

        // Method matching
        if let Some(method) = &self.method {
            predicates.push(Predicate::Single(SinglePredicate::new(
                Box::new(MethodInput),
                Box::new(ExactMatcher::new(method.as_str())),
            )));
        }

        // Header matching (all headers are ANDed)
        if let Some(headers) = &self.headers {
            for header_match in headers {
                predicates.push(compile_header_match(header_match));
            }
        }

        // Query param matching (all params are ANDed)
        if let Some(query_params) = &self.query_params {
            for query_match in query_params {
                predicates.push(compile_query_param_match(query_match));
            }
        }

        // If no conditions, match everything
        if predicates.is_empty() {
            return Predicate::Single(SinglePredicate::new(
                Box::new(PathInput),
                Box::new(PrefixMatcher::new("")), // Match any path
            ));
        }

        // Single predicate or AND them together
        if predicates.len() == 1 {
            predicates.pop().unwrap()
        } else {
            Predicate::And(predicates)
        }
    }
}

/// Compile a path match to a predicate.
fn compile_path_match(path_match: &HttpPathMatch) -> Predicate<HttpMessage> {
    let input = Box::new(PathInput);

    let matcher: Box<dyn InputMatcher> = match path_match {
        HttpPathMatch::Exact { value } => Box::new(ExactMatcher::new(value.as_str())),
        HttpPathMatch::PathPrefix { value } => Box::new(PrefixMatcher::new(value.as_str())),
        HttpPathMatch::RegularExpression { value } => {
            Box::new(StringMatcher::regex(value).unwrap_or_else(|_| {
                // Invalid regex falls back to exact match (fail-safe)
                StringMatcher::exact(value, false)
            }))
        }
    };

    Predicate::Single(SinglePredicate::new(input, matcher))
}

/// Compile a header match to a predicate.
fn compile_header_match(header_match: &HttpHeaderMatch) -> Predicate<HttpMessage> {
    match header_match {
        HttpHeaderMatch::Exact { name, value } => {
            let input = Box::new(HeaderInput::new(name.as_str()));
            let matcher = Box::new(ExactMatcher::new(value.as_str()));
            Predicate::Single(SinglePredicate::new(input, matcher))
        }
        HttpHeaderMatch::RegularExpression { name, value } => {
            let input = Box::new(HeaderInput::new(name.as_str()));
            let matcher: Box<dyn InputMatcher> = Box::new(
                StringMatcher::regex(value).unwrap_or_else(|_| StringMatcher::exact(value, false)),
            );
            Predicate::Single(SinglePredicate::new(input, matcher))
        }
    }
}

/// Compile a query param match to a predicate.
fn compile_query_param_match(query_match: &HttpQueryParamMatch) -> Predicate<HttpMessage> {
    match query_match {
        HttpQueryParamMatch::Exact { name, value } => {
            let input = Box::new(QueryParamInput::new(name.as_str()));
            let matcher = Box::new(ExactMatcher::new(value.as_str()));
            Predicate::Single(SinglePredicate::new(input, matcher))
        }
        HttpQueryParamMatch::RegularExpression { name, value } => {
            let input = Box::new(QueryParamInput::new(name.as_str()));
            let matcher: Box<dyn InputMatcher> = Box::new(
                StringMatcher::regex(value).unwrap_or_else(|_| StringMatcher::exact(value, false)),
            );
            Predicate::Single(SinglePredicate::new(input, matcher))
        }
    }
}

/// Compile multiple `HttpRouteMatch` entries into a single Matcher.
///
/// Multiple matches are `ORed` together per Gateway API semantics.
pub fn compile_route_matches<A: Clone + Send + Sync + 'static>(
    matches: &[HttpRouteMatch],
    action: A,
    on_no_match: Option<A>,
) -> Matcher<HttpMessage, A> {
    if matches.is_empty() {
        // Empty matches = match everything
        return Matcher::new(
            vec![FieldMatcher::new(
                Predicate::Single(SinglePredicate::new(
                    Box::new(PathInput),
                    Box::new(PrefixMatcher::new("")),
                )),
                OnMatch::Action(action),
            )],
            on_no_match.map(OnMatch::Action),
        );
    }

    if matches.len() == 1 {
        let predicate = matches[0].to_predicate();
        return Matcher::new(
            vec![FieldMatcher::new(predicate, OnMatch::Action(action))],
            on_no_match.map(OnMatch::Action),
        );
    }

    // Multiple matches: OR them together
    let predicates: Vec<Predicate<HttpMessage>> = matches
        .iter()
        .map(HttpRouteMatchExt::to_predicate)
        .collect();

    Matcher::new(
        vec![FieldMatcher::new(
            Predicate::Or(predicates),
            OnMatch::Action(action),
        )],
        on_no_match.map(OnMatch::Action),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use envoy_grpc_ext_proc::envoy::{
        config::core::v3::{HeaderMap, HeaderValue},
        service::ext_proc::v3::{processing_request::Request, HttpHeaders, ProcessingRequest},
    };

    // ========== Test Helpers ==========

    /// Builder for constructing test requests as `HttpMessage`.
    struct RequestBuilder {
        headers: Vec<HeaderValue>,
    }

    impl RequestBuilder {
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

    // ========== Predicate Structure Tests ==========

    #[test]
    fn test_compile_empty_match() {
        let route_match = HttpRouteMatch::default();
        let predicate = route_match.to_predicate();
        assert!(matches!(predicate, Predicate::Single(_)));
    }

    #[test]
    fn test_compile_path_prefix() {
        let route_match = HttpRouteMatch {
            path: Some(HttpPathMatch::PathPrefix {
                value: "/api".into(),
            }),
            ..Default::default()
        };

        let predicate = route_match.to_predicate();
        assert!(matches!(predicate, Predicate::Single(_)));
    }

    #[test]
    fn test_compile_multiple_conditions() {
        let route_match = HttpRouteMatch {
            path: Some(HttpPathMatch::PathPrefix {
                value: "/api".into(),
            }),
            method: Some("GET".into()),
            ..Default::default()
        };

        let predicate = route_match.to_predicate();
        assert!(matches!(predicate, Predicate::And(_)));
    }

    // ========== End-to-End Path Matching ==========

    #[test]
    fn e2e_path_prefix_matches() {
        let route_match = HttpRouteMatch {
            path: Some(HttpPathMatch::PathPrefix {
                value: "/api".into(),
            }),
            ..Default::default()
        };

        let matcher = route_match.compile("api_backend");

        let msg = RequestBuilder::new().path("/api/users").build();
        assert_eq!(matcher.evaluate(&msg), Some("api_backend"));

        let msg = RequestBuilder::new().path("/api").build();
        assert_eq!(matcher.evaluate(&msg), Some("api_backend"));

        let msg = RequestBuilder::new().path("/other").build();
        assert_eq!(matcher.evaluate(&msg), None);

        let msg = RequestBuilder::new().path("/apifoo").build();
        assert_eq!(matcher.evaluate(&msg), Some("api_backend"));
    }

    #[test]
    fn e2e_path_exact_matches() {
        let route_match = HttpRouteMatch {
            path: Some(HttpPathMatch::Exact {
                value: "/api/v1/health".into(),
            }),
            ..Default::default()
        };

        let matcher = route_match.compile("health_check");

        let msg = RequestBuilder::new().path("/api/v1/health").build();
        assert_eq!(matcher.evaluate(&msg), Some("health_check"));

        let msg = RequestBuilder::new().path("/api/v1/health/").build();
        assert_eq!(matcher.evaluate(&msg), None);

        let msg = RequestBuilder::new().path("/api/v1").build();
        assert_eq!(matcher.evaluate(&msg), None);
    }

    #[test]
    fn e2e_path_regex_matches() {
        let route_match = HttpRouteMatch {
            path: Some(HttpPathMatch::RegularExpression {
                value: r"^/users/\d+$".into(),
            }),
            ..Default::default()
        };

        let matcher = route_match.compile("user_detail");

        let msg = RequestBuilder::new().path("/users/123").build();
        assert_eq!(matcher.evaluate(&msg), Some("user_detail"));

        let msg = RequestBuilder::new().path("/users/1").build();
        assert_eq!(matcher.evaluate(&msg), Some("user_detail"));

        let msg = RequestBuilder::new().path("/users/abc").build();
        assert_eq!(matcher.evaluate(&msg), None);

        let msg = RequestBuilder::new().path("/users/123/edit").build();
        assert_eq!(matcher.evaluate(&msg), None);
    }

    // ========== End-to-End Method Matching ==========

    #[test]
    fn e2e_method_matches() {
        let route_match = HttpRouteMatch {
            method: Some("POST".into()),
            ..Default::default()
        };

        let matcher = route_match.compile("write_endpoint");

        let msg = RequestBuilder::new().method("POST").path("/").build();
        assert_eq!(matcher.evaluate(&msg), Some("write_endpoint"));

        let msg = RequestBuilder::new().method("GET").path("/").build();
        assert_eq!(matcher.evaluate(&msg), None);

        let msg = RequestBuilder::new().method("PUT").path("/").build();
        assert_eq!(matcher.evaluate(&msg), None);
    }

    // ========== End-to-End Header Matching ==========

    #[test]
    fn e2e_header_exact_matches() {
        let route_match = HttpRouteMatch {
            headers: Some(vec![HttpHeaderMatch::Exact {
                name: "x-api-version".into(),
                value: "v2".into(),
            }]),
            ..Default::default()
        };

        let matcher = route_match.compile("v2_api");

        let msg = RequestBuilder::new()
            .path("/")
            .header("x-api-version", "v2")
            .build();
        assert_eq!(matcher.evaluate(&msg), Some("v2_api"));

        let msg = RequestBuilder::new()
            .path("/")
            .header("x-api-version", "v1")
            .build();
        assert_eq!(matcher.evaluate(&msg), None);

        let msg = RequestBuilder::new().path("/").build();
        assert_eq!(matcher.evaluate(&msg), None);
    }

    #[test]
    fn e2e_header_regex_matches() {
        let route_match = HttpRouteMatch {
            headers: Some(vec![HttpHeaderMatch::RegularExpression {
                name: "authorization".into(),
                value: r"^Bearer .+$".into(),
            }]),
            ..Default::default()
        };

        let matcher = route_match.compile("authenticated");

        let msg = RequestBuilder::new()
            .path("/")
            .header("authorization", "Bearer token123")
            .build();
        assert_eq!(matcher.evaluate(&msg), Some("authenticated"));

        let msg = RequestBuilder::new()
            .path("/")
            .header("authorization", "Basic base64creds")
            .build();
        assert_eq!(matcher.evaluate(&msg), None);
    }

    // ========== End-to-End Query Param Matching ==========

    #[test]
    fn e2e_query_param_exact_matches() {
        let route_match = HttpRouteMatch {
            query_params: Some(vec![HttpQueryParamMatch::Exact {
                name: "format".into(),
                value: "json".into(),
            }]),
            ..Default::default()
        };

        let matcher = route_match.compile("json_response");

        let msg = RequestBuilder::new().path("/data?format=json").build();
        assert_eq!(matcher.evaluate(&msg), Some("json_response"));

        let msg = RequestBuilder::new().path("/data?format=xml").build();
        assert_eq!(matcher.evaluate(&msg), None);

        let msg = RequestBuilder::new().path("/data").build();
        assert_eq!(matcher.evaluate(&msg), None);
    }

    // ========== End-to-End Combined Conditions (AND) ==========

    #[test]
    fn e2e_combined_path_and_method() {
        let route_match = HttpRouteMatch {
            path: Some(HttpPathMatch::PathPrefix {
                value: "/api".into(),
            }),
            method: Some("POST".into()),
            ..Default::default()
        };

        let matcher = route_match.compile("api_write");

        let msg = RequestBuilder::new()
            .method("POST")
            .path("/api/users")
            .build();
        assert_eq!(matcher.evaluate(&msg), Some("api_write"));

        let msg = RequestBuilder::new()
            .method("GET")
            .path("/api/users")
            .build();
        assert_eq!(matcher.evaluate(&msg), None);

        let msg = RequestBuilder::new().method("POST").path("/other").build();
        assert_eq!(matcher.evaluate(&msg), None);
    }

    #[test]
    fn e2e_combined_all_conditions() {
        let route_match = HttpRouteMatch {
            path: Some(HttpPathMatch::PathPrefix {
                value: "/api/v2".into(),
            }),
            method: Some("PUT".into()),
            headers: Some(vec![HttpHeaderMatch::Exact {
                name: "content-type".into(),
                value: "application/json".into(),
            }]),
            query_params: Some(vec![HttpQueryParamMatch::Exact {
                name: "dry-run".into(),
                value: "true".into(),
            }]),
        };

        let matcher = route_match.compile("v2_api_dry_run");

        let msg = RequestBuilder::new()
            .method("PUT")
            .path("/api/v2/resource?dry-run=true")
            .header("content-type", "application/json")
            .build();
        assert_eq!(matcher.evaluate(&msg), Some("v2_api_dry_run"));

        let msg = RequestBuilder::new()
            .method("PUT")
            .path("/api/v2/resource")
            .header("content-type", "application/json")
            .build();
        assert_eq!(matcher.evaluate(&msg), None);

        let msg = RequestBuilder::new()
            .method("PUT")
            .path("/api/v2/resource?dry-run=true")
            .header("content-type", "text/plain")
            .build();
        assert_eq!(matcher.evaluate(&msg), None);
    }

    // ========== End-to-End Multiple Routes (OR) ==========

    #[test]
    fn e2e_multiple_routes_or() {
        let matches = vec![
            HttpRouteMatch {
                path: Some(HttpPathMatch::Exact {
                    value: "/health".into(),
                }),
                ..Default::default()
            },
            HttpRouteMatch {
                path: Some(HttpPathMatch::Exact {
                    value: "/ready".into(),
                }),
                ..Default::default()
            },
        ];

        let matcher = compile_route_matches(&matches, "health_check", None);

        let msg = RequestBuilder::new().path("/health").build();
        assert_eq!(matcher.evaluate(&msg), Some("health_check"));

        let msg = RequestBuilder::new().path("/ready").build();
        assert_eq!(matcher.evaluate(&msg), Some("health_check"));

        let msg = RequestBuilder::new().path("/other").build();
        assert_eq!(matcher.evaluate(&msg), None);
    }

    #[test]
    fn e2e_multiple_routes_with_fallback() {
        let matches = vec![HttpRouteMatch {
            path: Some(HttpPathMatch::PathPrefix {
                value: "/api".into(),
            }),
            ..Default::default()
        }];

        let matcher = compile_route_matches(&matches, "api_backend", Some("default_backend"));

        let msg = RequestBuilder::new().path("/api/users").build();
        assert_eq!(matcher.evaluate(&msg), Some("api_backend"));

        let msg = RequestBuilder::new().path("/other").build();
        assert_eq!(matcher.evaluate(&msg), Some("default_backend"));
    }

    #[test]
    fn e2e_empty_matches_matches_everything() {
        let matcher = compile_route_matches::<&str>(&[], "catch_all", None);

        let msg = RequestBuilder::new().path("/anything").build();
        assert_eq!(matcher.evaluate(&msg), Some("catch_all"));

        let msg = RequestBuilder::new().path("/").build();
        assert_eq!(matcher.evaluate(&msg), Some("catch_all"));
    }

    // ========== Edge Cases ==========

    #[test]
    fn e2e_missing_path_in_request() {
        let route_match = HttpRouteMatch {
            path: Some(HttpPathMatch::PathPrefix {
                value: "/api".into(),
            }),
            ..Default::default()
        };

        let matcher = route_match.compile("api_backend");

        let msg = RequestBuilder::new().method("GET").build();
        assert_eq!(matcher.evaluate(&msg), None);
    }

    #[test]
    fn e2e_empty_request() {
        let route_match = HttpRouteMatch {
            path: Some(HttpPathMatch::Exact {
                value: "/test".into(),
            }),
            ..Default::default()
        };

        let matcher = route_match.compile("test");

        let msg = HttpMessage::from(&ProcessingRequest::default());
        assert_eq!(matcher.evaluate(&msg), None);
    }
}
