//! Config conformance fixture runner.
//!
//! Loads YAML fixtures that use the **registry config format** (the same JSON/YAML
//! shape as `MatcherConfig<String>`). This tests the config-driven loading path:
//! YAML → `MatcherConfig<String>` → `Registry::load_matcher()` → evaluate.
//!
//! Unlike the existing `fixture` module (which uses a custom YAML format and manually
//! builds matchers), this module exercises the production config loading pipeline.

use serde::Deserialize;
use std::collections::HashMap;

/// A config conformance test fixture.
///
/// The `config` field is the raw YAML/JSON value that gets deserialized as
/// `MatcherConfig<String>` — the exact same format all implementations must support.
#[derive(Debug, Deserialize)]
pub struct ConfigFixture {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub config: serde_json::Value,
    #[serde(default)]
    pub cases: Vec<ConfigTestCase>,
    #[serde(default)]
    pub expect_error: bool,
}

/// A test case within a config fixture.
#[derive(Debug, Deserialize)]
pub struct ConfigTestCase {
    pub name: String,
    pub context: HashMap<String, String>,
    pub expect: Option<String>,
}

impl ConfigFixture {
    /// Parse a single config fixture from YAML.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Parse multiple config fixtures from a YAML file with `---` separators.
    pub fn from_yaml_multi(yaml: &str) -> Result<Vec<Self>, serde_yaml::Error> {
        let mut fixtures = Vec::new();
        for doc in serde_yaml::Deserializer::from_str(yaml) {
            fixtures.push(Self::deserialize(doc)?);
        }
        Ok(fixtures)
    }
}

impl ConfigTestCase {
    /// Build a `TestContext` from this case's context map.
    pub fn build_context(&self) -> crate::TestContext {
        let mut ctx = crate::TestContext::new();
        for (k, v) in &self.context {
            ctx = ctx.with(k.clone(), v.clone());
        }
        ctx
    }
}
