//! Reader for `spec/tests/05_http/` — Gateway API route matches through the
//! domain compiler.
//!
//! rumi reads this suite because it is the reference implementation: what a
//! fixture requires is only checked where a runner reads it (D-050).
//!
//! The fixture dialect is `snake_case` throughout (`http_route_match`,
//! `query_params`, `on_no_match`), while `k8s_gateway_api::HttpRouteMatch` is
//! `rename_all = "camelCase"`. The nested types need no help — `HttpPathMatch`
//! and friends are `tag = "type"` with `PascalCase` variants, exactly the shape
//! the fixtures already use — so only the outer struct is restated here.

use rumi_http::{HttpHeaderMatch, HttpPathMatch, HttpQueryParamMatch, HttpRouteMatch};
use serde::Deserialize;
use std::collections::HashMap;

use crate::implementations::Implementation;

/// One `---`-separated document in a `05_http` fixture file.
#[derive(Debug, Deserialize)]
pub struct HttpFixture {
    /// Fixture name, used in test output.
    pub name: String,
    /// What the fixture is for. Unused by the runner; read by people.
    #[serde(default)]
    #[allow(dead_code)]
    pub description: Option<String>,

    /// A single route match. Mutually exclusive with `http_route_matches`.
    #[serde(default)]
    pub http_route_match: Option<RouteMatchSpec>,
    /// Several route matches, `ORed`. Mutually exclusive with the above.
    #[serde(default)]
    pub http_route_matches: Option<Vec<RouteMatchSpec>>,

    /// Action returned on a match.
    pub action: String,
    /// Action returned when nothing matches.
    #[serde(default)]
    pub on_no_match: Option<String>,

    /// Which implementations are expected to run this fixture. Required.
    ///
    /// A shorter list than [`crate::implementations::ALL`] is an expiring
    /// exception, held in both directions by the runners.
    pub implementations: Vec<Implementation>,

    /// The compile must fail. `cases` is then ignored.
    #[serde(default)]
    pub expect_error: bool,
    /// A substring the compile error must contain.
    ///
    /// Required whenever `expect_error` is set: without it the fixture passes
    /// on any failure, so one that starts failing earlier still looks green
    /// while no longer testing what it was written for.
    #[serde(default)]
    pub error_contains: Option<String>,

    /// Requests to evaluate.
    #[serde(default)]
    pub cases: Vec<HttpCase>,
}

/// `HttpRouteMatch` in the fixture dialect's `snake_case`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteMatchSpec {
    /// Path condition.
    #[serde(default)]
    pub path: Option<HttpPathMatch>,
    /// Method condition.
    #[serde(default)]
    pub method: Option<String>,
    /// Header conditions, `ANDed`.
    #[serde(default)]
    pub headers: Option<Vec<HttpHeaderMatch>>,
    /// Query parameter conditions, `ANDed`.
    #[serde(default)]
    pub query_params: Option<Vec<HttpQueryParamMatch>>,
}

impl From<&RouteMatchSpec> for HttpRouteMatch {
    fn from(s: &RouteMatchSpec) -> Self {
        Self {
            path: s.path.clone(),
            method: s.method.clone(),
            headers: s.headers.clone(),
            query_params: s.query_params.clone(),
        }
    }
}

/// One request and the action it must produce.
#[derive(Debug, Deserialize)]
pub struct HttpCase {
    /// Case name, used in test output.
    pub name: String,
    /// The request.
    pub http_request: HttpRequestSpec,
    /// Expected action, or `None` for no match.
    #[serde(default)]
    pub expect: Option<String>,
}

/// A request, shaped like `07_protojson`'s so the two dialects agree.
#[derive(Debug, Deserialize)]
pub struct HttpRequestSpec {
    /// Request method.
    #[serde(default)]
    pub method: String,
    /// Request target, query string included.
    #[serde(default)]
    pub path: String,
    /// Request headers.
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

impl HttpFixture {
    /// Parse every `---`-separated fixture in a YAML file.
    ///
    /// # Errors
    ///
    /// If any document is not an `HttpFixture`.
    pub fn all_from_yaml(yaml: &str) -> Result<Vec<Self>, serde_yaml::Error> {
        let mut out = Vec::new();
        for doc in serde_yaml::Deserializer::from_str(yaml) {
            out.push(Self::deserialize(doc)?);
        }
        Ok(out)
    }

    /// The route matches this fixture compiles, in Gateway API types.
    #[must_use]
    pub fn route_matches(&self) -> Vec<HttpRouteMatch> {
        match (&self.http_route_match, &self.http_route_matches) {
            (Some(one), None) => vec![one.into()],
            (None, Some(many)) => many.iter().map(Into::into).collect(),
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HttpFixture;

    #[test]
    fn a_fixture_without_a_ledger_is_rejected() {
        let keyless = "name: keyless\naction: hit\ncases: []\n";
        let err = HttpFixture::all_from_yaml(keyless).expect_err("the ledger key is required");
        assert!(
            err.to_string().contains("implementations"),
            "the error should name the missing key, got: {err}"
        );
    }
}
