//! Compiler: Gateway API `HttpRouteMatch` → rumi Matcher
//!
//! Translates user-friendly Gateway API configuration into efficient
//! runtime matchers operating on `ext_proc` `ProcessingRequest`.

use crate::inputs::{HeaderInput, MethodInput, PathInput, QueryParamInput};
use envoy_grpc_ext_proc::envoy::service::ext_proc::v3::ProcessingRequest;
use k8s_gateway_api::{HttpHeaderMatch, HttpPathMatch, HttpQueryParamMatch, HttpRouteMatch};
use rumi::prelude::*;

/// Extension trait for compiling `HttpRouteMatch` to rumi Matcher.
pub trait HttpRouteMatchExt {
    /// Compile this `HttpRouteMatch` into a rumi Matcher.
    ///
    /// The resulting matcher operates on `ProcessingRequest` and returns
    /// the provided action when all conditions match.
    fn compile<A: Clone + Send + Sync + 'static>(&self, action: A)
        -> Matcher<ProcessingRequest, A>;

    /// Compile this `HttpRouteMatch` into a Predicate (without action).
    fn to_predicate(&self) -> Predicate<ProcessingRequest>;
}

impl HttpRouteMatchExt for HttpRouteMatch {
    fn compile<A: Clone + Send + Sync + 'static>(
        &self,
        action: A,
    ) -> Matcher<ProcessingRequest, A> {
        let predicate = self.to_predicate();

        Matcher::new(
            vec![FieldMatcher::new(predicate, OnMatch::Action(action))],
            None,
        )
    }

    fn to_predicate(&self) -> Predicate<ProcessingRequest> {
        let mut predicates: Vec<Predicate<ProcessingRequest>> = Vec::new();

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
fn compile_path_match(path_match: &HttpPathMatch) -> Predicate<ProcessingRequest> {
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
fn compile_header_match(header_match: &HttpHeaderMatch) -> Predicate<ProcessingRequest> {
    // HttpHeaderMatch is an enum with variants Exact and RegularExpression
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
fn compile_query_param_match(query_match: &HttpQueryParamMatch) -> Predicate<ProcessingRequest> {
    // HttpQueryParamMatch is an enum with variants Exact and RegularExpression
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
) -> Matcher<ProcessingRequest, A> {
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
    let predicates: Vec<Predicate<ProcessingRequest>> = matches
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

    #[test]
    fn test_compile_empty_match() {
        let route_match = HttpRouteMatch::default();
        let predicate = route_match.to_predicate();

        // Empty match should produce a match-all predicate
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

        // Multiple conditions should produce AND
        assert!(matches!(predicate, Predicate::And(_)));
    }
}
