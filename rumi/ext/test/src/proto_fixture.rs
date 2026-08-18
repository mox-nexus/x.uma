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

    /// The config must fail to load. `cases` is then ignored.
    #[serde(default)]
    pub expect_error: bool,
}

fn all_implementations() -> Vec<Implementation> {
    ALL.to_vec()
}

/// One evaluation against a key-value context.
#[derive(Debug, Deserialize)]
pub struct ProtoTestCase {
    /// Case name, used in failure messages.
    pub name: String,
    /// The context to evaluate against.
    #[serde(default)]
    pub context: HashMap<String, String>,
    /// The expected action, or `None` for no match.
    pub expect: Option<String>,
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
