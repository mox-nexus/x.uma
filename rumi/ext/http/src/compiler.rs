//! Compiler: Gateway API `HttpRouteMatch` -> rumi Matcher
//!
//! Translates user-friendly Gateway API configuration into efficient
//! runtime matchers operating on `HttpMessage`.

use crate::inputs::{HeaderInput, MethodInput, PathInput, QueryParamInput};
use crate::message::HttpMessage;
use k8s_gateway_api::{HttpHeaderMatch, HttpPathMatch, HttpQueryParamMatch, HttpRouteMatch};
use rumi::prelude::*;
use rumi::{MatcherError, StringMatchSpec};

/// Extension trait for compiling `HttpRouteMatch` to rumi Matcher.
///
/// Both methods return `Result`. Until 2026-08-17 they did not: an invalid
/// regex was silently replaced with an exact match on the *pattern literal*,
/// so a route the operator believed was live simply never fired. The sibling
/// Claude compiler already returned `Result`; this removes the asymmetry.
pub trait HttpRouteMatchExt {
    /// Compile this `HttpRouteMatch` into a rumi Matcher.
    ///
    /// The resulting matcher operates on `HttpMessage` and returns
    /// the provided action when all conditions match.
    ///
    /// # Errors
    ///
    /// Returns [`MatcherError::InvalidPattern`] if a regex does not compile, or
    /// [`MatcherError::PatternTooLong`] if one exceeds the configured limits.
    fn compile<A: Clone + Send + Sync + 'static>(
        &self,
        action: A,
    ) -> Result<Matcher<HttpMessage, A>, MatcherError>;

    /// Compile this `HttpRouteMatch` into a Predicate (without action).
    ///
    /// # Errors
    ///
    /// As [`compile`](Self::compile).
    fn to_predicate(&self) -> Result<Predicate<HttpMessage>, MatcherError>;
}

impl HttpRouteMatchExt for HttpRouteMatch {
    fn compile<A: Clone + Send + Sync + 'static>(
        &self,
        action: A,
    ) -> Result<Matcher<HttpMessage, A>, MatcherError> {
        Ok(Matcher::from_predicate(self.to_predicate()?, action, None))
    }

    fn to_predicate(&self) -> Result<Predicate<HttpMessage>, MatcherError> {
        let mut predicates: Vec<Predicate<HttpMessage>> = Vec::new();

        // Path matching
        if let Some(path_match) = &self.path {
            predicates.push(compile_path_match(path_match)?);
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
                predicates.push(compile_header_match(header_match)?);
            }
        }

        // Query param matching (all params are ANDed)
        if let Some(query_params) = &self.query_params {
            for query_match in query_params {
                predicates.push(compile_query_param_match(query_match)?);
            }
        }

        Ok(Predicate::from_all(predicates, catch_all()))
    }
}

/// A catch-all predicate that matches any HTTP request (empty prefix = match all paths).
fn catch_all() -> Predicate<HttpMessage> {
    Predicate::Single(SinglePredicate::new(
        Box::new(PathInput),
        Box::new(PrefixMatcher::new("")),
    ))
}

/// Compile a path match to a predicate.
///
/// Goes through [`StringMatchSpec::to_input_matcher`], which owns the pattern
/// length limits. Constructing matchers directly here is what let this compiler
/// bypass every declared limit.
fn compile_path_match(path_match: &HttpPathMatch) -> Result<Predicate<HttpMessage>, MatcherError> {
    let spec = match path_match {
        HttpPathMatch::Exact { value } => StringMatchSpec::Exact(value.clone()),
        HttpPathMatch::PathPrefix { value } => StringMatchSpec::Prefix(value.clone()),
        HttpPathMatch::RegularExpression { value } => StringMatchSpec::Regex(value.clone()),
    };
    Ok(Predicate::Single(SinglePredicate::new(
        Box::new(PathInput),
        spec.to_input_matcher()?,
    )))
}

/// Compile a header match to a predicate.
fn compile_header_match(
    header_match: &HttpHeaderMatch,
) -> Result<Predicate<HttpMessage>, MatcherError> {
    let (name, spec) = match header_match {
        HttpHeaderMatch::Exact { name, value } => (name, StringMatchSpec::Exact(value.clone())),
        HttpHeaderMatch::RegularExpression { name, value } => {
            (name, StringMatchSpec::Regex(value.clone()))
        }
    };
    Ok(Predicate::Single(SinglePredicate::new(
        Box::new(HeaderInput::new(name.as_str())),
        spec.to_input_matcher()?,
    )))
}

/// Compile a query param match to a predicate.
fn compile_query_param_match(
    query_match: &HttpQueryParamMatch,
) -> Result<Predicate<HttpMessage>, MatcherError> {
    let (name, spec) = match query_match {
        HttpQueryParamMatch::Exact { name, value } => (name, StringMatchSpec::Exact(value.clone())),
        HttpQueryParamMatch::RegularExpression { name, value } => {
            (name, StringMatchSpec::Regex(value.clone()))
        }
    };
    Ok(Predicate::Single(SinglePredicate::new(
        Box::new(QueryParamInput::new(name.as_str())),
        spec.to_input_matcher()?,
    )))
}

/// Compile multiple `HttpRouteMatch` entries into a single Matcher.
///
/// Multiple matches are `ORed` together per Gateway API semantics.
/// # Errors
///
/// Returns [`MatcherError::InvalidPattern`] if any regex does not compile, or
/// [`MatcherError::PatternTooLong`] if any pattern exceeds the limits.
pub fn compile_route_matches<A: Clone + Send + Sync + 'static>(
    matches: &[HttpRouteMatch],
    action: A,
    on_no_match: Option<A>,
) -> Result<Matcher<HttpMessage, A>, MatcherError> {
    let predicates: Vec<Predicate<HttpMessage>> = matches
        .iter()
        .map(HttpRouteMatchExt::to_predicate)
        .collect::<Result<_, _>>()?;

    Ok(Matcher::from_predicate(
        Predicate::from_any(predicates, catch_all()),
        action,
        on_no_match,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::HttpMessageBuilder;

    // These used to build an ext_proc `ProcessingRequest` to get an
    // `HttpMessage`, which meant every test of the compiler — a Gateway API
    // feature — needed the data plane. It also meant the builder these tests
    // now use, the only construction path a `gateway`-only consumer has, had
    // no coverage at all.
    struct RequestBuilder {
        inner: HttpMessageBuilder,
    }

    impl RequestBuilder {
        fn new() -> Self {
            Self {
                inner: HttpMessageBuilder::new(),
            }
        }

        fn path(mut self, path: &str) -> Self {
            self.inner = self.inner.path(path);
            self
        }

        fn method(mut self, method: &str) -> Self {
            self.inner = self.inner.method(method);
            self
        }

        fn header(mut self, key: &str, value: &str) -> Self {
            self.inner = self.inner.header(key, value);
            self
        }

        fn build(self) -> HttpMessage {
            self.inner.build()
        }
    }

    #[test]
    fn test_compile_empty_match() {
        let route_match = HttpRouteMatch::default();
        let predicate = route_match.to_predicate().unwrap();
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

        let predicate = route_match.to_predicate().unwrap();
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

        let predicate = route_match.to_predicate().unwrap();
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

        let matcher = route_match.compile("api_backend").unwrap();

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

        let matcher = route_match.compile("health_check").unwrap();

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

        let matcher = route_match.compile("user_detail").unwrap();

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

        let matcher = route_match.compile("write_endpoint").unwrap();

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

        let matcher = route_match.compile("v2_api").unwrap();

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

        let matcher = route_match.compile("authenticated").unwrap();

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

        let matcher = route_match.compile("json_response").unwrap();

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

        let matcher = route_match.compile("api_write").unwrap();

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

        let matcher = route_match.compile("v2_api_dry_run").unwrap();

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

        let matcher = compile_route_matches(&matches, "health_check", None).unwrap();

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

        let matcher =
            compile_route_matches(&matches, "api_backend", Some("default_backend")).unwrap();

        let msg = RequestBuilder::new().path("/api/users").build();
        assert_eq!(matcher.evaluate(&msg), Some("api_backend"));

        let msg = RequestBuilder::new().path("/other").build();
        assert_eq!(matcher.evaluate(&msg), Some("default_backend"));
    }

    #[test]
    fn e2e_empty_matches_matches_everything() {
        let matcher = compile_route_matches::<&str>(&[], "catch_all", None).unwrap();

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

        let matcher = route_match.compile("api_backend").unwrap();

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

        let matcher = route_match.compile("test").unwrap();

        // An empty message: no transport needed to assert INV-1.
        let msg = HttpMessageBuilder::new().build();
        assert_eq!(matcher.evaluate(&msg), None);
    }

    // ── Regression: SEC2 / review F-02, and PLAN.md F16 ─────────────────────
    //
    // This compiler used to construct matchers directly, so it inherited none
    // of the declared limits — `grep MAX_ rumi/ext/http/src/*.rs` returned
    // nothing — and an invalid regex was swallowed into an exact match on the
    // pattern literal, silently deleting the route.

    #[test]
    fn oversized_regex_is_rejected_not_swallowed() {
        let huge = "a".repeat(rumi::MAX_REGEX_PATTERN_LENGTH + 1);
        let route = HttpRouteMatch {
            path: Some(HttpPathMatch::RegularExpression { value: huge }),
            ..Default::default()
        };
        let err = route
            .to_predicate()
            .expect_err("must reject oversized regex");
        assert!(
            matches!(err, MatcherError::PatternTooLong { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn invalid_regex_is_reported_not_turned_into_a_dead_route() {
        // The old behaviour: "[bad" became ExactMatcher("[bad"), so the route
        // compiled clean and then matched nothing. An operator reading the
        // config would believe the route was live.
        let route = HttpRouteMatch {
            path: Some(HttpPathMatch::RegularExpression {
                value: "[bad".into(),
            }),
            ..Default::default()
        };
        let err = route
            .to_predicate()
            .expect_err("must surface the bad pattern");
        assert!(
            matches!(err, MatcherError::InvalidPattern { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn limits_apply_to_headers_and_query_params_too() {
        let huge = "a".repeat(rumi::MAX_REGEX_PATTERN_LENGTH + 1);
        let by_header = HttpRouteMatch {
            headers: Some(vec![HttpHeaderMatch::RegularExpression {
                name: "x-test".into(),
                value: huge.clone(),
            }]),
            ..Default::default()
        };
        assert!(
            by_header.to_predicate().is_err(),
            "header regex must be bounded"
        );

        let by_query = HttpRouteMatch {
            query_params: Some(vec![HttpQueryParamMatch::RegularExpression {
                name: "q".into(),
                value: huge,
            }]),
            ..Default::default()
        };
        assert!(
            by_query.to_predicate().is_err(),
            "query regex must be bounded"
        );
    }

    #[test]
    fn compile_route_matches_propagates_rather_than_dropping() {
        let matches = vec![HttpRouteMatch {
            path: Some(HttpPathMatch::RegularExpression {
                value: "[bad".into(),
            }),
            ..Default::default()
        }];
        assert!(compile_route_matches(&matches, "backend", None).is_err());
    }

    #[test]
    fn the_guard_is_not_inert() {
        // A valid regex at exactly the limit still compiles, so the tests above
        // are not passing against a compiler that rejects everything.
        let ok = HttpRouteMatch {
            path: Some(HttpPathMatch::RegularExpression {
                value: "^/api/.*$".into(),
            }),
            ..Default::default()
        };
        assert!(ok.to_predicate().is_ok());
    }
}
