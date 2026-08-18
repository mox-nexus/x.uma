//! Conformance fixtures written in canonical protojson.
//!
//! This is the **fifth** top-level fixture key, and adding one rather than
//! changing the four that exist is deliberate. Rewriting a fixture in place
//! changes it for all three implementations at the same instant, which forces a
//! simultaneous three-language migration — the opposite of the "Rust first,
//! then Python, then TypeScript" order the release plan mandates. A new key
//! lets one implementation move at a time.
//!
//! # The invariant this protects
//!
//! The conformance suite's value is not that it is green. It is that a
//! disagreement between two implementations means something. So the property to
//! hold at every commit is:
//!
//! > for every fixture and every pair of implementations, the two produce the
//! > **same verdict**
//!
//! not "everything passes". A migration that preserves that never emits a false
//! divergence signal even while failing; one that breaks it emits nothing else,
//! because red stops carrying information.
//!
//! Rust necessarily runs ahead here, and that is the one transient the ordering
//! cannot design away. It is recorded rather than tolerated: each fixture names
//! the implementations expected to run it, and a runner that is **not** listed
//! must fail to run it. An exception that quietly starts working is as much a
//! defect in the ledger as one that quietly starts failing — a stale skip is
//! how a suite ends up reporting on work nobody has done.
//!
//! The migration is finished when every fixture lists all three.

use serde::Deserialize;
use std::collections::HashMap;

/// Which domain a fixture's contexts belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Domain {
    /// A string-to-string map. The default.
    #[default]
    Kv,
    /// An HTTP request.
    Http,
}

/// An implementation that may be expected to run a fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Implementation {
    /// rumi, the Rust reference implementation.
    Rust,
    /// puma, the pure Python implementation.
    Python,
    /// bumi, the pure TypeScript implementation.
    Typescript,
}

/// Every implementation the suite covers. The migration's finish line.
pub const ALL: [Implementation; 3] = [
    Implementation::Rust,
    Implementation::Python,
    Implementation::Typescript,
];

/// A fixture whose matcher is written in canonical protojson.
#[derive(Debug, Deserialize)]
pub struct ProtoFixture {
    /// Fixture name, used in failure messages.
    pub name: String,
    /// What this fixture is for.
    #[serde(default)]
    pub description: String,

    /// The matcher, as canonical protojson for `xds.type.matcher.v3.Matcher`.
    pub proto_matcher: serde_json::Value,

    /// Which implementations are expected to run this fixture.
    ///
    /// Absent means all three, which is the end state. A shorter list is an
    /// **expiring exception**: it says the others have not been migrated yet,
    /// and CI holds it to that in both directions.
    #[serde(default = "all_implementations")]
    pub implementations: Vec<Implementation>,

    /// Cases to evaluate. Empty when `expect_error` is set.
    #[serde(default)]
    pub cases: Vec<ProtoTestCase>,

    /// Which domain's context this fixture evaluates against.
    ///
    /// `kv` (the default) reads a string map; `http` builds an `HttpRequest`.
    /// The matcher config itself is domain-agnostic — only the context and the
    /// registry differ.
    #[serde(default)]
    pub domain: Domain,

    /// The config must fail to load. `cases` is then ignored.
    #[serde(default)]
    pub expect_error: bool,

    /// A substring the load error must contain.
    ///
    /// Without this an `expect_error` fixture passes on *any* failure, so one
    /// that starts failing earlier — a type URL nobody registered, a typo in
    /// the fixture itself — still looks green while no longer testing what it
    /// was written for. That happened here: a fixture meant to prove a
    /// both-set oneof is rejected was instead failing on an unregistered type,
    /// and passing.
    #[serde(default)]
    pub error_contains: Option<String>,
}

fn all_implementations() -> Vec<Implementation> {
    ALL.to_vec()
}

/// One evaluation against a key-value context.
#[derive(Debug, Deserialize)]
pub struct ProtoTestCase {
    /// Case name, used in failure messages.
    pub name: String,
    /// A key-value context, for fixtures in the `kv` domain.
    #[serde(default)]
    pub context: HashMap<String, String>,
    /// An HTTP request, for fixtures in the `http` domain.
    #[serde(default)]
    pub http_request: Option<HttpRequestSpec>,
    /// The expected action, or `None` for no match.
    pub expect: Option<String>,
}

/// One HTTP request a fixture evaluates against.
///
/// Shaped like `spec/tests/05_http`'s `http_request:` so the two dialects
/// describe a request the same way — they exercise different code paths (the
/// compiler there, the config path here) and there is no reason for the input
/// to look different.
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

impl ProtoFixture {
    /// Parse one fixture from YAML.
    ///
    /// # Errors
    ///
    /// If the document is not a `ProtoFixture`.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Parse every `---`-separated fixture in a YAML file.
    ///
    /// # Errors
    ///
    /// If any document is not a `ProtoFixture`.
    pub fn from_yaml_multi(yaml: &str) -> Result<Vec<Self>, serde_yaml::Error> {
        serde_yaml::Deserializer::from_str(yaml)
            .map(Self::deserialize)
            .collect()
    }

    /// Is this implementation expected to run this fixture?
    #[must_use]
    pub fn expects(&self, who: Implementation) -> bool {
        self.implementations.contains(&who)
    }
}
